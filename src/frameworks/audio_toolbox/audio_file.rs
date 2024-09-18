/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AudioFile.h` (Audio File Services)

use crate::abi::{CallFromHost, GuestFunction};
use crate::audio; // Keep this module namespaced to avoid confusion
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::carbon_core::{eofErr, OSStatus};
use crate::frameworks::core_audio_types::{
    debug_fourcc, fourcc, kAudioFormatAppleIMA4, kAudioFormatFlagIsBigEndian,
    kAudioFormatFlagIsFloat, kAudioFormatFlagIsPacked, kAudioFormatFlagIsSignedInteger,
    kAudioFormatLinearPCM, AudioStreamBasicDescription,
};
use crate::frameworks::core_foundation::cf_url::CFURLRef;
use crate::frameworks::foundation::ns_url::to_rust_path;
use crate::mem::{guest_size_of, GuestUSize, MutPtr, MutVoidPtr, SafeRead};
use crate::Environment;
use std::collections::HashMap;

#[derive(Default)]
pub struct State {
    pub audio_files: HashMap<AudioFileID, AudioFileHostObject>,
}
impl State {
    pub fn get(framework_state: &mut crate::frameworks::State) -> &mut Self {
        &mut framework_state.audio_toolbox.audio_file
    }
}

pub struct AudioFileHostObject {
    pub audio_file: audio::AudioFile,
}

#[repr(C, packed)]
pub struct OpaqueAudioFileID {
    _filler: u8,
}
unsafe impl SafeRead for OpaqueAudioFileID {}

pub type AudioFileID = MutPtr<OpaqueAudioFileID>;

#[allow(dead_code)]
const kAudioFileFileNotFoundError: OSStatus = -43;
const kAudioFileBadPropertySizeError: OSStatus = fourcc(b"!siz") as _;
const kAudioFileUnsupportedProperty: OSStatus = fourcc(b"pty?") as _;
const kAudioFileUnsupportedFileTypeError: OSStatus = fourcc(b"typ?") as _;
const kAudioFileUnspecifiedError: OSStatus = fourcc(b"wht?") as _;

type AudioFilePermissions = i8;
pub const kAudioFileReadPermission: AudioFilePermissions = 1;

/// Usually a FourCC.
type AudioFileTypeID = u32;
const kAudioFileCAFType: AudioFileTypeID = fourcc(b"caff");

/// Usually a FourCC.
type AudioFilePropertyID = u32;
pub const kAudioFilePropertyDataFormat: AudioFilePropertyID = fourcc(b"dfmt");
const kAudioFilePropertyAudioDataByteCount: AudioFilePropertyID = fourcc(b"bcnt");
const kAudioFilePropertyAudioDataPacketCount: AudioFilePropertyID = fourcc(b"pcnt");
pub const kAudioFilePropertyPacketSizeUpperBound: AudioFilePropertyID = fourcc(b"pkub");
const kAudioFilePropertyMagicCookieData: AudioFilePropertyID = fourcc(b"mgic");
const kAudioFilePropertyChannelLayout: AudioFilePropertyID = fourcc(b"cmap");

pub fn AudioFileOpenURL(
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
    // Hints for well-known types are ignored as well.
    match in_file_type_hint {
        0 => {}
        kAudioFileCAFType => {
            log!("Ignoring 'caff' file type hint for AudioFileOpenURL()");
        }
        _ => unimplemented!(),
    }

    let path = to_rust_path(env, in_file_ref);
    let audio_file = match audio::AudioFile::open_for_reading(path, &env.fs) {
        Ok(audio_file) => audio_file,
        Err(error) => {
            log!(
                "Warning: AudioFileOpenURL() for path {:?} failed",
                in_file_ref
            );
            return match error {
                audio::AudioFileOpenError::FileDecodeError => kAudioFileUnsupportedFileTypeError,
                _ => kAudioFileUnspecifiedError,
            };
        }
    };

    let host_object = AudioFileHostObject { audio_file };

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

pub fn AudioFileOpenWithCallbacks(
    env: &mut Environment,
    client_data: MutVoidPtr,
    // typedef OSStatus (*AudioFile_ReadProc)
    //      (void *inClientData,
    //       SInt64 inPosition,
    //       UInt32 requestCount,
    //       void *buffer,
    //       UInt32 *actualCount);
    read_callback: GuestFunction,
    // typedef OSStatus (*AudioFile_WriteProc)
    //      (void *inClientData,
    //       SInt64 inPosition,
    //       UInt32 requestCount,
    //       const void *buffer,
    //       UInt32 *actualCount);
    _write_callback: GuestFunction,
    // typedef SInt64 (*AudioFile_GetSizeProc)(void *inClientData);
    getsize_callback: GuestFunction,
    // typedef OSStatus (*AudioFile_SetSizeProc)
    //      (void *inClientData, SInt64 inSize);
    _setsize_callback: GuestFunction,
    in_file_type_hint: AudioFileTypeID,
    out_audio_file: MutPtr<AudioFileID>,
) -> OSStatus {
    if _write_callback.to_ptr().is_null() || _setsize_callback.to_ptr().is_null() {
        log_dbg!("AudioFileOpenWithCallbacks() called with (unsupported) write({:?})/set_size({:?}) callbacks!",
            _write_callback,
            _setsize_callback);
    }
    // The hint is optional and is supposed to only be used for certain file
    // formats that can't be uniquely identified, which we don't support so far.
    if in_file_type_hint != 0 {
        log!("Ignoring file type hint for AudioFileOpenWithCallbacks()");
    }

    // TODO: We're just reading in the whole file at once and parsing it here,
    // this should change when streaming parsing is implemented.
    let size: i64 = getsize_callback.call_from_host(env, (client_data,));
    let size: u32 = size.try_into().unwrap();

    assert!(
        size != 0,
        "0 byte size of file for AudioFileOpenWithCallbacks(), likely bad!"
    );

    let data_ptr: MutPtr<u8> = env.mem.alloc(size).cast();
    let bytes_read_ptr: MutPtr<u32> = env.mem.alloc(guest_size_of::<u32>()).cast();

    env.mem.write(bytes_read_ptr, 0);
    log_dbg!(
        "AudioFileOpenWithCallbacks() calling read: {:?}",
        (client_data, 0_i64, size, data_ptr, bytes_read_ptr)
    );
    let status: OSStatus =
        read_callback.call_from_host(env, (client_data, 0_i64, size, data_ptr, bytes_read_ptr));
    if status != 0 {
        log!(
            "AudioFileOpenWithCallbacks() failed read, returning {}",
            fourcc(&status.to_le_bytes())
        );

        return status;
    }

    assert!(
        env.mem.read(bytes_read_ptr) == size,
        "Bytes read != size for AudioFileOpenWithCallbacks(), likely bad!"
    );

    let data_vec = env
        .mem
        .bytes_at(data_ptr, env.mem.read(bytes_read_ptr))
        .to_vec();

    let Ok(audio_file) = audio::AudioFile::read_from_vec(data_vec) else {
        log!("Warning: AudioFileOpenWithCallbacks() failed parse",);
        return kAudioFileUnsupportedFileTypeError;
    };
    let guest_audio_file = env.mem.alloc_and_write(OpaqueAudioFileID { _filler: 0 });

    let host_object = AudioFileHostObject { audio_file };

    State::get(&mut env.framework_state)
        .audio_files
        .insert(guest_audio_file, host_object);

    env.mem.write(out_audio_file, guest_audio_file);

    log_dbg!(
        "AudioFileOpenWithCallbacks() opened, new audio file handle: {:?}",
        guest_audio_file
    );

    0 // success
}

fn property_size(property_id: AudioFilePropertyID) -> GuestUSize {
    match property_id {
        kAudioFilePropertyDataFormat => guest_size_of::<AudioStreamBasicDescription>(),
        kAudioFilePropertyAudioDataByteCount => guest_size_of::<u64>(),
        kAudioFilePropertyAudioDataPacketCount => guest_size_of::<u64>(),
        kAudioFilePropertyPacketSizeUpperBound => guest_size_of::<u32>(),
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

pub fn AudioFileGetProperty(
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
        kAudioFilePropertyDataFormat => {
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
        _ => unreachable!(),
    }

    0 // success
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

fn AudioFileReadPacketData(
    env: &mut Environment,
    in_audio_file: AudioFileID,
    in_use_cache: bool,
    out_num_bytes: MutPtr<u32>,
    out_packet_descriptions: MutVoidPtr, // unimplemented
    in_starting_packet: i64,
    io_num_packets: MutPtr<u32>,
    out_buffer: MutVoidPtr,
) -> OSStatus {
    // TODO: real VBR support
    AudioFileReadPackets(
        env,
        in_audio_file,
        in_use_cache,
        out_num_bytes,
        out_packet_descriptions,
        in_starting_packet,
        io_num_packets,
        out_buffer,
    )
}

pub fn AudioFileReadPackets(
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
    if !out_packet_descriptions.is_null() {
        log!("Warning: ignoring non-null out_packet_descriptions in AudioFileReadPackets()");
    }

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

pub fn AudioFileClose(env: &mut Environment, in_audio_file: AudioFileID) -> OSStatus {
    return_if_null!(in_audio_file);

    let Some(_host_object) = State::get(&mut env.framework_state)
        .audio_files
        .remove(&in_audio_file)
    else {
        log!(
            "Bad AudioFileClose for {:?} (likely double close), ignoring!",
            in_audio_file
        );
        return kAudioFileUnspecifiedError;
    };
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
    export_c_func!(AudioFileReadPacketData(_, _, _, _, _, _, _)),
    export_c_func!(AudioFileOpenWithCallbacks(_, _, _, _, _, _, _)),
    export_c_func!(AudioFileClose(_)),
];
