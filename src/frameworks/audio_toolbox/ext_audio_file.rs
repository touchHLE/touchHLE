/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `ExtAudioFile.h` (Extended Audio File Services)

use crate::audio;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::audio_toolbox::audio_file::{
    AudioFileHostObject,
    AudioFileID, State as AudioFileState,
};
use crate::frameworks::carbon_core::{eofErr, OSStatus};
use crate::frameworks::core_audio_types::{
    debug_fourcc, fourcc,
    kAudioFormatFlagIsPacked, kAudioFormatFlagIsSignedInteger, kAudioFormatLinearPCM,
    AudioStreamBasicDescription,
};
use crate::frameworks::core_foundation::cf_url::CFURLRef;
use crate::frameworks::foundation::ns_url::to_rust_path;
use crate::mem::{guest_size_of, GuestUSize, MutPtr, MutVoidPtr, SafeRead};
use crate::Environment;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct State {
    pub ext_audio_files: HashMap<ExtAudioFileRef, ExtAudioFileHostObject>,
}
impl State {
    pub fn get(framework_state: &mut crate::frameworks::State) -> &mut Self {
        &mut framework_state.audio_toolbox.ext_audio_file
    }
}

pub struct ExtAudioFileHostObject {
    /// The underlying audio file (can be Real or Dummy).
    pub audio_file: AudioFileHostObject,
    /// Client format requested via `kExtAudioFileProperty_ClientDataFormat`.
    /// `None` means "use the file's native format" (no conversion).
    pub client_format: Option<AudioStreamBasicDescription>,
    /// Current read position in *frames* (used for `ExtAudioFileRead`).
    pub frame_position: u64,
    /// When this ExtAudioFile was created by wrapping an existing AudioFileID
    /// we remember that ID so we don't double-free the underlying guest memory.
    pub wrapped_audio_file_id: Option<AudioFileID>,
}

// ---------------------------------------------------------------------------
// Opaque handle type
// ---------------------------------------------------------------------------

#[repr(C, packed)]
pub struct OpaqueExtAudioFileID {
    _filler: u8,
}
unsafe impl SafeRead for OpaqueExtAudioFileID {}

pub type ExtAudioFileRef = MutPtr<OpaqueExtAudioFileID>;

// ---------------------------------------------------------------------------
// Error codes
// ---------------------------------------------------------------------------

const kExtAudioFileError_InvalidProperty: OSStatus = fourcc(b"pty?") as _;
const kExtAudioFileError_InvalidPropertySize: OSStatus = fourcc(b"!siz") as _;
const kExtAudioFileError_NonPCMClientFormat: OSStatus = fourcc(b"!pcm") as _;
const kExtAudioFileError_InvalidOperationOrder: OSStatus = fourcc(b"ord?") as _;
const kExtAudioFileError_InvalidDataFormat: OSStatus = fourcc(b"fmt?") as _;

// ---------------------------------------------------------------------------
// Property IDs
// ---------------------------------------------------------------------------

/// Usually a FourCC.
type ExtAudioFilePropertyID = u32;
const kExtAudioFileProperty_FileDataFormat: ExtAudioFilePropertyID = fourcc(b"ffmt");
const kExtAudioFileProperty_ClientDataFormat: ExtAudioFilePropertyID = fourcc(b"cfmt");
const kExtAudioFileProperty_FileLengthFrames: ExtAudioFilePropertyID = fourcc(b"#frm");
const kExtAudioFileProperty_AudioFile: ExtAudioFilePropertyID = fourcc(b"afil");
const kExtAudioFileProperty_AudioConverter: ExtAudioFilePropertyID = fourcc(b"acnv");

fn property_size(property_id: ExtAudioFilePropertyID) -> Option<GuestUSize> {
    match property_id {
        kExtAudioFileProperty_FileDataFormat => {
            Some(guest_size_of::<AudioStreamBasicDescription>())
        }
        kExtAudioFileProperty_ClientDataFormat => {
            Some(guest_size_of::<AudioStreamBasicDescription>())
        }
        kExtAudioFileProperty_FileLengthFrames => Some(guest_size_of::<i64>()),
        kExtAudioFileProperty_AudioFile => Some(guest_size_of::<AudioFileID>()),
        kExtAudioFileProperty_AudioConverter => Some(guest_size_of::<u32>()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build an `ExtAudioFileHostObject` from an already-opened
//`AudioFileHostObject`
/// and insert it into state, returning the new opaque handle written to
/// `out_ext_audio_file`.
fn register_ext_audio_file(
    env: &mut Environment,
    audio_file: AudioFileHostObject,
    wrapped_id: Option<AudioFileID>,
    out_ext_audio_file: MutPtr<ExtAudioFileRef>,
) -> OSStatus {
    let host_object = ExtAudioFileHostObject {
        audio_file,
        client_format: None,
        frame_position: 0,
        wrapped_audio_file_id: wrapped_id,
    };
    let guest_ref = env.mem.alloc_and_write(OpaqueExtAudioFileID { _filler: 0 });
    State::get(&mut env.framework_state)
        .ext_audio_files
        .insert(guest_ref, host_object);
    env.mem.write(out_ext_audio_file, guest_ref);
    log_dbg!("ExtAudioFile registered, new handle: {:?}", guest_ref);
    0 // success
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn ExtAudioFileOpenURL(
    env: &mut Environment,
    in_url: CFURLRef,
    out_ext_audio_file: MutPtr<ExtAudioFileRef>,
) -> OSStatus {
    return_if_null!(in_url);
    let path = to_rust_path(env, in_url);
    let audio_file = match audio::AudioFile::open_for_reading(path.clone(), &env.fs) {
        Ok(af) => AudioFileHostObject::Real(af),
        Err(e) => {
            log!(
                "Warning: ExtAudioFileOpenURL() failed for {:?}: {:?}. Returning Dummy.",
                path,
                e
            );
            AudioFileHostObject::Dummy {
                format: AudioStreamBasicDescription {
                    sample_rate: 44100.0,
                    format_id: kAudioFormatLinearPCM,
                    format_flags: kAudioFormatFlagIsSignedInteger | kAudioFormatFlagIsPacked,
                    bytes_per_packet: 4,
                    frames_per_packet: 1,
                    bytes_per_frame: 4,
                    channels_per_frame: 2,
                    bits_per_channel: 16,
                    _reserved: 0,
                },
                byte_count: 52920000,
                packet_count: 13230000,
            }
        }
    };

    log_dbg!("ExtAudioFileOpenURL() opened {:?}", in_url);
    register_ext_audio_file(env, audio_file, None, out_ext_audio_file)
}

pub fn ExtAudioFileWrapAudioFileID(
    env: &mut Environment,
    in_audio_file: AudioFileID,
    _in_for_writing: bool,
    out_ext_audio_file: MutPtr<ExtAudioFileRef>,
) -> OSStatus {
    return_if_null!(in_audio_file);

    let audio_file = {
        let host_obj = AudioFileState::get(&mut env.framework_state)
            .audio_files
            .get(&in_audio_file)
            .expect("ExtAudioFileWrapAudioFileID: unknown AudioFileID");

        match host_obj {
            AudioFileHostObject::Real(af) => AudioFileHostObject::Real(af.clone()),
            AudioFileHostObject::Dummy {
                format,
                byte_count,
                packet_count,
            } => AudioFileHostObject::Dummy {
                format: *format,
                byte_count: *byte_count,
                packet_count: *packet_count,
            },
            AudioFileHostObject::Writable {
                format,
                ref data,
                ref user_data,
            } => AudioFileHostObject::Writable {
                format: *format,
                data: data.clone(),
                user_data: user_data.clone(),
            },
        }
    };

    log_dbg!(
        "ExtAudioFileWrapAudioFileID() wrapping AudioFileID {:?}",
        in_audio_file
    );
    register_ext_audio_file(env, audio_file, Some(in_audio_file), out_ext_audio_file)
}

pub fn ExtAudioFileDispose(env: &mut Environment, in_ext_audio_file: ExtAudioFileRef) -> OSStatus {
    return_if_null!(in_ext_audio_file);
    let Some(host_object) = State::get(&mut env.framework_state)
        .ext_audio_files
        .remove(&in_ext_audio_file)
    else {
        log!(
            "Bad ExtAudioFileDispose for {:?} (likely double-dispose), ignoring!",
            in_ext_audio_file
        );
        return kExtAudioFileError_InvalidOperationOrder;
    };

    if host_object.wrapped_audio_file_id.is_some() {
        log_dbg!(
            "ExtAudioFileDispose {:?}: wrapped AudioFileID retained by caller",
            in_ext_audio_file
        );
    }
    env.mem.free(in_ext_audio_file.cast());
    log_dbg!(
        "ExtAudioFileDispose() destroyed handle {:?}",
        in_ext_audio_file
    );
    0 // success
}

pub fn ExtAudioFileGetPropertyInfo(
    env: &mut Environment,
    in_ext_audio_file: ExtAudioFileRef,
    in_property_id: ExtAudioFilePropertyID,
    out_size: MutPtr<u32>,
    out_writable: MutPtr<u32>,
) -> OSStatus {
    return_if_null!(in_ext_audio_file);
    let Some(size) = property_size(in_property_id) else {
        log!(
            "Warning: ExtAudioFileGetPropertyInfo() unknown property {}",
            debug_fourcc(in_property_id)
        );
        return kExtAudioFileError_InvalidProperty;
    };

    if in_property_id == kExtAudioFileProperty_AudioConverter {
        if !out_size.is_null() {
            env.mem.write(out_size, 0);
        }
        if !out_writable.is_null() {
            env.mem.write(out_writable, 0);
        }
        return kExtAudioFileError_InvalidProperty;
    }

    if !out_size.is_null() {
        env.mem.write(out_size, size);
    }
    if !out_writable.is_null() {
        let writable: u32 = (in_property_id == kExtAudioFileProperty_ClientDataFormat) as u32;
        env.mem.write(out_writable, writable);
    }
    0 // success
}

pub fn ExtAudioFileGetProperty(
    env: &mut Environment,
    in_ext_audio_file: ExtAudioFileRef,
    in_property_id: ExtAudioFilePropertyID,
    io_data_size: MutPtr<u32>,
    out_property_data: MutVoidPtr,
) -> OSStatus {
    return_if_null!(in_ext_audio_file);
    let Some(required_size) = property_size(in_property_id) else {
        log!(
            "Warning: ExtAudioFileGetProperty() unknown property {}",
            debug_fourcc(in_property_id)
        );
        return kExtAudioFileError_InvalidProperty;
    };
    if env.mem.read(io_data_size) < required_size {
        log!("Warning: ExtAudioFileGetProperty() bad property size");
        return kExtAudioFileError_InvalidPropertySize;
    }

    let host_object = State::get(&mut env.framework_state)
        .ext_audio_files
        .get(&in_ext_audio_file)
        .expect("ExtAudioFileGetProperty: unknown ExtAudioFileRef");

    match in_property_id {
        kExtAudioFileProperty_FileDataFormat => {
            let desc = build_asbd(&host_object.audio_file);
            env.mem.write(out_property_data.cast(), desc);
        }
        kExtAudioFileProperty_ClientDataFormat => {
            let desc = host_object
                .client_format
                .unwrap_or_else(|| build_asbd(&host_object.audio_file));
            env.mem.write(out_property_data.cast(), desc);
        }
        kExtAudioFileProperty_FileLengthFrames => {
            let total_frames: i64 = match &host_object.audio_file {
                AudioFileHostObject::Real(af) => {
                    let desc = af.audio_description();
                    if desc.bytes_per_packet != 0 {
                        (af.byte_count() as i64 * desc.frames_per_packet as i64)
                            / desc.bytes_per_packet as i64
                    } else {
                        0
                    }
                }
                AudioFileHostObject::Dummy {
                    format,
                    packet_count,
                    ..
                } => (*packet_count * format.frames_per_packet as u64) as i64,
                AudioFileHostObject::Writable { format, ref data, .. } => {
                    let bpp = format.bytes_per_packet;
                    let fpp = format.frames_per_packet;
                    if bpp > 0 { (data.len() as i64 * fpp as i64) / bpp as i64 } else { 0 }
                }
            };
            env.mem.write(out_property_data.cast(), total_frames);
        }
        kExtAudioFileProperty_AudioFile => {
            let null_id: u32 = 0;
            env.mem.write(out_property_data.cast(), null_id);
        }
        kExtAudioFileProperty_AudioConverter => {
            return kExtAudioFileError_InvalidProperty;
        }
        other => {
            // ExtAudioFileGetPropertyInfo() filters this out before we get
            // here, but a guest could theoretically construct a call that
            // bypasses that path. Don't crash the host on a bad property id.
            log!(
                "Warning: ExtAudioFileGetProperty(): unknown property {}; returning kExtAudioFileError_InvalidProperty.",
                debug_fourcc(other)
            );
            return kExtAudioFileError_InvalidProperty;
        }
    }

    0 // success
}

pub fn ExtAudioFileSetProperty(
    env: &mut Environment,
    in_ext_audio_file: ExtAudioFileRef,
    in_property_id: ExtAudioFilePropertyID,
    in_data_size: u32,
    in_property_data: MutVoidPtr,
) -> OSStatus {
    return_if_null!(in_ext_audio_file);
    match in_property_id {
        kExtAudioFileProperty_ClientDataFormat => {
            let required = guest_size_of::<AudioStreamBasicDescription>();
            if in_data_size < required {
                log!("Warning: ExtAudioFileSetProperty(ClientDataFormat) bad size");
                return kExtAudioFileError_InvalidPropertySize;
            }
            let new_format: AudioStreamBasicDescription = env.mem.read(in_property_data.cast());
            log_dbg!(
                "ExtAudioFileSetProperty(ClientDataFormat): {:?}",
                new_format
            );
            let Some(host_object) = State::get(&mut env.framework_state)
                .ext_audio_files
                .get_mut(&in_ext_audio_file)
            else {
                log!(
                    "Warning: ExtAudioFileSetProperty(): unknown ExtAudioFileRef {:?}; returning kExtAudioFileError_InvalidOperationOrder.",
                    in_ext_audio_file
                );
                return kExtAudioFileError_InvalidOperationOrder;
            };
            host_object.client_format = Some(new_format);
            0 // success
        }
        kExtAudioFileProperty_FileDataFormat
        | kExtAudioFileProperty_FileLengthFrames
        | kExtAudioFileProperty_AudioFile
        | kExtAudioFileProperty_AudioConverter => {
            log!(
                "Warning: ExtAudioFileSetProperty() read-only property {}",
                debug_fourcc(in_property_id)
            );
            kExtAudioFileError_InvalidProperty
        }
        _ => {
            log!(
                "Warning: ExtAudioFileSetProperty() unknown property {}",
                debug_fourcc(in_property_id)
            );
            kExtAudioFileError_InvalidProperty
        }
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct AudioBuffer {
    number_channels: u32,
    data_byte_size: u32,
    data: MutVoidPtr,
}
unsafe impl SafeRead for AudioBuffer {}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct AudioBufferList {
    number_buffers: u32,
    first_buffer: AudioBuffer,
}
unsafe impl SafeRead for AudioBufferList {}

pub fn ExtAudioFileRead(
    env: &mut Environment,
    in_ext_audio_file: ExtAudioFileRef,
    io_num_frames: MutPtr<u32>,
    io_data: MutPtr<AudioBufferList>,
) -> OSStatus {
    return_if_null!(in_ext_audio_file);

    let frames_requested = env.mem.read(io_num_frames);
    if frames_requested == 0 {
        return 0;
    }

    let mut abl: AudioBufferList = env.mem.read(io_data);
    let out_buffer = abl.first_buffer.data;
    let max_bytes = abl.first_buffer.data_byte_size;

    let Some(host) = State::get(&mut env.framework_state)
        .ext_audio_files
        .get_mut(&in_ext_audio_file)
    else {
        log!(
            "Warning: ExtAudioFileRead(): unknown ExtAudioFileRef {:?}; returning kExtAudioFileError_InvalidOperationOrder.",
            in_ext_audio_file
        );
        return kExtAudioFileError_InvalidOperationOrder;
    };

    // Extract format info safely from real or dummy files
    let (frames_per_packet, packet_size) = match &host.audio_file {
        AudioFileHostObject::Real(af) => (
            af.audio_description().frames_per_packet,
            af.packet_size_fixed(),
        ),
        AudioFileHostObject::Dummy { format, .. } => {
            (format.frames_per_packet, format.bytes_per_packet)
        }
        AudioFileHostObject::Writable { format, .. } => {
            (format.frames_per_packet, format.bytes_per_packet)
        }
    };

    if frames_per_packet == 0 || packet_size == 0 {
        env.mem.write(io_num_frames, 0);
        abl.first_buffer.data_byte_size = 0;
        env.mem.write(io_data, abl);
        return 0;
    }

    let starting_packet = (host.frame_position / frames_per_packet as u64) as i64;
    let packets_to_read = frames_requested.div_ceil(frames_per_packet);
    let mut bytes_to_read = packets_to_read * packet_size;

    if bytes_to_read > max_bytes {
        bytes_to_read = max_bytes;
    }
    bytes_to_read -= bytes_to_read % packet_size;

    if bytes_to_read == 0 || out_buffer.is_null() {
        env.mem.write(io_num_frames, 0);
        abl.first_buffer.data_byte_size = 0;
        env.mem.write(io_data, abl);
        return 0;
    }

    let starting_byte = starting_packet as u64 * packet_size as u64;
    let buffer_slice = env.mem.bytes_at_mut(out_buffer.cast(), bytes_to_read); // ИСПРАВЛЕНИЕ: Убран 'as usize', так как функция ожидает u32

    // Read logic directly without using error-prone map ID transmutes
    let bytes_read = match &mut host.audio_file {
        AudioFileHostObject::Real(af) => af.read_bytes(starting_byte, buffer_slice).unwrap_or(0),
        AudioFileHostObject::Dummy { byte_count, .. } => {
            for b in buffer_slice.iter_mut() {
                *b = 0;
            }
            let max_read = byte_count.saturating_sub(starting_byte);
            std::cmp::min(bytes_to_read as u64, max_read) as usize
        }
        AudioFileHostObject::Writable { ref data, .. } => {
            let start = starting_byte as usize;
            if start >= data.len() {
                0
            } else {
                let available = data.len() - start;
                let to_copy = std::cmp::min(bytes_to_read as usize, available);
                buffer_slice[..to_copy].copy_from_slice(&data[start..start + to_copy]);
                to_copy
            }
        }
    };

    let packets_read = bytes_read as u32 / packet_size;
    let frames_read = packets_read * frames_per_packet;

    host.frame_position += frames_read as u64;

    env.mem.write(io_num_frames, frames_read);
    abl.first_buffer.data_byte_size = bytes_read as u32;
    env.mem.write(io_data, abl);

    if bytes_read < bytes_to_read as usize {
        eofErr
    } else {
        0
    }
}

pub fn ExtAudioFileSeek(
    env: &mut Environment,
    in_ext_audio_file: ExtAudioFileRef,
    in_frame_offset: i64,
) -> OSStatus {
    return_if_null!(in_ext_audio_file);
    let Some(host_object) = State::get(&mut env.framework_state)
        .ext_audio_files
        .get_mut(&in_ext_audio_file)
    else {
        log!(
            "Warning: ExtAudioFileSeek() unknown handle {:?}",
            in_ext_audio_file
        );
        return kExtAudioFileError_InvalidOperationOrder;
    };

    if in_frame_offset < 0 {
        log!("Warning: ExtAudioFileSeek() negative offset not supported");
        return kExtAudioFileError_InvalidOperationOrder;
    }
    host_object.frame_position = in_frame_offset as u64;
    log_dbg!(
        "ExtAudioFileSeek() {:?} -> frame {}",
        in_ext_audio_file,
        in_frame_offset
    );
    0 // success
}

pub fn ExtAudioFileTell(
    env: &mut Environment,
    in_ext_audio_file: ExtAudioFileRef,
    out_frame_offset: MutPtr<i64>,
) -> OSStatus {
    return_if_null!(in_ext_audio_file);
    let Some(host_object) = State::get(&mut env.framework_state)
        .ext_audio_files
        .get(&in_ext_audio_file)
    else {
        log!(
            "Warning: ExtAudioFileTell(): unknown ExtAudioFileRef {:?}; returning kExtAudioFileError_InvalidOperationOrder.",
            in_ext_audio_file
        );
        return kExtAudioFileError_InvalidOperationOrder;
    };
    let pos = host_object.frame_position as i64;
    env.mem.write(out_frame_offset, pos);
    log_dbg!(
        "ExtAudioFileTell() {:?} -> frame {}",
        in_ext_audio_file,
        pos
    );
    0 // success
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn build_asbd(audio_file: &AudioFileHostObject) -> AudioStreamBasicDescription {
    use crate::frameworks::core_audio_types::{
        kAudioFormatFlagIsBigEndian, kAudioFormatFlagIsFloat, kAudioFormatFlagIsPacked,
        kAudioFormatFlagIsSignedInteger, kAudioFormatLinearPCM,
    };

    match audio_file {
        AudioFileHostObject::Real(af) => {
            let audio::AudioDescription {
                sample_rate,
                format,
                bytes_per_packet,
                frames_per_packet,
                channels_per_frame,
                bits_per_channel,
            } = af.audio_description();

            match format {
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
                audio::AudioFormat::Mpeg4Aac => AudioStreamBasicDescription {
                    sample_rate,
                    format_id: fourcc(b"aac "), // Формат AAC
                    format_flags: 0,
                    bytes_per_packet,
                    frames_per_packet,
                    bytes_per_frame: 0,
                    channels_per_frame,
                    bits_per_channel,
                    _reserved: 0,
                },
            }
        }
        AudioFileHostObject::Dummy { format, .. } => *format,
        AudioFileHostObject::Writable { format, .. } => *format,
    }
}

// ---------------------------------------------------------------------------
// Function export table
// ---------------------------------------------------------------------------

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(ExtAudioFileOpenURL(_, _)),
    export_c_func!(ExtAudioFileWrapAudioFileID(_, _, _)),
    export_c_func!(ExtAudioFileDispose(_)),
    export_c_func!(ExtAudioFileGetPropertyInfo(_, _, _, _)),
    export_c_func!(ExtAudioFileGetProperty(_, _, _, _)),
    export_c_func!(ExtAudioFileSetProperty(_, _, _, _)),
    export_c_func!(ExtAudioFileRead(_, _, _)),
    export_c_func!(ExtAudioFileSeek(_, _)),
    export_c_func!(ExtAudioFileTell(_, _)),
];
