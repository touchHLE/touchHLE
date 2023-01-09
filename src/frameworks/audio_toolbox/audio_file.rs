//! `AudioFile.h` (Audio File Services)

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::cf_url::CFURLRef;
use crate::frameworks::mac_types::OSStatus;
use crate::mem::MutPtr;
use crate::Environment;

type AudioFilePermissions = i8;
const kAudioFileReadPermission: AudioFilePermissions = 1;

/// Usually a FourCC.
type AudioFileTypeID = u32;

struct OpaqueAudioFileID {
    _filler: u8,
}

type AudioFileID = MutPtr<OpaqueAudioFileID>;

fn AudioFileOpenURL(
    _env: &mut Environment,
    in_file_ref: CFURLRef,
    in_permissions: AudioFilePermissions,
    in_file_type_hint: AudioFileTypeID,
    out_audio_file: MutPtr<AudioFileID>,
) -> OSStatus {
    assert!(in_permissions == kAudioFileReadPermission); // unimplemented
    unimplemented!(
        "AudioFileOpenURL({:?}, {:#x}, {:#x}, {:?})",
        in_file_ref,
        in_permissions,
        in_file_type_hint,
        out_audio_file
    );
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(AudioFileOpenURL(_, _, _, _))];
