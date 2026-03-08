/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Wrapper for OpenAL usage.
use al_sys::alc_types::{ALCcontext, ALCdevice};
use std::marker::PhantomData;
use touchHLE_openal_soft_wrapper as al_sys;
use touchHLE_openal_soft_wrapper::alc_types::ALCint;

pub use al_sys::al_defines::*;
pub use al_sys::al_types;
pub use al_sys::alc_defines::*;
pub use al_sys::alc_types;
pub use al_sys::{alcCloseDevice, alcGetError, alcGetString, alcOpenDevice};

use al_types::*;

static OPENALMANAGER_INSTANCE_EXISTS: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
pub struct OpenALManager {}
impl OpenALManager {
    pub fn new() -> Result<Self, String> {
        if OPENALMANAGER_INSTANCE_EXISTS.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Err("Only one OpenALManager can exist at a time!".to_string());
        }
        Ok(Self {})
    }
}

impl Drop for OpenALManager {
    fn drop(&mut self) {
        OPENALMANAGER_INSTANCE_EXISTS.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

#[derive(Debug)]
pub struct OpenALContext {
    context: *mut ALCcontext,
    // We don't need to guarantee the lifetime or thread-safety of the device -
    // OpenAL (should) do that for us.
    device: *mut ALCdevice,
}

impl OpenALContext {
    pub fn new(_manager: &mut OpenALManager) -> Result<Self, String> {
        let device = unsafe { al_sys::alcOpenDevice(std::ptr::null()) };
        if device.is_null() {
            return Err("Could not open OpenAL device".to_string());
        }
        unsafe { Self::new_with_device_and_attrlist(_manager, device, std::ptr::null()) }
    }

    pub unsafe fn new_with_device_and_attrlist(
        _manager: &mut OpenALManager,
        device: *mut ALCdevice,
        attrlist: *const ALCint,
    ) -> Result<Self, String> {
        let context = unsafe { al_sys::alcCreateContext(device, attrlist) };
        if context.is_null() {
            return Err("Could not open OpenAL context".to_string());
        }
        log_dbg!(
            "New OpenAL device ({:?}) and context ({:?})",
            device,
            context
        );
        Ok(Self { context, device })
    }

    pub fn make_current<'al_ctx, 'manager: 'al_ctx>(
        &'al_ctx mut self,
        _manager: &'manager mut OpenALManager,
    ) -> OpenAL<'al_ctx> {
        // OpenALsoft already caches the current context, no need to
        // check ourselves.
        let ret = unsafe { al_sys::alcMakeContextCurrent(self.context) };
        assert!(ret == ALC_TRUE);
        OpenAL {
            _al_lifetime: PhantomData,
        }
    }

    pub fn SuspendContext(&mut self) {
        unsafe { al_sys::alcSuspendContext(self.context) };
    }

    pub fn ProcessContext(&mut self) {
        unsafe { al_sys::alcProcessContext(self.context) };
    }

    pub fn GetContextsDevice(&self) -> *mut ALCdevice {
        self.device
    }

    // According to both OpenAL docs and OpenALsoft source code, this is
    // context independent, and can be called without one active.
    pub unsafe fn GetEnumValue(enumName: *const ALchar) -> ALenum {
        al_sys::alGetEnumValue(enumName)
    }
}

impl Drop for OpenALContext {
    fn drop(&mut self) {
        unsafe { al_sys::alcDestroyContext(self.context) };
    }
}

pub struct OpenAL<'al_ctx> {
    _al_lifetime: PhantomData<&'al_ctx ()>,
}

impl OpenAL<'_> {
    pub unsafe fn GetError(&self) -> ALenum {
        al_sys::alGetError()
    }

    pub unsafe fn DistanceModel(&self, value: ALenum) {
        al_sys::alDistanceModel(value)
    }

    pub unsafe fn IsBuffer(&self, buffer: ALuint) -> ALboolean {
        al_sys::alIsBuffer(buffer)
    }
    pub unsafe fn IsSource(&self, source: ALuint) -> ALboolean {
        al_sys::alIsSource(source)
    }

    pub unsafe fn Enable(&self, capability: ALenum) {
        al_sys::alEnable(capability)
    }

    pub unsafe fn GetBufferi(&self, buffer: ALuint, param: ALenum, value: *const ALint) {
        al_sys::alGetBufferi(buffer, param, value)
    }

    pub unsafe fn Listenerf(&self, param: ALenum, value: ALfloat) {
        al_sys::alListenerf(param, value)
    }
    pub unsafe fn Listener3f(
        &self,
        param: ALenum,
        value1: ALfloat,
        value2: ALfloat,
        value3: ALfloat,
    ) {
        al_sys::alListener3f(param, value1, value2, value3)
    }
    pub unsafe fn Listenerfv(&self, param: ALenum, values: *const ALfloat) {
        al_sys::alListenerfv(param, values)
    }
    pub unsafe fn Listeneri(&self, param: ALenum, value: ALint) {
        al_sys::alListeneri(param, value)
    }
    pub unsafe fn Listener3i(&self, param: ALenum, value1: ALint, value2: ALint, value3: ALint) {
        al_sys::alListener3i(param, value1, value2, value3)
    }
    pub unsafe fn Listeneriv(&self, param: ALenum, values: *const ALint) {
        al_sys::alListeneriv(param, values)
    }

    pub unsafe fn GetListenerf(&self, param: ALenum, value: *mut ALfloat) {
        al_sys::alGetListenerf(param, value)
    }
    pub unsafe fn GetListener3f(
        &self,
        param: ALenum,
        value1: *mut ALfloat,
        value2: *mut ALfloat,
        value3: *mut ALfloat,
    ) {
        al_sys::alGetListener3f(param, value1, value2, value3)
    }
    pub unsafe fn GetListenerfv(&self, param: ALenum, values: *mut ALfloat) {
        al_sys::alGetListenerfv(param, values)
    }
    pub unsafe fn GetListeneri(&self, param: ALenum, value: *mut ALint) {
        al_sys::alGetListeneri(param, value)
    }
    pub unsafe fn GetListener3i(
        &self,
        param: ALenum,
        value1: *mut ALint,
        value2: *mut ALint,
        value3: *mut ALint,
    ) {
        al_sys::alGetListener3i(param, value1, value2, value3)
    }
    pub unsafe fn GetListeneriv(&self, param: ALenum, values: *mut ALint) {
        al_sys::alGetListeneriv(param, values)
    }

    pub unsafe fn GenSources(&self, n: ALsizei, sources: *mut ALuint) {
        al_sys::alGenSources(n, sources)
    }
    pub unsafe fn DeleteSources(&self, n: ALsizei, sources: *const ALuint) {
        al_sys::alDeleteSources(n, sources)
    }

    pub unsafe fn Sourcef(&self, source: ALuint, param: ALenum, value: ALfloat) {
        al_sys::alSourcef(source, param, value)
    }
    pub unsafe fn Source3f(
        &self,
        source: ALuint,
        param: ALenum,
        value1: ALfloat,
        value2: ALfloat,
        value3: ALfloat,
    ) {
        al_sys::alSource3f(source, param, value1, value2, value3)
    }
    pub unsafe fn Sourcefv(&self, source: ALuint, param: ALenum, values: *const ALfloat) {
        al_sys::alSourcefv(source, param, values)
    }
    pub unsafe fn Sourcei(&self, source: ALuint, param: ALenum, value: ALint) {
        al_sys::alSourcei(source, param, value)
    }
    pub unsafe fn Source3i(
        &self,
        source: ALuint,
        param: ALenum,
        value1: ALint,
        value2: ALint,
        value3: ALint,
    ) {
        al_sys::alSource3i(source, param, value1, value2, value3)
    }
    pub unsafe fn Sourceiv(&self, source: ALuint, param: ALenum, values: *const ALint) {
        al_sys::alSourceiv(source, param, values)
    }

    pub unsafe fn GetSourcef(&self, source: ALuint, param: ALenum, value: *mut ALfloat) {
        al_sys::alGetSourcef(source, param, value)
    }
    pub unsafe fn GetSource3f(
        &self,
        source: ALuint,
        param: ALenum,
        value1: *mut ALfloat,
        value2: *mut ALfloat,
        value3: *mut ALfloat,
    ) {
        al_sys::alGetSource3f(source, param, value1, value2, value3)
    }
    pub unsafe fn GetSourcefv(&self, source: ALuint, param: ALenum, values: *mut ALfloat) {
        al_sys::alGetSourcefv(source, param, values)
    }
    pub unsafe fn GetSourcei(&self, source: ALuint, param: ALenum, value: *mut ALint) {
        al_sys::alGetSourcei(source, param, value)
    }
    pub unsafe fn GetSource3i(
        &self,
        source: ALuint,
        param: ALenum,
        value1: *mut ALint,
        value2: *mut ALint,
        value3: *mut ALint,
    ) {
        al_sys::alGetSource3i(source, param, value1, value2, value3)
    }
    pub unsafe fn GetSourceiv(&self, source: ALuint, param: ALenum, values: *mut ALint) {
        al_sys::alGetSourceiv(source, param, values)
    }

    pub unsafe fn SourcePlay(&self, source: ALuint) {
        al_sys::alSourcePlay(source)
    }
    pub unsafe fn SourcePause(&self, source: ALuint) {
        al_sys::alSourcePause(source)
    }
    pub unsafe fn SourceStop(&self, source: ALuint) {
        al_sys::alSourceStop(source)
    }
    pub unsafe fn SourceRewind(&self, source: ALuint) {
        al_sys::alSourceRewind(source)
    }

    pub unsafe fn SourceQueueBuffers(&self, source: ALuint, nb: ALsizei, buffers: *const ALuint) {
        al_sys::alSourceQueueBuffers(source, nb, buffers)
    }
    pub unsafe fn SourceUnqueueBuffers(&self, source: ALuint, nb: ALsizei, buffers: *mut ALuint) {
        al_sys::alSourceUnqueueBuffers(source, nb, buffers)
    }

    pub unsafe fn GenBuffers(&self, n: ALsizei, buffers: *mut ALuint) {
        al_sys::alGenBuffers(n, buffers)
    }
    pub unsafe fn DeleteBuffers(&self, n: ALsizei, buffers: *const ALuint) {
        al_sys::alDeleteBuffers(n, buffers)
    }

    pub unsafe fn BufferData(
        &self,
        buffer: ALuint,
        format: ALenum,
        data: *const ALvoid,
        size: ALsizei,
        samplerate: ALsizei,
    ) {
        al_sys::alBufferData(buffer, format, data, size, samplerate)
    }

    pub unsafe fn DopplerFactor(&self, dopplerFactor: ALfloat) {
        al_sys::alDopplerFactor(dopplerFactor)
    }
    pub unsafe fn DopplerVelocity(&self, dopplerVelocity: ALfloat) {
        al_sys::alDopplerVelocity(dopplerVelocity)
    }
    pub unsafe fn SpeedOfSound(&self, speed: ALfloat) {
        al_sys::alSpeedOfSound(speed)
    }
}
