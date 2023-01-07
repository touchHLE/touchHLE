//! OpenAL.
//!
//! This is a thin layer on top of OpenAL Soft, see [crate::audio::openal].
//!
//! Resources:
//! - [OpenAL 1.1 specification](https://www.openal.org/documentation/openal-1.1-specification.pdf)

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
    pub fn get(env: &mut Environment) -> &mut Self {
        &mut env.framework_state.openal
    }
}

/// Opaque type in guest memory standing in for [ALCdevice] in host memory.
pub struct GuestALCdevice {
    _filler: u8,
}
impl SafeWrite for GuestALCdevice {}
/// Opaque type in guest memory standing in for [ALCcontext] in host memory.
pub struct GuestALCcontext {
    _filler: u8,
}
impl SafeWrite for GuestALCcontext {}

// === alc.h ===

pub fn alcOpenDevice(env: &mut Environment, devicename: ConstPtr<u8>) -> MutPtr<GuestALCdevice> {
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
pub fn alcCloseDevice(env: &mut Environment, device: MutPtr<GuestALCdevice>) -> bool {
    let host_device = State::get(env).devices.remove(&device).unwrap();
    env.mem.free(device.cast());
    let res = unsafe { al::alcCloseDevice(host_device) };
    log_dbg!("alcCloseDevice({:?}) => {:?}", device, res,);
    res != al::ALC_FALSE
}

pub fn alcGetError(env: &mut Environment, device: MutPtr<GuestALCdevice>) -> i32 {
    let &host_device = State::get(env).devices.get(&device).unwrap();

    let res = unsafe { al::alcGetError(host_device) };
    log_dbg!("alcGetError({:?}) => {:#x}", host_device, res);
    res
}

pub fn alcCreateContext(
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
pub fn alcDestroyContext(env: &mut Environment, context: MutPtr<GuestALCcontext>) {
    let host_context = State::get(env).contexts.remove(&context).unwrap();
    env.mem.free(context.cast());
    unsafe { al::alcDestroyContext(host_context) };
    log_dbg!("alcDestroyContext({:?})", context);
}

pub fn alcMakeContextCurrent(env: &mut Environment, context: MutPtr<GuestALCcontext>) -> bool {
    let host_context = if context.is_null() {
        std::ptr::null_mut()
    } else {
        State::get(env).contexts.get(&context).copied().unwrap()
    };
    let res = unsafe { al::alcMakeContextCurrent(host_context) };
    log_dbg!("alcMakeContextCurrent({:?}) => {:?}", context, res);
    res != al::ALC_FALSE
}

pub fn alcGetProcAddress(
    env: &mut Environment,
    _device: ConstPtr<GuestALCdevice>,
    func_name: ConstPtr<u8>,
) -> ConstVoidPtr {
    // Apple-specific extension that Super Monkey Ball tries to use.
    // Conveniently, if NULL is returned, it just skips trying to use it, so
    // let's do that.
    if env.mem.cstr_at_utf8(func_name) == "alcMacOSXMixerOutputRate" {
        // Warn in case other apps don't check for NULL. The spec doesn't even
        // mention that as a possibility.
        log!("Returning NULL for alcGetProcAddress(..., \"alcMacOSXMixerOutputRate\").");
        return Ptr::null();
    }
    unimplemented!(); // TODO general implementation
}

// TODO: more functions

// === al.h ===

pub fn alGetError(_env: &mut Environment) -> i32 {
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

pub fn alGenSources(env: &mut Environment, n: ALsizei, sources: MutPtr<ALuint>) {
    let n_usize: GuestUSize = n.try_into().unwrap();
    let sources = env.mem.ptr_at_mut(sources, n_usize);
    unsafe { al::alGenSources(n, sources) };
}
pub fn alDeleteSources(env: &mut Environment, n: ALsizei, sources: ConstPtr<ALuint>) {
    let n_usize: GuestUSize = n.try_into().unwrap();
    let sources = env.mem.ptr_at(sources, n_usize);
    unsafe { al::alDeleteSources(n, sources) };
}

pub fn alGenBuffers(env: &mut Environment, n: ALsizei, buffers: MutPtr<ALuint>) {
    let n_usize: GuestUSize = n.try_into().unwrap();
    let buffers = env.mem.ptr_at_mut(buffers, n_usize);
    unsafe { al::alGenBuffers(n, buffers) };
}
pub fn alDeleteBuffers(env: &mut Environment, n: ALsizei, buffers: ConstPtr<ALuint>) {
    let n_usize: GuestUSize = n.try_into().unwrap();
    let buffers = env.mem.ptr_at(buffers, n_usize);
    unsafe { al::alDeleteBuffers(n, buffers) };
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
    export_c_func!(alGenSources(_, _)),
    export_c_func!(alDeleteSources(_, _)),
    export_c_func!(alGenBuffers(_, _)),
    export_c_func!(alDeleteBuffers(_, _)),
];
