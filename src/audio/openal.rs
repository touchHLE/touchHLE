//! OpenAL bindings (linked to OpenAL Soft via [touchHLE_openal_soft_wrapper]).
//!
//! See `vendor/openal-soft/AL/` for the headers this should mirror.

// === alc.h ===

#[allow(dead_code)]
pub mod alc_types {
    use std::ffi;

    // TODO: If Rust ever stabilises a good way to do opaque types, use that
    // instead of a typedef of void.
    /// Opaque type.
    pub type ALCdevice = ffi::c_void;
    /// Opaque type.
    pub type ALCcontext = ffi::c_void;

    pub type ALCboolean = ffi::c_char;
    pub type ALCchar = ffi::c_char;
    pub type ALCbyte = ffi::c_schar;
    pub type ALCubyte = ffi::c_uchar;
    pub type ALCshort = ffi::c_short;
    pub type ALCushort = ffi::c_ushort;
    pub type ALCint = ffi::c_int;
    pub type ALCuint = ffi::c_uint;
    pub type ALCsizei = ffi::c_int;
    pub type ALCenum = ffi::c_int;
    pub type ALCfloat = ffi::c_float;
    pub type ALCdouble = ffi::c_double;
    pub type ALCvoid = ffi::c_void;
}
use alc_types::*;

pub const ALC_FALSE: ALCboolean = 0;
#[allow(dead_code)]
pub const ALC_TRUE: ALCboolean = 1;

#[link(name = "openal")] // see also src/audio/openal_soft_wrapper/build.rs
extern "C" {
    pub fn alcOpenDevice(devicename: *const ALCchar) -> *mut ALCdevice;
    pub fn alcCloseDevice(device: *mut ALCdevice) -> ALCboolean;

    pub fn alcCreateContext(device: *mut ALCdevice, attrlist: *const ALCint) -> *mut ALCcontext;
    pub fn alcDestroyContext(context: *mut ALCcontext);

    pub fn alcMakeContextCurrent(context: *mut ALCcontext) -> ALCboolean;
    pub fn alcGetCurrentContext() -> *mut ALCcontext;

    pub fn alcGetError(device: *mut ALCdevice) -> ALCenum;
}

// === al.h ===

#[allow(dead_code)]
pub mod al_types {
    use std::ffi;

    pub type ALboolean = ffi::c_char;
    pub type ALchar = ffi::c_char;
    pub type ALbyte = ffi::c_schar;
    pub type ALubyte = ffi::c_uchar;
    pub type ALshort = ffi::c_short;
    pub type ALushort = ffi::c_ushort;
    pub type ALint = ffi::c_int;
    pub type ALuint = ffi::c_uint;
    pub type ALsizei = ffi::c_int;
    pub type ALenum = ffi::c_int;
    pub type ALfloat = ffi::c_float;
    pub type ALdouble = ffi::c_double;
    pub type ALvoid = ffi::c_void;
}
use al_types::*;

pub const AL_NO_ERROR: ALenum = 0;

#[link(name = "openal")] // see also src/audio/openal_soft_wrapper/build.rs
extern "C" {
    pub fn alGetError() -> ALenum;

    pub fn alGenSources(n: ALsizei, sources: *mut ALuint);
    pub fn alDeleteSources(n: ALsizei, sources: *const ALuint);

    pub fn alGenBuffers(n: ALsizei, buffers: *mut ALuint);
    pub fn alDeleteBuffers(n: ALsizei, buffers: *const ALuint);
}
