/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `cxxabi.h` and the SjLj exception unwinder.
//!
//! Resources:
//! - [Itanium C++ ABI specification](https://itanium-cxx-abi.github.io/cxx-abi/abi.html)
//! - [SjLj-style exception unwinding overview](https://gcc.gnu.org/wiki/SjLjEH)

use crate::abi::{GuestFunction, FRAME_POINTER};
use crate::cpu::Cpu;
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr};
use crate::Environment;
use std::sync::Mutex;

// === atexit / finalize ===

static ATEXIT_HANDLERS: Mutex<Vec<(GuestFunction, MutVoidPtr, MutVoidPtr)>> =
    Mutex::new(Vec::new());

fn __cxa_atexit(_env: &mut Environment, func: GuestFunction, p: MutVoidPtr, d: MutVoidPtr) -> i32 {
    if let Ok(mut handlers) = ATEXIT_HANDLERS.lock() {
        handlers.push((func, p, d));
        0
    } else {
        -1
    }
}

fn __cxa_finalize(_env: &mut Environment, d: MutVoidPtr) {
    let mut to_run = Vec::new();
    if let Ok(mut handlers) = ATEXIT_HANDLERS.lock() {
        if d.is_null() {
            to_run = handlers.drain(..).collect();
        } else {
            let mut i = 0;
            while i < handlers.len() {
                if handlers[i].2 == d {
                    to_run.push(handlers.remove(i));
                } else {
                    i += 1;
                }
            }
        }
    }
    for (_func, _p, _d) in to_run.into_iter().rev() {
        // touchHLE relies on host-process exit for cleanup; we just drop
        // the registered destructors.
    }
}

// === C++ Itanium ABI: guard variables for static initialisation ===
//
// `__cxa_guard_acquire(guard*)` returns 1 if the guarded static still
// needs to be initialised, 0 if it's already initialised. After a
// successful initialisation the caller invokes `__cxa_guard_release`.
//
// On 32-bit ARM the guard is a 64-bit object whose first byte is the
// "initialised" flag. touchHLE is single-threaded for static init so
// we don't need locking. We pre-mark it 1 so a re-entrant call sees
// "already done".

fn __cxa_guard_acquire(env: &mut Environment, guard: MutPtr<u8>) -> i32 {
    let initialized = env.mem.read(guard);
    if initialized != 0 {
        0
    } else {
        env.mem.write(guard, 1);
        1
    }
}

fn __cxa_guard_release(env: &mut Environment, guard: MutPtr<u8>) {
    env.mem.write(guard, 1);
}

fn __cxa_guard_abort(env: &mut Environment, guard: MutPtr<u8>) {
    env.mem.write(guard, 0);
}

// === SjLj exception bypass ===
//
// touchHLE has no real C++ unwinder. Implementing one means parsing
// .gcc_except_table LSDAs, walking the SjLj jmpbuf chain and dispatching
// to the right `catch` clause. That's a multi-week project.
//
// Instead, when a guest exception is thrown we walk the ARM frame-pointer
// chain looking for a return address that lives inside the user code
// segment (below 0x10000000 — guest binaries always load there; the
// system dylibs are mapped at >= 0x38000000). When we find one, we treat
// that frame as if it caught the exception: restore SP and FP for that
// frame, set R0=0 (the "no exception in flight" return) and branch to
// LR. Effectively we make the throwing function return to the first
// app-level frame above it.
//
// This is wrong in the strict sense — destructors of automatic objects
// in skipped frames don't run, the exception object leaks, and the
// caller's local state may be inconsistent — but it lets games that
// throw recoverable errors (parse failures, missing assets, etc.) keep
// running instead of crashing on a NULL-page indirect call.

const APP_CODE_LIMIT: u32 = 0x1000_0000;

fn unwind_to_app_frame(env: &mut Environment) -> bool {
    // The thread's stack typically lives at the top of the 4 GiB guest
    // address space (e.g. SP ≈ 0xffffee40). Use the recorded stack range
    // when available — otherwise fall back to "any non-zero, non-all-ones
    // address that's 4-byte aligned".
    let stack_range = env
        .threads
        .get(env.current_thread)
        .and_then(|t| t.stack.clone());

    let mut fp = env.cpu.regs()[FRAME_POINTER];
    let return_to_host = env.dyld.return_to_host_routine().addr_with_thumb_bit();
    let thread_exit = env.dyld.thread_exit_routine().addr_with_thumb_bit();

    for _ in 0..64 {
        if fp == 0 || fp == 0xffff_ffff || (fp & 3) != 0 {
            break;
        }
        if let Some(ref r) = stack_range {
            if !r.contains(&fp) {
                break;
            }
        }
        let prev_fp: u32 = env.mem.read(ConstPtr::<u32>::from_bits(fp));
        let lr: u32 = env.mem.read(ConstPtr::<u32>::from_bits(fp + 4));
        let lr_no_thumb = lr & !1;
        // Skip frames where LR is one of touchHLE's host trampoline
        // sentinels (return-to-host / thread-exit). Those mark the
        // boundary between host and guest code; unwinding past them
        // would dump us back into the wrong place.
        let is_host_trampoline = lr == return_to_host || lr == thread_exit;
        if !is_host_trampoline && lr_no_thumb > 0 && lr_no_thumb < APP_CODE_LIMIT {
            let regs = env.cpu.regs_mut();
            regs[FRAME_POINTER] = prev_fp;
            regs[Cpu::SP] = fp + 8;
            regs[0] = 0;
            env.cpu.branch(GuestFunction::from_addr_with_thumb_bit(lr));
            return true;
        }
        fp = prev_fp;
    }
    false
}

// === C++ Itanium ABI: exception machinery ===
//
// We allocate the requested storage prefixed by a fake __cxa_exception
// header, so that pointer arithmetic in the app's exception-handling
// code (ABI offsets, exception_class field, etc.) lands inside live
// memory. We never actually free the storage — exceptions are extremely
// rare and the leak is bounded.

const CXA_EXCEPTION_HEADER_SIZE: GuestUSize = 0x60;

fn __cxa_allocate_exception(env: &mut Environment, thrown_size: GuestUSize) -> MutVoidPtr {
    let block: MutVoidPtr = env.mem.alloc(CXA_EXCEPTION_HEADER_SIZE + thrown_size);
    Ptr::from_bits(block.to_bits() + CXA_EXCEPTION_HEADER_SIZE)
}

fn __cxa_free_exception(_env: &mut Environment, _thrown: MutVoidPtr) {
    // Leak — see comment above.
}

fn __cxa_throw(env: &mut Environment, _exc: MutVoidPtr, tinfo: ConstVoidPtr, _dtor: GuestFunction) {
    // Itanium type_info layout (32-bit):
    //   +0  vptr
    //   +4  const char *name
    let type_name = if !tinfo.is_null() {
        let name_field: ConstPtr<ConstPtr<u8>> = tinfo.cast();
        let name_ptr: ConstPtr<u8> = env.mem.read(name_field + 1);
        if !name_ptr.is_null() {
            env.mem
                .cstr_at_utf8(name_ptr)
                .unwrap_or("(non-utf8)")
                .to_owned()
        } else {
            "(unknown)".to_owned()
        }
    } else {
        "(null type_info)".to_owned()
    };

    // Throw-rate limiter: if the app enters an exception loop (e.g. because
    // our SjLj bypass returns it to a `while (true) new X;` path that throws
    // again the next iteration), we'll silently burn CPU forever. Track the
    // rate of throws of the same type and abort after a reasonable ceiling so
    // the emulator stays responsive.
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex;
    static LAST_THROW_TYPE: Mutex<String> = Mutex::new(String::new());
    static SAME_TYPE_COUNT: AtomicU32 = AtomicU32::new(0);
    const THROW_LOOP_LIMIT: u32 = 512;
    let count = {
        let mut last = LAST_THROW_TYPE.lock().unwrap();
        if *last == type_name {
            SAME_TYPE_COUNT.fetch_add(1, Ordering::Relaxed) + 1
        } else {
            *last = type_name.clone();
            SAME_TYPE_COUNT.store(1, Ordering::Relaxed);
            1
        }
    };

    if count <= 3 || count.is_multiple_of(64) {
        log!(
            "Guest threw a C++ exception of type {:?} (consecutive #{}): \
             touchHLE has no real unwinder, so we unwind to the nearest \
             app-level frame via the frame-pointer chain.",
            type_name,
            count
        );
    }
    if count >= THROW_LOOP_LIMIT {
        log!(
            "Warning: Exception loop detected: {} threw {} times in a row. \
             touchHLE's SjLj bypass is returning into a caller that re-throws \
             every iteration. Returning to caller to break the loop; the \
             guest will likely abort on its own shortly.",
            type_name,
            count
        );
        return;
    }

    if !unwind_to_app_frame(env) {
        log!(
            "Warning: Could not unwind past C++ exception ({}); no app-level \
             frame on the stack. Returning to caller; guest will likely abort.",
            type_name
        );
    }
}

fn __cxa_rethrow(env: &mut Environment) {
    log!("__cxa_rethrow — bypassing");
    if !unwind_to_app_frame(env) {
        log!(
            "Warning: Could not unwind past __cxa_rethrow; no app-level frame. \
             Returning to caller; guest will likely abort."
        );
    }
}

fn __cxa_begin_catch(_env: &mut Environment, exception_obj: MutVoidPtr) -> MutVoidPtr {
    // Return the exception object unchanged. Combined with the unwind
    // bypass above, no frame ever actually reaches __cxa_begin_catch
    // unless someone called it manually; in that case keep the value
    // sane.
    exception_obj
}

fn __cxa_end_catch(_env: &mut Environment) {}

fn __cxa_pure_virtual(env: &mut Environment) {
    log!("Pure virtual function called — vtable slot was NULL. Bypassing.");
    if !unwind_to_app_frame(env) {
        log!(
            "Warning: Pure virtual function called and no recoverable frame; \
             returning to caller. Guest will likely abort."
        );
    }
}

/// `__cxa_uncaught_exception` (Itanium C++ ABI; also re-exported by
/// libc++abi). Returns whether an exception is currently in flight
/// ("thrown but not yet caught"). touchHLE's exception bypass never
/// leaves an exception in flight after `__cxa_throw` returns control to
/// an app frame, so the truthful answer is `false`. libstdc++/libc++
/// use this in `std::uncaught_exception()` and in stream/destructor
/// guards.
fn __cxa_uncaught_exception(_env: &mut Environment) -> bool {
    false
}

fn __cxa_call_unexpected(env: &mut Environment, _exc: MutVoidPtr) {
    log!("__cxa_call_unexpected — bypassing");
    if !unwind_to_app_frame(env) {
        log!(
            "Warning: __cxa_call_unexpected with no recoverable frame; \
             returning to caller. Guest will likely abort."
        );
    }
}

/// Itanium ABI `__dynamic_cast`:
///
/// ```c
/// void *__dynamic_cast(const void *src,
///                      const __class_type_info *src_type,
///                      const __class_type_info *dst_type,
///                      ptrdiff_t src2dst_offset);
/// ```
///
/// Returns the casted pointer on success, or NULL on failure (the cast
/// does not apply / a `dynamic_cast<T*>` should evaluate to nullptr).
///
/// touchHLE has no real RTTI walk because every Itanium type_info vtable
/// is stubbed (see [crate::dyld::do_non_lazy_linking]). We can't ever
/// say "yes this is the right cast", so always returning NULL is the
/// only safe answer — it matches the language semantics for failed
/// casts. Apps that rely on dynamic_cast to *succeed* (rather than just
/// using it as a defensive nullptr check) will still misbehave, but
/// they were already going to crash on the broken vtables anyway.
fn __dynamic_cast(
    _env: &mut Environment,
    _src: ConstVoidPtr,
    _src_type: ConstVoidPtr,
    _dst_type: ConstVoidPtr,
    _src2dst_offset: i32,
) -> ConstVoidPtr {
    log_dbg!("__dynamic_cast: returning NULL (RTTI vtables are stubbed)");
    Ptr::null()
}

// === SjLj unwinder entry points ===
//
// `_Unwind_SjLj_Register/Unregister` push and pop a jmpbuf onto the
// thread-local SjLj exception chain. We never actually consult the
// chain (because __cxa_throw doesn't walk it), so register/unregister
// are no-ops. `_Unwind_SjLj_RaiseException` and `_Unwind_SjLj_Resume`
// fall through to the same frame-pointer bypass as __cxa_throw.

#[allow(non_snake_case)]
fn _Unwind_SjLj_Register(_env: &mut Environment, _jmpbuf: MutVoidPtr) {}

#[allow(non_snake_case)]
fn _Unwind_SjLj_Unregister(_env: &mut Environment, _jmpbuf: MutVoidPtr) {}

#[allow(non_snake_case)]
fn _Unwind_SjLj_RaiseException(env: &mut Environment, _exc: MutVoidPtr) -> i32 {
    log!("_Unwind_SjLj_RaiseException — bypassing");
    if !unwind_to_app_frame(env) {
        log!(
            "Warning: _Unwind_SjLj_RaiseException with no recoverable frame; \
             returning _URC_FATAL_PHASE1_ERROR to caller."
        );
        // _URC_FATAL_PHASE1_ERROR
        return 3;
    }
    0
}

#[allow(non_snake_case)]
fn _Unwind_SjLj_Resume(env: &mut Environment, _exc: MutVoidPtr) {
    log!("_Unwind_SjLj_Resume — bypassing");
    if !unwind_to_app_frame(env) {
        log!(
            "Warning: _Unwind_SjLj_Resume with no recoverable frame; returning \
             to caller. Guest will likely abort."
        );
    }
}

#[allow(non_snake_case)]
fn _Unwind_SjLj_Resume_or_Rethrow(env: &mut Environment, _exc: MutVoidPtr) -> i32 {
    log!("_Unwind_SjLj_Resume_or_Rethrow — bypassing");
    if !unwind_to_app_frame(env) {
        log!(
            "Warning: _Unwind_SjLj_Resume_or_Rethrow with no recoverable frame; \
             returning _URC_FATAL_PHASE2_ERROR."
        );
        // _URC_FATAL_PHASE2_ERROR
        return 2;
    }
    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(__cxa_atexit(_, _, _)),
    export_c_func!(__cxa_finalize(_)),
    export_c_func!(__cxa_guard_acquire(_)),
    export_c_func!(__cxa_guard_release(_)),
    export_c_func!(__cxa_guard_abort(_)),
    export_c_func!(__cxa_allocate_exception(_)),
    export_c_func!(__cxa_free_exception(_)),
    export_c_func!(__cxa_throw(_, _, _)),
    export_c_func!(__cxa_rethrow()),
    export_c_func!(__cxa_begin_catch(_)),
    export_c_func!(__cxa_end_catch()),
    export_c_func!(__cxa_pure_virtual()),
    export_c_func!(__cxa_call_unexpected(_)),
    export_c_func!(__cxa_uncaught_exception()),
    export_c_func!(__dynamic_cast(_, _, _, _)),
    export_c_func!(_Unwind_SjLj_Register(_)),
    export_c_func!(_Unwind_SjLj_Unregister(_)),
    export_c_func!(_Unwind_SjLj_RaiseException(_)),
    export_c_func!(_Unwind_SjLj_Resume(_)),
    export_c_func!(_Unwind_SjLj_Resume_or_Rethrow(_)),
];
