/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `cxxabi.h`
//!
//! Resources:
//! - [Itanium C++ ABI specification](https://itanium-cxx-abi.github.io/cxx-abi/abi.html#dso-dtor-runtime-api)

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::MutVoidPtr;
use crate::Environment;

fn __cxa_atexit(
    _env: &mut Environment,
    _func: GuestFunction, // void (*func)(void *)
    _p: MutVoidPtr,
    _d: MutVoidPtr,
) -> i32 {
    0 // success
}

fn __cxa_finalize(_env: &mut Environment, _d: MutVoidPtr) {}

fn _Unwind_SjLj_Register(_env: &mut Environment, _data: MutVoidPtr) {
    log_dbg!("TODO: _Unwind_SjLj_Register({:?}) (stub)", _data);
}

fn _Unwind_SjLj_Unregister(_env: &mut Environment, _data: MutVoidPtr) {
    log_dbg!("TODO: _Unwind_SjLj_Unregister({:?}) (stub)", _data);
}

fn xmlReadFile(
    _env: &mut Environment,
    _filename: crate::mem::ConstPtr<u8>,
    _encoding: crate::mem::ConstPtr<u8>,
    _options: i32,
) -> MutVoidPtr {
    log_dbg!("TODO: xmlReadFile (stub)");
    crate::mem::Ptr::null()
}

fn xmlDocGetRootElement(_env: &mut Environment, _doc: MutVoidPtr) -> MutVoidPtr {
    log_dbg!("TODO: xmlDocGetRootElement (stub)");
    crate::mem::Ptr::null()
}

fn xmlFreeDoc(_env: &mut Environment, _doc: MutVoidPtr) {
    log_dbg!("TODO: xmlFreeDoc (stub)");
}

fn xmlCleanupMemory(_env: &mut Environment) {
    log_dbg!("TODO: xmlCleanupMemory (stub)");
}

fn xmlFree(_env: &mut Environment, _ptr: MutVoidPtr) {
    log_dbg!("TODO: xmlFree (stub)");
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(__cxa_atexit(_, _, _)),
    export_c_func!(__cxa_finalize(_)),
    export_c_func!(_Unwind_SjLj_Register(_)),
    export_c_func!(_Unwind_SjLj_Unregister(_)),
    export_c_func!(xmlReadFile(_, _, _)),
    export_c_func!(xmlDocGetRootElement(_)),
    export_c_func!(xmlFreeDoc(_)),
    export_c_func!(xmlCleanupMemory()),
    export_c_func!(xmlFree(_)),
];
