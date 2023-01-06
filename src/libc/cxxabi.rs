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
    func: GuestFunction, // void (*func)(void *)
    p: MutVoidPtr,
    d: MutVoidPtr,
) -> i32 {
    // TODO: when this is implemented, make sure it's properly compatible with
    // C atexit.
    log!(
        "TODO: __cxa_atexit({:?}, {:?}, {:?}) (unimplemented)",
        func,
        p,
        d
    );
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(__cxa_atexit(_, _, _))];
