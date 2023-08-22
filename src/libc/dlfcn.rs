/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `dlfcn.h` (`dlopen()` and friends)

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, MutVoidPtr, Ptr};
use crate::Environment;

const ALLOWED_LIBRARIES: [Result<&str, &[u8]>; 2] = [
    Ok("/usr/lib/libSystem.B.dylib"),
    Ok("/System/Library/Frameworks/OpenAL.framework/OpenAL"),
];

fn dlopen(env: &mut Environment, path: ConstPtr<u8>, _mode: i32) -> MutVoidPtr {
    // TODO: dlopen() support for real dynamic libraries, and support for all
    // libraries with host implementations.
    assert!(ALLOWED_LIBRARIES.contains(&env.mem.cstr_at_utf8(path)));
    // For convenience, use the path as the handle.
    // TODO: Find out whether the handle is truly opaque on iPhone OS, and if
    // not, where it points.
    path.cast_mut().cast()
}

fn dlsym(env: &mut Environment, handle: MutVoidPtr, symbol: ConstPtr<u8>) -> MutVoidPtr {
    assert!(ALLOWED_LIBRARIES.contains(&env.mem.cstr_at_utf8(handle.cast())));
    // For some reason, the symbols passed to dlsym() don't have the leading _.
    let symbol = format!("_{}", env.mem.cstr_at_utf8(symbol).unwrap());
    // TODO: error handling. dlsym() should just return NULL in this case, but
    // currently it's probably more useful to have the emulator crash if there's
    // no symbol found, since it most likely indicates a missing host function.
    let addr = env
        .dyld
        .create_proc_address(&mut env.mem, &mut env.cpu, &symbol)
        .unwrap_or_else(|_| panic!("dlsym() for unimplemented function {}", symbol));
    Ptr::from_bits(addr.addr_with_thumb_bit())
}

fn dlclose(env: &mut Environment, handle: MutVoidPtr) -> i32 {
    assert!(ALLOWED_LIBRARIES.contains(&env.mem.cstr_at_utf8(handle.cast())));
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(dlopen(_, _)),
    export_c_func!(dlsym(_, _)),
    export_c_func!(dlclose(_)),
];
