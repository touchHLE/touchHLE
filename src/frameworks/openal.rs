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
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, Ptr, SafeWrite};
use crate::Environment;
use std::collections::HashMap;

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
    // NULL means you don't care what device is opened. If an app tries to use
    // a specific device name, it's probably going to be something specific to
    // Apple and fail, so let's assert just in case that happens.
    assert!(devicename.is_null());

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

fn alcGetProcAddress(
    env: &mut Environment,
    _device: ConstPtr<GuestALCdevice>,
    func_name: ConstPtr<u8>,
) -> ConstVoidPtr {
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

fn alListenerf(_env: &mut Environment, param: ALenum, value: ALfloat) {
    unsafe { al::alListenerf(param, value) };
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
fn alSourcei(_env: &mut Environment, source: ALuint, param: ALenum, value: ALint) {
    unsafe { al::alSourcei(source, param, value) };
}
fn alGetSourcef(env: &mut Environment, source: ALuint, param: ALenum, value: MutPtr<ALfloat>) {
    unsafe { al::alGetSourcef(source, param, env.mem.ptr_at_mut(value, 1)) };
}
fn alGetSourcei(env: &mut Environment, source: ALuint, param: ALenum, value: MutPtr<ALint>) {
    unsafe { al::alGetSourcei(source, param, env.mem.ptr_at_mut(value, 1)) };
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
    let data_slice = env.mem.bytes_at(data.cast(), size_usize);
    unsafe {
        al::alBufferData(
            buffer,
            format,
            data_slice.as_ptr() as *const _,
            size,
            samplerate,
        )
    };
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

// TODO: more functions

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(alcOpenDevice(_)),
    export_c_func!(alcCloseDevice(_)),
    export_c_func!(alcGetError(_)),
    export_c_func!(alcCreateContext(_, _)),
    export_c_func!(alcDestroyContext(_)),
    export_c_func!(alcMakeContextCurrent(_)),
    export_c_func!(alcGetProcAddress(_, _)),
    export_c_func!(alGetError()),
    export_c_func!(alDistanceModel(_)),
    export_c_func!(alListenerf(_, _)),
    export_c_func!(alListener3f(_, _, _, _)),
    export_c_func!(alGenSources(_, _)),
    export_c_func!(alDeleteSources(_, _)),
    export_c_func!(alGetSourcef(_, _, _)),
    export_c_func!(alGetSourcei(_, _, _)),
    export_c_func!(alSourcef(_, _, _)),
    export_c_func!(alSourcei(_, _, _)),
    export_c_func!(alSourcePlay(_)),
    export_c_func!(alSourcePause(_)),
    export_c_func!(alSourceStop(_)),
    export_c_func!(alSourceQueueBuffers(_, _, _)),
    export_c_func!(alSourceUnqueueBuffers(_, _, _)),
    export_c_func!(alGenBuffers(_, _)),
    export_c_func!(alDeleteBuffers(_, _)),
    export_c_func!(alBufferData(_, _, _, _, _)),
    export_c_func!(alBufferDataStatic(_, _, _, _, _)),
    export_c_func!(alcMacOSXMixerOutputRate(_)),
];
