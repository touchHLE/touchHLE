/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! OpenAL bindings (linked to OpenAL Soft).
//!
//! See `vendor/openal-soft/AL/` for the headers this should mirror.
//!
//! This is separated out into its own package so that we can avoid rebuilding
//! OpenAL Soft more often than necessary, and to improve build-time
//! parallelism.

// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]

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

pub const ALC_DEVICE_SPECIFIER: ALCenum = 0x1005;

extern "C" {
    pub fn alcOpenDevice(devicename: *const ALCchar) -> *mut ALCdevice;
    pub fn alcCloseDevice(device: *mut ALCdevice) -> ALCboolean;

    pub fn alcCreateContext(device: *mut ALCdevice, attrlist: *const ALCint) -> *mut ALCcontext;
    pub fn alcDestroyContext(context: *mut ALCcontext);

    pub fn alcProcessContext(context: *mut ALCcontext);
    pub fn alcSuspendContext(context: *mut ALCcontext);

    pub fn alcMakeContextCurrent(context: *mut ALCcontext) -> ALCboolean;
    pub fn alcGetCurrentContext() -> *mut ALCcontext;
    pub fn alcGetContextsDevice(context: *mut ALCcontext) -> *mut ALCdevice;

    pub fn alcGetError(device: *mut ALCdevice) -> ALCenum;

    pub fn alcGetString(device: *mut ALCdevice, param: ALCenum) -> *const ALCchar;
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

pub const AL_MAX_GAIN: ALenum = 0x100E;

pub const AL_SOURCE_STATE: ALenum = 0x1010;

pub const AL_INITIAL: ALenum = 0x1011;
pub const AL_PLAYING: ALenum = 0x1012;
pub const AL_PAUSED: ALenum = 0x1013;
pub const AL_STOPPED: ALenum = 0x1014;

pub const AL_BUFFERS_QUEUED: ALenum = 0x1015;
pub const AL_BUFFERS_PROCESSED: ALenum = 0x1016;

pub const AL_FORMAT_MONO8: ALenum = 0x1100;
pub const AL_FORMAT_MONO16: ALenum = 0x1101;
pub const AL_FORMAT_STEREO8: ALenum = 0x1102;
pub const AL_FORMAT_STEREO16: ALenum = 0x1103;

extern "C" {
    pub fn alGetError() -> ALenum;

    pub fn alDistanceModel(value: ALenum);

    pub fn alGetEnumValue(enumName: *const ALchar) -> ALenum;

    pub fn alIsBuffer(buffer: ALuint) -> ALboolean;
    pub fn alIsSource(source: ALuint) -> ALboolean;

    pub fn alListenerf(param: ALenum, value: ALfloat);
    pub fn alListener3f(param: ALenum, value1: ALfloat, value2: ALfloat, value3: ALfloat);
    pub fn alListenerfv(param: ALenum, values: *const ALfloat);
    pub fn alListeneri(param: ALenum, value: ALint);
    pub fn alListener3i(param: ALenum, value1: ALint, value2: ALint, value3: ALint);
    pub fn alListeneriv(param: ALenum, values: *const ALint);

    pub fn alGetListenerf(param: ALenum, value: *mut ALfloat);
    pub fn alGetListener3f(
        param: ALenum,
        value1: *mut ALfloat,
        value2: *mut ALfloat,
        value3: *mut ALfloat,
    );
    pub fn alGetListenerfv(param: ALenum, values: *mut ALfloat);
    pub fn alGetListeneri(param: ALenum, value: *mut ALint);
    pub fn alGetListener3i(
        param: ALenum,
        value1: *mut ALint,
        value2: *mut ALint,
        value3: *mut ALint,
    );
    pub fn alGetListeneriv(param: ALenum, values: *mut ALint);

    pub fn alGenSources(n: ALsizei, sources: *mut ALuint);
    pub fn alDeleteSources(n: ALsizei, sources: *const ALuint);

    pub fn alSourcef(source: ALuint, param: ALenum, value: ALfloat);
    pub fn alSource3f(
        source: ALuint,
        param: ALenum,
        value1: ALfloat,
        value2: ALfloat,
        value3: ALfloat,
    );
    pub fn alSourcefv(source: ALuint, param: ALenum, values: *const ALfloat);
    pub fn alSourcei(source: ALuint, param: ALenum, value: ALint);
    pub fn alSource3i(source: ALuint, param: ALenum, value1: ALint, value2: ALint, value3: ALint);
    pub fn alSourceiv(source: ALuint, param: ALenum, values: *const ALint);

    pub fn alGetSourcef(source: ALuint, param: ALenum, value: *mut ALfloat);
    pub fn alGetSource3f(
        source: ALuint,
        param: ALenum,
        value1: *mut ALfloat,
        value2: *mut ALfloat,
        value3: *mut ALfloat,
    );
    pub fn alGetSourcefv(source: ALuint, param: ALenum, values: *mut ALfloat);
    pub fn alGetSourcei(source: ALuint, param: ALenum, value: *mut ALint);
    pub fn alGetSource3i(
        source: ALuint,
        param: ALenum,
        value1: *mut ALint,
        value2: *mut ALint,
        value3: *mut ALint,
    );
    pub fn alGetSourceiv(source: ALuint, param: ALenum, values: *mut ALint);

    pub fn alSourcePlay(source: ALuint);
    pub fn alSourcePause(source: ALuint);
    pub fn alSourceStop(source: ALuint);
    pub fn alSourceRewind(source: ALuint);

    pub fn alSourceQueueBuffers(source: ALuint, nb: ALsizei, buffers: *const ALuint);
    pub fn alSourceUnqueueBuffers(source: ALuint, nb: ALsizei, buffers: *mut ALuint);

    pub fn alGenBuffers(n: ALsizei, buffers: *mut ALuint);
    pub fn alDeleteBuffers(n: ALsizei, buffers: *const ALuint);

    pub fn alBufferData(
        buffer: ALuint,
        format: ALenum,
        data: *const ALvoid,
        size: ALsizei,
        samplerate: ALsizei,
    );

    pub fn alDopplerFactor(dopplerFactor: ALfloat);
    pub fn alDopplerVelocity(dopplerVelocity: ALfloat);
    pub fn alSpeedOfSound(speed: ALfloat);
}
