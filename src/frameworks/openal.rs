/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! OpenAL.
//!
//! This is a thin layer on top of OpenAL Soft, see [crate::audio::openal].
//!
//! Resources:
//! - [OpenAL 1.1 specification](https://www.openal.org/documentation/openal-1.1-specification.pdf)
//! - Apple's [Technical Note TN2199: OpenAL FAQ for iPhone OS](https://web.archive.org/web/20090826202158/http://developer.apple.com/iPhone/library/technotes/tn2008/tn2199.html) (also available [here](https://developer.apple.com/library/archive/technotes/tn2199/_index.html))

use crate::audio::openal as al;
use crate::audio::openal::al_types::*;
use crate::audio::openal::alc_types::*;
use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::string::strcmp;
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr, SafeWrite};
use crate::Environment;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use touchHLE_openal_soft_wrapper::ALC_DEVICE_SPECIFIER;

#[derive(Default)]
pub struct State {
    devices: HashMap<MutPtr<GuestALCdevice>, *mut ALCdevice>,
    contexts: HashMap<MutPtr<GuestALCcontext>, *mut ALCcontext>,
}
impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.framework_state.openal
    }
}

/// Opaque type in guest memory standing in for [ALCdevice] in host memory.
struct GuestALCdevice {
    _filler: u8,
}
impl SafeWrite for GuestALCdevice {}
/// Opaque type in guest memory standing in for [ALCcontext] in host memory.
struct GuestALCcontext {
    _filler: u8,
}
impl SafeWrite for GuestALCcontext {}

// === alc.h ===

fn alcOpenDevice(env: &mut Environment, devicename: ConstPtr<u8>) -> MutPtr<GuestALCdevice> {
    if !devicename.is_null() {
        // If device name name is not null, we check if it's the one which was
        // obtained from a call to alcGetString(NULL, ALC_DEVICE_SPECIFIER)

        let d_name = alcGetString(env, Ptr::null(), ALC_DEVICE_SPECIFIER);
        assert_eq!(strcmp(env, d_name, devicename), 0);
        env.mem.free(d_name.cast_mut().cast());
    }

    let res = unsafe { al::alcOpenDevice(std::ptr::null()) };
    if res.is_null() {
        log_dbg!("alcOpenDevice(NULL) returned NULL");
        return Ptr::null();
    }

    let guest_res = env.mem.alloc_and_write(GuestALCdevice { _filler: 0 });
    State::get(env).devices.insert(guest_res, res);
    log_dbg!("alcOpenDevice(NULL) => {:?} (host: {:?})", guest_res, res,);
    guest_res
}
fn alcCloseDevice(env: &mut Environment, device: MutPtr<GuestALCdevice>) -> bool {
    let host_device = State::get(env).devices.remove(&device).unwrap();
    env.mem.free(device.cast());
    let res = unsafe { al::alcCloseDevice(host_device) };
    log_dbg!("alcCloseDevice({:?}) => {:?}", device, res,);
    res != al::ALC_FALSE
}

fn alcGetError(env: &mut Environment, device: MutPtr<GuestALCdevice>) -> i32 {
    let &host_device = State::get(env).devices.get(&device).unwrap();

    let res = unsafe { al::alcGetError(host_device) };
    log_dbg!("alcGetError({:?}) => {:#x}", host_device, res);
    res
}

fn alcGetString(
    env: &mut Environment,
    device: MutPtr<GuestALCdevice>,
    param: ALenum,
) -> ConstPtr<u8> {
    assert!(device.is_null());

    let res = unsafe { al::alcGetString(std::ptr::null_mut(), param) };
    let s = unsafe { CStr::from_ptr(res) };
    log_dbg!("alcGetString({:?}) => {:?}", param, s);
    log!("TODO: alcGetString({}) leaks memory", param);
    env.mem.alloc_and_write_cstr(s.to_bytes()).cast_const()
}

fn alcCreateContext(
    env: &mut Environment,
    device: MutPtr<GuestALCdevice>,
    attrlist: ConstPtr<i32>,
) -> MutPtr<GuestALCcontext> {
    assert!(attrlist.is_null()); // unimplemented

    let &host_device = State::get(env).devices.get(&device).unwrap();

    let res = unsafe { al::alcCreateContext(host_device, std::ptr::null()) };
    if res.is_null() {
        log_dbg!("alcCreateContext({:?}, NULL) returned NULL", device);
        return Ptr::null();
    }

    let guest_res = env.mem.alloc_and_write(GuestALCcontext { _filler: 0 });
    State::get(env).contexts.insert(guest_res, res);
    log_dbg!(
        "alcCreateContext({:?}, NULL) => {:?} (host: {:?})",
        device,
        guest_res,
        res,
    );
    guest_res
}
fn alcDestroyContext(env: &mut Environment, context: MutPtr<GuestALCcontext>) {
    let host_context = State::get(env).contexts.remove(&context).unwrap();
    env.mem.free(context.cast());
    unsafe { al::alcDestroyContext(host_context) };
    log_dbg!("alcDestroyContext({:?})", context);
}

fn alcProcessContext(env: &mut Environment, context: MutPtr<GuestALCcontext>) {
    if context.is_null() {
        log!("alcProcessContext() is called with NULL context, ignoring");
        return;
    }
    let host_context = State::get(env).contexts.get(&context).copied().unwrap();
    unsafe { al::alcProcessContext(host_context) }
}
fn alcSuspendContext(env: &mut Environment, context: MutPtr<GuestALCcontext>) {
    if context.is_null() {
        log!("alcSuspendContext() is called with NULL context, ignoring");
        return;
    }
    let host_context = State::get(env).contexts.get(&context).copied().unwrap();
    unsafe { al::alcSuspendContext(host_context) }
}

fn alcMakeContextCurrent(env: &mut Environment, context: MutPtr<GuestALCcontext>) -> bool {
    let host_context = if context.is_null() {
        std::ptr::null_mut()
    } else {
        State::get(env).contexts.get(&context).copied().unwrap()
    };
    let res = unsafe { al::alcMakeContextCurrent(host_context) };
    log_dbg!("alcMakeContextCurrent({:?}) => {:?}", context, res);
    res != al::ALC_FALSE
}

fn alcGetCurrentContext(env: &mut Environment) -> MutPtr<GuestALCcontext> {
    let host_context = unsafe { al::alcGetCurrentContext() };
    if host_context.is_null() {
        Ptr::null()
    } else {
        *State::get(env)
            .contexts
            .iter()
            .find(|(&_guest, &host)| host == host_context)
            .unwrap()
            .0
    }
}

fn alcGetContextsDevice(
    env: &mut Environment,
    context: MutPtr<GuestALCcontext>,
) -> MutPtr<GuestALCdevice> {
    let host_context = State::get(env).contexts.get(&context).copied().unwrap();
    let host_device = unsafe { al::alcGetContextsDevice(host_context) };
    *State::get(env)
        .devices
        .iter()
        .find(|(&_guest, &host)| host == host_device)
        .unwrap()
        .0
}

fn alcGetProcAddress(
    env: &mut Environment,
    _device: ConstPtr<GuestALCdevice>,
    func_name: ConstPtr<u8>,
) -> MutVoidPtr {
    let mangled_func_name = format!("_{}", env.mem.cstr_at_utf8(func_name).unwrap());
    assert!(mangled_func_name.starts_with("_al"));

    if let Ok(ptr) = env
        .dyld
        .create_proc_address(&mut env.mem, &mut env.cpu, &mangled_func_name)
    {
        Ptr::from_bits(ptr.addr_with_thumb_bit())
    } else {
        panic!(
            "Request for procedure address for unimplemented OpenAL function {}",
            mangled_func_name
        );
    }
}

// TODO: more functions

// === al.h ===

fn alGetError(_env: &mut Environment) -> i32 {
    // Super Monkey Ball tries to use this function (rather than alcGetError) to
    // figure out whether opening the device succeeded. This is not correct and
    // seems to be a bug. Presumably iPhone OS doesn't mind this, but OpenAL
    // Soft returns an error in this case, and the game skips the rest of its
    // audio initialization.
    if unsafe { al::alcGetCurrentContext() }.is_null() {
        log!("alGetError() called with no current context. Ignoring and returning AL_NO_ERROR for compatibility with Super Monkey Ball.");
        return al::AL_NO_ERROR;
    }

    let res = unsafe { al::alGetError() };
    log_dbg!("alGetError() => {:#x}", res);
    res
}

fn alDistanceModel(_env: &mut Environment, value: ALenum) {
    unsafe { al::alDistanceModel(value) };
}

fn alGetEnumValue(env: &mut Environment, enumName: ConstPtr<u8>) -> ALenum {
    let s = env.mem.cstr_at_utf8(enumName).unwrap();
    let ss = CString::new(s).unwrap();
    let res = unsafe { al::alGetEnumValue(ss.as_ptr()) };
    log_dbg!("alGetEnumValue({:?}) => {:?}", s, res);
    res
}

fn alIsBuffer(_env: &mut Environment, buffer: ALuint) -> ALboolean {
    unsafe { al::alIsBuffer(buffer) }
}

fn alIsSource(_env: &mut Environment, source: ALuint) -> ALboolean {
    unsafe { al::alIsSource(source) }
}

fn alListenerf(_env: &mut Environment, param: ALenum, value: ALfloat) {
    unsafe { al::alListenerf(param, value) };
}
fn alListenerfv(env: &mut Environment, param: ALenum, values: ConstPtr<ALfloat>) {
    // we assume that at least 1 parameter should be passed
    let values = env.mem.ptr_at(values, 1);
    unsafe { al::alListenerfv(param, values) };
}
fn alListener3f(
    _env: &mut Environment,

    param: ALenum,
    value1: ALfloat,
    value2: ALfloat,
    value3: ALfloat,
) {
    unsafe { al::alListener3f(param, value1, value2, value3) };
}
fn alListeneri(_env: &mut Environment, param: ALenum, value: ALint) {
    unsafe { al::alListeneri(param, value) };
}
fn alListener3i(
    _env: &mut Environment,

    param: ALenum,
    value1: ALint,
    value2: ALint,
    value3: ALint,
) {
    unsafe { al::alListener3i(param, value1, value2, value3) };
}
fn alListeneriv(env: &mut Environment, param: ALenum, values: ConstPtr<ALint>) {
    let values = env.mem.ptr_at(values, 3); // upper bound
    unsafe { al::alListeneriv(param, values) };
}

fn alGetListenerf(env: &mut Environment, param: ALenum, value: MutPtr<ALfloat>) {
    unsafe { al::alGetListenerf(param, env.mem.ptr_at_mut(value, 1)) };
}
fn alGetListener3f(
    env: &mut Environment,

    param: ALenum,
    value1: MutPtr<ALfloat>,
    value2: MutPtr<ALfloat>,
    value3: MutPtr<ALfloat>,
) {
    let mut values = [0.0; 3];
    unsafe { al::alGetListener3f(param, &mut values[0], &mut values[1], &mut values[2]) };
    env.mem.write(value1, values[0]);
    env.mem.write(value2, values[1]);
    env.mem.write(value3, values[2]);
}
fn alGetListenerfv(env: &mut Environment, param: ALenum, values: MutPtr<ALfloat>) {
    let values = env.mem.ptr_at_mut(values, 3); // upper bound
    unsafe { al::alGetListenerfv(param, values) };
}
fn alGetListeneri(env: &mut Environment, param: ALenum, value: MutPtr<ALint>) {
    unsafe { al::alGetListeneri(param, env.mem.ptr_at_mut(value, 1)) };
}
fn alGetListener3i(
    env: &mut Environment,

    param: ALenum,
    value1: MutPtr<ALint>,
    value2: MutPtr<ALint>,
    value3: MutPtr<ALint>,
) {
    let mut values = [0; 3];
    unsafe { al::alGetListener3i(param, &mut values[0], &mut values[1], &mut values[2]) };
    env.mem.write(value1, values[0]);
    env.mem.write(value2, values[1]);
    env.mem.write(value3, values[2]);
}
fn alGetListeneriv(env: &mut Environment, param: ALenum, values: MutPtr<ALint>) {
    let values = env.mem.ptr_at_mut(values, 3); // upper bound
    unsafe { al::alGetListeneriv(param, values) };
}

fn alGenSources(env: &mut Environment, n: ALsizei, sources: MutPtr<ALuint>) {
    let n_usize: GuestUSize = n.try_into().unwrap();
    let sources = env.mem.ptr_at_mut(sources, n_usize);
    unsafe { al::alGenSources(n, sources) };
}
fn alDeleteSources(env: &mut Environment, n: ALsizei, sources: ConstPtr<ALuint>) {
    let n_usize: GuestUSize = n.try_into().unwrap();
    let sources = env.mem.ptr_at(sources, n_usize);
    unsafe { al::alDeleteSources(n, sources) };
}

fn alSourcef(_env: &mut Environment, source: ALuint, param: ALenum, value: ALfloat) {
    unsafe { al::alSourcef(source, param, value) };
}
fn alSourcefv(env: &mut Environment, source: ALuint, param: ALenum, values: ConstPtr<ALfloat>) {
    // we assume that at least 1 parameter should be passed
    let values = env.mem.ptr_at(values, 1);
    unsafe { al::alSourcefv(source, param, values) };
}
fn alSource3f(
    _env: &mut Environment,
    source: ALuint,
    param: ALenum,
    value1: ALfloat,
    value2: ALfloat,
    value3: ALfloat,
) {
    unsafe { al::alSource3f(source, param, value1, value2, value3) };
}
fn alSourcei(_env: &mut Environment, source: ALuint, param: ALenum, value: ALint) {
    unsafe { al::alSourcei(source, param, value) };
}
fn alSource3i(
    _env: &mut Environment,
    source: ALuint,
    param: ALenum,
    value1: ALint,
    value2: ALint,
    value3: ALint,
) {
    unsafe { al::alSource3i(source, param, value1, value2, value3) };
}
fn alSourceiv(env: &mut Environment, source: ALuint, param: ALenum, values: ConstPtr<ALint>) {
    let values = env.mem.ptr_at(values, 3); // upper bound
    unsafe { al::alSourceiv(source, param, values) };
}

fn alGetSourcef(env: &mut Environment, source: ALuint, param: ALenum, value: MutPtr<ALfloat>) {
    unsafe { al::alGetSourcef(source, param, env.mem.ptr_at_mut(value, 1)) };
}
fn alGetSource3f(
    env: &mut Environment,
    source: ALuint,
    param: ALenum,
    value1: MutPtr<ALfloat>,
    value2: MutPtr<ALfloat>,
    value3: MutPtr<ALfloat>,
) {
    let mut values = [0.0; 3];
    unsafe {
        al::alGetSource3f(
            source,
            param,
            &mut values[0],
            &mut values[1],
            &mut values[2],
        )
    };
    env.mem.write(value1, values[0]);
    env.mem.write(value2, values[1]);
    env.mem.write(value3, values[2]);
}
fn alGetSourcefv(env: &mut Environment, source: ALuint, param: ALenum, values: MutPtr<ALfloat>) {
    let values = env.mem.ptr_at_mut(values, 3); // upper bound
    unsafe { al::alGetSourcefv(source, param, values) };
}
fn alGetSourcei(env: &mut Environment, source: ALuint, param: ALenum, value: MutPtr<ALint>) {
    unsafe { al::alGetSourcei(source, param, env.mem.ptr_at_mut(value, 1)) };
}
fn alGetSource3i(
    env: &mut Environment,
    source: ALuint,
    param: ALenum,
    value1: MutPtr<ALint>,
    value2: MutPtr<ALint>,
    value3: MutPtr<ALint>,
) {
    let mut values = [0; 3];
    unsafe {
        al::alGetSource3i(
            source,
            param,
            &mut values[0],
            &mut values[1],
            &mut values[2],
        )
    };
    env.mem.write(value1, values[0]);
    env.mem.write(value2, values[1]);
    env.mem.write(value3, values[2]);
}
fn alGetSourceiv(env: &mut Environment, source: ALuint, param: ALenum, values: MutPtr<ALint>) {
    let values = env.mem.ptr_at_mut(values, 3); // upper bound
    unsafe { al::alGetSourceiv(source, param, values) };
}

fn alSourcePlay(_env: &mut Environment, source: ALuint) {
    unsafe { al::alSourcePlay(source) };
}
fn alSourcePause(_env: &mut Environment, source: ALuint) {
    unsafe { al::alSourcePause(source) };
}
fn alSourceStop(_env: &mut Environment, source: ALuint) {
    unsafe { al::alSourceStop(source) };
}
fn alSourceRewind(_env: &mut Environment, source: ALuint) {
    unsafe { al::alSourceRewind(source) };
}

fn alSourceQueueBuffers(
    env: &mut Environment,
    source: ALuint,
    nb: ALsizei,
    buffers: ConstPtr<ALuint>,
) {
    let nb_usize: GuestUSize = nb.try_into().unwrap();
    let buffers = env.mem.ptr_at(buffers, nb_usize);
    unsafe { al::alSourceQueueBuffers(source, nb, buffers) }
}
fn alSourceUnqueueBuffers(
    env: &mut Environment,
    source: ALuint,
    nb: ALsizei,
    buffers: MutPtr<ALuint>,
) {
    // Apple's sample code for a looping sound effect contains a function called
    // SoundEngineEffect::ClearSourceBuffers() that has the following pattern:
    //
    //    alGetSourcei(source, AL_BUFFERS_QUEUED, &n);
    //    alSourceUnqueueBuffers(source, n, &buffers);
    //
    // Unfortunately, this is incorrect code in some circumstances: unqueueing
    // buffers while they are playing is not permitted by the OpenAL spec! Maybe
    // it worked with Apple's OpenAL implementation for some reason, but OpenAL
    // Soft does not tolerate this, so many apps that used this sample code
    // (e.g. Super Monkey Ball) run into an unexpected OpenAL error.
    //
    // Limiting the number dequeued seems to be an effective workaround for the
    // apps that have been tested. That sample code isn't interested in actually
    // using the returned buffer IDs, so it's no problem that we write too few.
    let buffers_processed = {
        let mut val = 0;
        unsafe { al::alGetSourcei(source, al::AL_BUFFERS_PROCESSED, &mut val) };
        val
    };
    let nb = if buffers_processed < nb {
        log_dbg!("Applying workaround for Apple sample code bug: ignoring unqueueing of {}/{} processed buffers from source {}", nb, buffers_processed, source);
        buffers_processed
    } else {
        nb
    };

    let nb_usize: GuestUSize = nb.try_into().unwrap();
    let buffers = env.mem.ptr_at_mut(buffers, nb_usize);
    unsafe { al::alSourceUnqueueBuffers(source, nb, buffers) }
}

fn alGenBuffers(env: &mut Environment, n: ALsizei, buffers: MutPtr<ALuint>) {
    let n_usize: GuestUSize = n.try_into().unwrap();
    let buffers = env.mem.ptr_at_mut(buffers, n_usize);
    unsafe { al::alGenBuffers(n, buffers) };
}
fn alDeleteBuffers(env: &mut Environment, n: ALsizei, buffers: ConstPtr<ALuint>) {
    let n_usize: GuestUSize = n.try_into().unwrap();
    let buffers = env.mem.ptr_at(buffers, n_usize);
    unsafe { al::alDeleteBuffers(n, buffers) };
}

fn alBufferData(
    env: &mut Environment,
    buffer: ALuint,
    format: ALenum,
    data: ConstVoidPtr,
    size: ALsizei,
    samplerate: ALsizei,
) {
    let size_usize: GuestUSize = size.try_into().unwrap();
    let data_ptr: *const ALvoid = if data.is_null() {
        std::ptr::null()
    } else {
        let data_slice = env.mem.bytes_at(data.cast(), size_usize);
        data_slice.as_ptr() as *const _
    };
    unsafe { al::alBufferData(buffer, format, data_ptr, size, samplerate) };
}

/// This is an Apple extension that treats the data passed as a static buffer
/// rather than a temporary one, which means it never has to be copied.
/// OpenAL Soft doesn't support this, so we pass through to `alBufferData`
/// and hope the guest app doesn't rely on the static-ness (it shouldn't).
fn alBufferDataStatic(
    env: &mut Environment,
    buffer: ALuint,
    format: ALenum,
    data: ConstVoidPtr,
    size: ALsizei,
    samplerate: ALsizei,
) {
    alBufferData(env, buffer, format, data, size, samplerate);
}

// Apple-specific extension to OpenAL
fn alcMacOSXMixerOutputRate(_env: &mut Environment, value: ALdouble) {
    log!("App wants to set mixer output sample rate to {} Hz", value);
}
fn alcMacOSXGetMixerOutputRate(_env: &mut Environment) -> ALdouble {
    // Default was checked on iPhone 3GS, iOS 4.0.1
    log!("App wants to get mixer output sample rate, returning default 0");
    0.0
}

fn alDopplerFactor(_env: &mut Environment, value: ALfloat) {
    unsafe { al::alDopplerFactor(value) };
}

fn alDopplerVelocity(env: &mut Environment, value: ALfloat) {
    // Apparently wolf3d sets doppler velocity to zero, but this results in
    // muting all of the audio with Open AL 1.1 soft implementation!
    // Check "A note for OpenAL library implementors regarding OpenAL 1.0" from
    // OpenAL 1.1 specs for more info
    let bundle_id = env.bundle.bundle_identifier();
    if bundle_id.starts_with("com.zodttd.wolf3d") || bundle_id.starts_with("com.idsoftware.wolf3d")
    {
        log_dbg!("Applying game-specific hack for Wolf3D-iOS: ignoring 0.0 doppler velocity.");
        assert_eq!(value, 0.0);
        return;
    }
    unsafe { al::alDopplerVelocity(value) };
}

fn alSpeedOfSound(_env: &mut Environment, value: ALfloat) {
    unsafe { al::alSpeedOfSound(value) };
}

// TODO: more functions

// Note: For some reasons Wolf3d registers many OpenAl functions, but actually
// uses only few ones. To workaround this, we just provide stubs.

fn alcGetEnumValue(
    _env: &mut Environment,
    _device: MutPtr<GuestALCdevice>,
    _enumName: ConstPtr<u8>,
) -> ALenum {
    todo!();
}
fn alcGetIntegerv(
    _env: &mut Environment,
    _device: MutPtr<GuestALCdevice>,
    _param: ALenum,
    _size: ALCsizei,
    _values: MutPtr<ALCint>,
) {
    todo!();
}
fn alcIsExtensionPresent(
    _env: &mut Environment,
    _device: MutPtr<GuestALCdevice>,
    _extName: ConstPtr<u8>,
) -> ALCboolean {
    0
}
fn alGetBufferf(_env: &mut Environment, _buffer: ALuint, _param: ALenum, _value: MutPtr<ALfloat>) {
    todo!();
}
fn alGetBufferi(_env: &mut Environment, _buffer: ALuint, _param: ALenum, _value: MutPtr<ALint>) {
    todo!();
}
fn alEnable(_env: &mut Environment, _capability: ALenum) {
    todo!();
}
fn alDisable(_env: &mut Environment, _capability: ALenum) {
    todo!();
}
fn alGetBoolean(_env: &mut Environment, _param: ALenum) -> ALboolean {
    todo!();
}
fn alGetBooleanv(_env: &mut Environment, _param: ALenum, _values: MutPtr<ALboolean>) {
    todo!();
}
fn alGetDouble(_env: &mut Environment, _param: ALenum) -> ALdouble {
    todo!();
}
fn alGetDoublev(_env: &mut Environment, _param: ALenum, _values: MutPtr<ALdouble>) {
    todo!();
}
fn alGetFloat(_env: &mut Environment, _param: ALenum) -> ALfloat {
    todo!();
}
fn alGetFloatv(_env: &mut Environment, _param: ALenum, _values: MutPtr<ALfloat>) {
    todo!();
}
fn alGetInteger(_env: &mut Environment, _param: ALenum) -> ALint {
    todo!();
}
fn alGetIntegerv(_env: &mut Environment, _param: ALenum, _values: MutPtr<ALint>) {
    todo!();
}
fn alGetProcAddress(env: &mut Environment, funcName: ConstPtr<u8>) -> MutVoidPtr {
    alcGetProcAddress(env, Ptr::null(), funcName)
}
fn alGetString(_env: &mut Environment, _param: ALenum) -> ConstPtr<u8> {
    todo!();
}
fn alIsExtensionPresent(_env: &mut Environment, _extName: ConstPtr<u8>) -> ALboolean {
    todo!();
}
fn alIsEnabled(_env: &mut Environment, _capability: ALenum) -> ALboolean {
    todo!();
}
fn alSourcePlayv(_env: &mut Environment, _nsources: ALsizei, _sources: ConstPtr<ALuint>) {
    todo!();
}
fn alSourcePausev(_env: &mut Environment, _nsources: ALsizei, _sources: ConstPtr<ALuint>) {
    todo!();
}
fn alSourceStopv(_env: &mut Environment, _nsources: ALsizei, _sources: ConstPtr<ALuint>) {
    todo!();
}
fn alSourceRewindv(_env: &mut Environment, _nsources: ALsizei, _sources: ConstPtr<ALuint>) {
    todo!();
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(alcOpenDevice(_)),
    export_c_func!(alcCloseDevice(_)),
    export_c_func!(alcGetError(_)),
    export_c_func!(alcCreateContext(_, _)),
    export_c_func!(alcDestroyContext(_)),
    export_c_func!(alcProcessContext(_)),
    export_c_func!(alcSuspendContext(_)),
    export_c_func!(alcMakeContextCurrent(_)),
    export_c_func!(alcGetProcAddress(_, _)),
    export_c_func!(alGetError()),
    export_c_func!(alDistanceModel(_)),
    export_c_func!(alListenerf(_, _)),
    export_c_func!(alListener3f(_, _, _, _)),
    export_c_func!(alListenerfv(_, _)),
    export_c_func!(alListeneri(_, _)),
    export_c_func!(alListener3i(_, _, _, _)),
    export_c_func!(alListeneriv(_, _)),
    export_c_func!(alGetListenerf(_, _)),
    export_c_func!(alGetListener3f(_, _, _, _)),
    export_c_func!(alGetListenerfv(_, _)),
    export_c_func!(alGetListeneri(_, _)),
    export_c_func!(alGetListener3i(_, _, _, _)),
    export_c_func!(alGetListeneriv(_, _)),
    export_c_func!(alGenSources(_, _)),
    export_c_func!(alDeleteSources(_, _)),
    export_c_func!(alSourcef(_, _, _)),
    export_c_func!(alSource3f(_, _, _, _, _)),
    export_c_func!(alSourcefv(_, _, _)),
    export_c_func!(alSourcei(_, _, _)),
    export_c_func!(alSource3i(_, _, _, _, _)),
    export_c_func!(alSourceiv(_, _, _)),
    export_c_func!(alGetSourcef(_, _, _)),
    export_c_func!(alGetSource3f(_, _, _, _, _)),
    export_c_func!(alGetSourcefv(_, _, _)),
    export_c_func!(alGetSourcei(_, _, _)),
    export_c_func!(alGetSource3i(_, _, _, _, _)),
    export_c_func!(alGetSourceiv(_, _, _)),
    export_c_func!(alSourcePlay(_)),
    export_c_func!(alSourcePause(_)),
    export_c_func!(alSourceStop(_)),
    export_c_func!(alSourceRewind(_)),
    export_c_func!(alSourceQueueBuffers(_, _, _)),
    export_c_func!(alSourceUnqueueBuffers(_, _, _)),
    export_c_func!(alGenBuffers(_, _)),
    export_c_func!(alDeleteBuffers(_, _)),
    export_c_func!(alBufferData(_, _, _, _, _)),
    export_c_func!(alBufferDataStatic(_, _, _, _, _)),
    export_c_func!(alcMacOSXMixerOutputRate(_)),
    export_c_func!(alcMacOSXGetMixerOutputRate()),
    export_c_func!(alcGetContextsDevice(_)),
    export_c_func!(alcGetCurrentContext()),
    export_c_func!(alcGetEnumValue(_, _)),
    export_c_func!(alcGetIntegerv(_, _, _, _)),
    export_c_func!(alcGetString(_, _)),
    export_c_func!(alcIsExtensionPresent(_, _)),
    export_c_func!(alIsBuffer(_)),
    export_c_func!(alGetBufferf(_, _, _)),
    export_c_func!(alGetBufferi(_, _, _)),
    export_c_func!(alEnable(_)),
    export_c_func!(alDisable(_)),
    export_c_func!(alDopplerFactor(_)),
    export_c_func!(alDopplerVelocity(_)),
    export_c_func!(alGetBoolean(_)),
    export_c_func!(alGetBooleanv(_, _)),
    export_c_func!(alGetDouble(_)),
    export_c_func!(alGetDoublev(_, _)),
    export_c_func!(alGetFloat(_)),
    export_c_func!(alGetFloatv(_, _)),
    export_c_func!(alGetInteger(_)),
    export_c_func!(alGetIntegerv(_, _)),
    export_c_func!(alGetEnumValue(_)),
    export_c_func!(alGetProcAddress(_)),
    export_c_func!(alGetString(_)),
    export_c_func!(alIsExtensionPresent(_)),
    export_c_func!(alIsEnabled(_)),
    export_c_func!(alListenerfv(_, _)),
    export_c_func!(alListeneri(_, _)),
    export_c_func!(alGetListenerf(_, _)),
    export_c_func!(alGetListener3f(_, _, _, _)),
    export_c_func!(alGetListenerfv(_, _)),
    export_c_func!(alGetListeneri(_, _)),
    export_c_func!(alIsSource(_)),
    export_c_func!(alSourcePlayv(_, _)),
    export_c_func!(alSourcePause(_)),
    export_c_func!(alSourcePausev(_, _)),
    export_c_func!(alSourceStopv(_, _)),
    export_c_func!(alSourceRewindv(_, _)),
    export_c_func!(alSpeedOfSound(_)),
];
