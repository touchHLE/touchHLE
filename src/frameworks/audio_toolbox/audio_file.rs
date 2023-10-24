/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AudioFile.h` (Audio File Services)

use crate::audio; // Keep this module namespaced to avoid confusion
use crate::audio::{decode_ima4, AudioFormat};
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::carbon_core::{eofErr, OSStatus};
use crate::frameworks::core_audio_types::{
    debug_fourcc, fourcc, kAudioFormatAppleIMA4, kAudioFormatFlagIsBigEndian,
    kAudioFormatFlagIsFloat, kAudioFormatFlagIsPacked, kAudioFormatFlagIsSignedInteger,
    kAudioFormatLinearPCM, AudioBuffer, AudioStreamBasicDescription,
};
use crate::frameworks::core_foundation::cf_url::CFURLRef;
use crate::frameworks::foundation::ns_url::to_rust_path;
use crate::mem::{guest_size_of, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, SafeRead};
use crate::Environment;
use std::collections::HashMap;
use std::slice;

#[derive(Default)]
pub struct State {
    audio_files: HashMap<AudioFileID, AudioFileHostObject>,
}
impl State {
    pub fn get(framework_state: &mut crate::frameworks::State) -> &mut Self {
        &mut framework_state.audio_toolbox.audio_file
    }
}

struct AudioFileHostObject {
    audio_file: audio::AudioFile,
    position: u64,
}

#[repr(C, packed)]
struct OpaqueAudioFileID {
    _filler: u8,
}
unsafe impl SafeRead for OpaqueAudioFileID {}

type AudioFileID = MutPtr<OpaqueAudioFileID>;

const kAudioFileFileNotFoundError: OSStatus = -43;
const kAudioFileBadPropertySizeError: OSStatus = fourcc(b"!siz") as _;
const kAudioFileUnsupportedProperty: OSStatus = fourcc(b"pty?") as _;

type AudioFilePermissions = i8;
const kAudioFileReadPermission: AudioFilePermissions = 1;

/// Usually a FourCC.
type AudioFileTypeID = u32;

/// Usually a FourCC.
type AudioFilePropertyID = u32;
const kAudioFilePropertyDataFormat: AudioFilePropertyID = fourcc(b"dfmt");
const kAudioFilePropertyAudioDataByteCount: AudioFilePropertyID = fourcc(b"bcnt");
const kAudioFilePropertyAudioDataPacketCount: AudioFilePropertyID = fourcc(b"pcnt");
const kAudioFilePropertyPacketSizeUpperBound: AudioFilePropertyID = fourcc(b"pkub");
const kAudioFilePropertyMagicCookieData: AudioFilePropertyID = fourcc(b"mgic");
const kAudioFilePropertyChannelLayout: AudioFilePropertyID = fourcc(b"cmap");
const kExtAudioFileProperty_FileDataFormat: AudioFilePropertyID = fourcc(b"ffmt");
const kExtAudioFileProperty_ClientDataFormat: AudioFilePropertyID = fourcc(b"cfmt");
const kExtAudioFileProperty_FileLengthFrames: AudioFilePropertyID = fourcc(b"#frm");

fn AudioFileOpenURL(
    env: &mut Environment,
    in_file_ref: CFURLRef,
    in_permissions: AudioFilePermissions,
    in_file_type_hint: AudioFileTypeID,
    out_audio_file: MutPtr<AudioFileID>,
) -> OSStatus {
    return_if_null!(in_file_ref);

    assert!(in_permissions == kAudioFileReadPermission); // writing TODO

    // The hint is optional and is supposed to only be used for certain file
    // formats that can't be uniquely identified, which we don't support so far.
    assert!(in_file_type_hint == 0);
    audio_file_open_inner(env, in_file_ref, out_audio_file)
}

fn ExtAudioFileOpenURL(
    env: &mut Environment,
    in_file_ref: CFURLRef,
    out_audio_file: MutPtr<AudioFileID>,
) -> OSStatus {
    audio_file_open_inner(env, in_file_ref, out_audio_file)
}

fn audio_file_open_inner(
    env: &mut Environment,
    in_file_ref: CFURLRef,
    out_audio_file: MutPtr<AudioFileID>,
) -> OSStatus {
    let path = to_rust_path(env, in_file_ref);
    let Ok(audio_file) = audio::AudioFile::open_for_reading(path, &env.fs) else {
        log!(
            "Warning: AudioFileOpenURL() for path {:?} failed",
            in_file_ref
        );
        return kAudioFileFileNotFoundError;
    };

    let host_object = AudioFileHostObject {
        audio_file,
        position: 0,
    };

    let guest_audio_file = env.mem.alloc_and_write(OpaqueAudioFileID { _filler: 0 });
    State::get(&mut env.framework_state)
        .audio_files
        .insert(guest_audio_file, host_object);

    env.mem.write(out_audio_file, guest_audio_file);

    log_dbg!(
        "AudioFileOpenURL() opened path {:?}, new audio file handle: {:?}",
        in_file_ref,
        guest_audio_file
    );

    0 // success
}

fn property_size(property_id: AudioFilePropertyID) -> GuestUSize {
    match property_id {
        kAudioFilePropertyDataFormat
        | kExtAudioFileProperty_FileDataFormat
        | kExtAudioFileProperty_ClientDataFormat => guest_size_of::<AudioStreamBasicDescription>(),
        kAudioFilePropertyAudioDataByteCount => guest_size_of::<u64>(),
        kAudioFilePropertyAudioDataPacketCount => guest_size_of::<u64>(),
        kAudioFilePropertyPacketSizeUpperBound => guest_size_of::<u32>(),
        kExtAudioFileProperty_FileLengthFrames => guest_size_of::<i64>(),
        _ => unimplemented!("Unimplemented property ID: {}", debug_fourcc(property_id)),
    }
}

fn AudioFileGetPropertyInfo(
    env: &mut Environment,
    in_audio_file: AudioFileID,
    in_property_id: AudioFilePropertyID,
    out_data_size: MutPtr<u32>,
    is_writable: MutPtr<u32>,
) -> OSStatus {
    return_if_null!(in_audio_file);

    if in_property_id == kAudioFilePropertyMagicCookieData
        || in_property_id == kAudioFilePropertyChannelLayout
    {
        // Our currently supported formats probably don't use these properties.
        // Not sure if this is correct, but it skips some code we don't want to
        // run in Touch & Go.
        if !out_data_size.is_null() {
            env.mem.write(out_data_size, 0);
        }
        if !is_writable.is_null() {
            env.mem.write(is_writable, 0);
        }
        return kAudioFileUnsupportedProperty;
    }
    if !out_data_size.is_null() {
        env.mem.write(out_data_size, property_size(in_property_id));
    }
    if !is_writable.is_null() {
        env.mem.write(is_writable, 0); // TODO: probably not always correct
    }
    0 // success
}

fn ExtAudioFileGetProperty(
    env: &mut Environment,
    in_audio_file: AudioFileID,
    in_property_id: AudioFilePropertyID,
    io_data_size: MutPtr<u32>,
    out_property_data: MutVoidPtr,
) -> OSStatus {
    AudioFileGetProperty(
        env,
        in_audio_file,
        in_property_id,
        io_data_size,
        out_property_data,
    )
}

fn AudioFileGetProperty(
    env: &mut Environment,
    in_audio_file: AudioFileID,
    in_property_id: AudioFilePropertyID,
    io_data_size: MutPtr<u32>,
    out_property_data: MutVoidPtr,
) -> OSStatus {
    return_if_null!(in_audio_file);

    let required_size = property_size(in_property_id);
    if env.mem.read(io_data_size) != required_size {
        log!("Warning: AudioFileGetProperty() failed");
        return kAudioFileBadPropertySizeError;
    }

    let host_object = State::get(&mut env.framework_state)
        .audio_files
        .get_mut(&in_audio_file)
        .unwrap();

    match in_property_id {
        kAudioFilePropertyDataFormat | kExtAudioFileProperty_FileDataFormat => {
            let audio::AudioDescription {
                sample_rate,
                format,
                bytes_per_packet,
                frames_per_packet,
                channels_per_frame,
                bits_per_channel,
            } = host_object.audio_file.audio_description();

            let desc: AudioStreamBasicDescription = match format {
                audio::AudioFormat::LinearPcm {
                    is_float,
                    is_little_endian,
                } => {
                    let is_packed = (bits_per_channel * channels_per_frame * frames_per_packet)
                        == (bytes_per_packet * 8);
                    let format_flags = (u32::from(is_float) * kAudioFormatFlagIsFloat)
                        | (u32::from((!is_float) && matches!(bits_per_channel, 16 | 24))
                            * kAudioFormatFlagIsSignedInteger)
                        | (u32::from(is_packed) * kAudioFormatFlagIsPacked)
                        | (u32::from(!is_little_endian) * kAudioFormatFlagIsBigEndian);
                    AudioStreamBasicDescription {
                        sample_rate,
                        format_id: kAudioFormatLinearPCM,
                        format_flags,
                        bytes_per_packet,
                        frames_per_packet,
                        bytes_per_frame: bytes_per_packet / frames_per_packet,
                        channels_per_frame,
                        bits_per_channel,
                        _reserved: 0,
                    }
                }
                audio::AudioFormat::AppleIma4 => {
                    AudioStreamBasicDescription {
                        sample_rate,
                        format_id: kAudioFormatAppleIMA4,
                        format_flags: 0,
                        bytes_per_packet,
                        frames_per_packet,
                        bytes_per_frame: 0, // compressed
                        channels_per_frame,
                        bits_per_channel,
                        _reserved: 0,
                    }
                }
            };
            env.mem.write(out_property_data.cast(), desc);
        }
        kAudioFilePropertyAudioDataByteCount => {
            let byte_count: u64 = host_object.audio_file.byte_count();
            env.mem.write(out_property_data.cast(), byte_count);
        }
        kAudioFilePropertyAudioDataPacketCount => {
            let packet_count: u64 = host_object.audio_file.packet_count();
            env.mem.write(out_property_data.cast(), packet_count);
        }
        kAudioFilePropertyPacketSizeUpperBound => {
            let packet_size_upper_bound: u32 = host_object.audio_file.packet_size_upper_bound();
            env.mem
                .write(out_property_data.cast(), packet_size_upper_bound);
        }
        kExtAudioFileProperty_FileLengthFrames => {
            if host_object.audio_file.audio_description().format != AudioFormat::AppleIma4 {
                unimplemented!();
            }
            // Each packet decodes to 64 samples
            let sample_count = host_object.audio_file.packet_count() as i64 * 64;
            env.mem.write(out_property_data.cast(), sample_count);
        }
        _ => unreachable!(),
    }

    0 // success
}

fn ExtAudioFileSetProperty(
    env: &mut Environment,
    in_audio_file: AudioFileID,
    in_property_id: AudioFilePropertyID,
    in_data_size: u32,
    in_property_data: ConstVoidPtr,
) -> OSStatus {
    let required_size = property_size(in_property_id);
    if in_data_size != required_size {
        log!("Warning: AudioFileGetProperty() failed");
        return kAudioFileBadPropertySizeError;
    }
    match in_property_id {
        kExtAudioFileProperty_ClientDataFormat => {
            let host_object = State::get(&mut env.framework_state)
                .audio_files
                .get_mut(&in_audio_file)
                .unwrap();
            let format = env
                .mem
                .read(in_property_data.cast::<AudioStreamBasicDescription>());
            assert!(format.bits_per_channel == 16);
            assert!(format.bytes_per_frame == 2);
            assert!(format.channels_per_frame == 1);
            assert!(format.bytes_per_packet == 2);
            assert!(format.frames_per_packet == 1);
            assert!(format.format_id == kAudioFormatLinearPCM);
            assert!(format.sample_rate == host_object.audio_file.audio_description().sample_rate);
        }
        _ => unreachable!(),
    }
    0
}

fn AudioFileReadBytes(
    env: &mut Environment,
    in_audio_file: AudioFileID,
    _in_use_cache: bool,
    in_starting_byte: i64,
    io_num_bytes: MutPtr<u32>,
    out_buffer: MutVoidPtr,
) -> OSStatus {
    return_if_null!(in_audio_file);

    let host_object = State::get(&mut env.framework_state)
        .audio_files
        .get_mut(&in_audio_file)
        .unwrap();

    let bytes_to_read = env.mem.read(io_num_bytes);
    let buffer_slice = env.mem.bytes_at_mut(out_buffer.cast(), bytes_to_read);

    let bytes_read = host_object
        .audio_file
        .read_bytes(in_starting_byte.try_into().unwrap(), buffer_slice)
        .unwrap(); // TODO: handle seek error?
    env.mem.write(io_num_bytes, bytes_read.try_into().unwrap());

    if bytes_read < bytes_to_read as usize {
        eofErr
    } else {
        0 // success
    }
}

fn AudioFileReadPackets(
    env: &mut Environment,
    in_audio_file: AudioFileID,
    in_use_cache: bool,
    out_num_bytes: MutPtr<u32>,
    out_packet_descriptions: MutVoidPtr, // unimplemented
    in_starting_packet: i64,
    io_num_packets: MutPtr<u32>,
    out_buffer: MutVoidPtr,
) -> OSStatus {
    return_if_null!(in_audio_file);

    // Variable-size packets are not implemented currently. When they are,
    // this parameter should be a `MutPtr<AudioStreamPacketDescription>`.
    assert!(out_packet_descriptions.is_null());

    let host_object = State::get(&mut env.framework_state)
        .audio_files
        .get_mut(&in_audio_file)
        .unwrap();
    let packet_size = host_object.audio_file.packet_size_fixed();

    let packets_to_read = env.mem.read(io_num_packets);

    let starting_byte = i64::from(packet_size)
        .checked_mul(in_starting_packet)
        .unwrap();
    let bytes_to_read = packets_to_read.checked_mul(packet_size).unwrap();

    env.mem.write(out_num_bytes, bytes_to_read);
    let res = AudioFileReadBytes(
        env,
        in_audio_file,
        in_use_cache,
        starting_byte,
        out_num_bytes,
        out_buffer,
    );

    let bytes_read = env.mem.read(out_num_bytes);
    let packets_read = bytes_read / packet_size;
    env.mem.write(io_num_packets, packets_read);

    res
}

fn ExtAudioFileRead(
    env: &mut Environment,
    in_audio_file: AudioFileID,
    io_num_frames: MutPtr<u32>,
    io_data: MutVoidPtr,
) -> OSStatus {
    let host_object = State::get(&mut env.framework_state)
        .audio_files
        .get_mut(&in_audio_file)
        .unwrap();

    let packet_size = host_object.audio_file.packet_size_fixed();
    let frames_to_read = env.mem.read(io_num_frames);
    let packets_to_read = frames_to_read / 64;
    let mut data = vec![0; (packets_to_read * packet_size) as usize];
    let actually_read = host_object
        .audio_file
        .read_bytes(host_object.position * packet_size as u64, &mut data)
        .unwrap();

    let mut packets_consumed = 0;
    let buf_count_ptr = io_data.cast::<u32>();
    let buf_count = env.mem.read(buf_count_ptr);
    let buf_ptr = (buf_count_ptr + 1).cast::<AudioBuffer>();
    let mut buf_no = 0;
    let mut buf_offset = 0;
    'outer: for packet in data[..actually_read].chunks(packet_size as usize) {
        let pcm = decode_ima4(packet.try_into().unwrap());
        loop {
            let buf = env.mem.read(buf_ptr + buf_no);
            if ((buf_offset + pcm.len() as GuestUSize) * 2) < buf.data_byte_size {
                let target = env
                    .mem
                    .ptr_at_mut(buf.data.cast::<i16>() + buf_offset, pcm.len() as GuestUSize);
                unsafe {
                    slice::from_raw_parts_mut(target, pcm.len()).copy_from_slice(&pcm);
                }
                packets_consumed += 1;
                break;
            }
            buf_no += 1;
            buf_offset = 0;
            if buf_no >= buf_count {
                break 'outer;
            }
        }
    }
    host_object.position += packets_consumed as u64;
    env.mem.write(io_num_frames, packets_consumed * 64);
    0
}

fn ExtAudioFileDispose(env: &mut Environment, in_audio_file: AudioFileID) -> OSStatus {
    AudioFileClose(env, in_audio_file)
}

fn AudioFileClose(env: &mut Environment, in_audio_file: AudioFileID) -> OSStatus {
    return_if_null!(in_audio_file);

    let _host_object = State::get(&mut env.framework_state)
        .audio_files
        .remove(&in_audio_file)
        .unwrap();
    env.mem.free(in_audio_file.cast());
    log_dbg!(
        "AudioFileClose() destroyed audio file handle: {:?}",
        in_audio_file
    );
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioFileOpenURL(_, _, _, _)),
    export_c_func!(AudioFileGetPropertyInfo(_, _, _, _)),
    export_c_func!(AudioFileGetProperty(_, _, _, _)),
    export_c_func!(AudioFileReadBytes(_, _, _, _, _)),
    export_c_func!(AudioFileReadPackets(_, _, _, _, _, _, _)),
    export_c_func!(AudioFileClose(_)),
    export_c_func!(ExtAudioFileOpenURL(_, _)),
    export_c_func!(ExtAudioFileGetProperty(_, _, _, _)),
    export_c_func!(ExtAudioFileSetProperty(_, _, _, _)),
    export_c_func!(ExtAudioFileRead(_, _, _)),
    export_c_func!(ExtAudioFileDispose(_)),
];
