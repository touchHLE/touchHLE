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
use crate::mem::{ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr};
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

fn __cxa_finalize(_env: &mut Environment, d: MutVoidPtr) {
    log!("TODO: __cxa_finalize({:?}) (unimplemented)", d);
}

// Static local ("guard variable") initialization. The guard object's first
// byte is the "done" flag; we don't track a pending flag since the guest is
// effectively single-threaded here.
fn __cxa_guard_acquire(env: &mut Environment, guard: MutVoidPtr) -> i32 {
    let done: u8 = env.mem.read(guard.cast::<u8>());
    if done == 0 {
        1 // caller should run the initializer, then call __cxa_guard_release
    } else {
        0 // already initialized
    }
}

fn __cxa_guard_release(env: &mut Environment, guard: MutVoidPtr) {
    env.mem.write(guard.cast::<u8>(), 1u8);
}

fn __cxa_guard_abort(_env: &mut Environment, _guard: MutVoidPtr) {
    // Initializer threw before completing. We never set a pending flag,
    // so there is nothing to undo.
}

fn __cxa_begin_catch(_env: &mut Environment, exception: MutVoidPtr) -> MutVoidPtr {
    log!("TODO: __cxa_begin_catch({:?}) (unimplemented)", exception);
    exception
}

fn __cxa_end_catch(_env: &mut Environment) {
    log!("TODO: __cxa_end_catch() (unimplemented)");
}

fn __cxa_rethrow(_env: &mut Environment) {
    panic!("__cxa_rethrow(): C++ exception rethrowing is not implemented");
}

fn __cxa_call_unexpected(_env: &mut Environment, exception: MutVoidPtr) {
    panic!(
        "__cxa_call_unexpected({:?}): C++ exception specifications are not implemented",
        exception
    );
}

fn __cxa_allocate_exception(env: &mut Environment, size: GuestUSize) -> MutVoidPtr {
    // Never returns null in practice; the guest writes the thrown object here.
    env.mem.alloc(size)
}

fn __cxa_free_exception(_env: &mut Environment, _exception: MutVoidPtr) {}

/// C++ exceptions are thrown by the guest's SjLj unwinder, which we do not
/// implement. Rather than let the guest walk its cleanup tables with an
/// unwinding state we never set up (which manifests as an unrelated null
/// dereference deep inside the guest), fail loudly with the exception's
/// mangled type name so the offending call site can be identified.
fn __cxa_throw(
    env: &mut Environment,
    exception: MutVoidPtr,
    tinfo: MutVoidPtr,
    _destructor: GuestFunction,
) -> MutVoidPtr {
    let name = if tinfo.is_null() {
        "<unknown>".to_string()
    } else {
        // Itanium ABI `std::type_info`: vtable pointer, then `const char*
        // __type_name` pointing at a mangled name (libstdc++ prefixes it
        // with '*', which demanglers ignore).
        let name_ptr: MutVoidPtr = env.mem.read(tinfo.cast::<MutVoidPtr>() + 4u32);
        if name_ptr.is_null() {
            "<null>".to_string()
        } else {
            let bytes = env.mem.cstr_at(name_ptr.cast::<u8>());
            // libstdc++ prefixes mangled names with '*'.
            String::from_utf8_lossy(bytes.trim_ascii_start().strip_prefix(b"*").unwrap_or(bytes))
                .into_owned()
        }
    };
    panic!(
        "__cxa_throw({:?}) of type \"{}\": C++ exception unwinding (SjLj) is not implemented",
        exception, name
    );
}

fn __cxa_pure_virtual(_env: &mut Environment) {
    panic!("Pure virtual function called");
}

// ARM EHABI setjmp/longjmp-based C++ exception handling ("SjLj"). The
// guest's compiler emits calls to _Unwind_SjLj_Register/Unregister at the
// prologue/epilogue of any function with cleanups, so they must exist even
// when no exception is ever thrown (MikuFlick2 does not import
// _Unwind_SjLj_RaiseException or __cxa_throw, so none can be thrown).
// TODO: keep track of registered contexts to support real unwinding.
#[allow(non_snake_case)]
fn _Unwind_SjLj_Register(_env: &mut Environment, _context: MutVoidPtr) -> i32 {
    0 // _URC_NO_REASON
}

#[allow(non_snake_case)]
fn _Unwind_SjLj_Unregister(_env: &mut Environment, _context: MutVoidPtr) {}

#[allow(non_snake_case)]
fn _Unwind_SjLj_Resume(_env: &mut Environment, exception: MutVoidPtr) -> i32 {
    // Only reachable while unwinding an exception past a cleanup frame.
    panic!(
        "_Unwind_SjLj_Resume({:?}): C++ exception unwinding is not implemented",
        exception
    );
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(__cxa_atexit(_, _, _)),
    export_c_func!(__cxa_finalize(_)),
    export_c_func!(__cxa_guard_acquire(_)),
    export_c_func!(__cxa_guard_release(_)),
    export_c_func!(__cxa_guard_abort(_)),
    export_c_func!(__cxa_begin_catch(_)),
    export_c_func!(__cxa_end_catch()),
    export_c_func!(__cxa_rethrow()),
    export_c_func!(__cxa_call_unexpected(_)),
    export_c_func!(__cxa_pure_virtual()),
    export_c_func!(__cxa_allocate_exception(_)),
    export_c_func!(__cxa_free_exception(_)),
    export_c_func!(__cxa_throw(_, _, _)),
    export_c_func!(_Unwind_SjLj_Register(_)),
    export_c_func!(_Unwind_SjLj_Unregister(_)),
    export_c_func!(_Unwind_SjLj_Resume(_)),
];
