/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Grand Central Dispatch (GCD) — `dispatch/dispatch.h`
//!
//! touchHLE is single-threaded from the guest's perspective, so most
//! concurrency primitives can be dramatically simplified:
//!   - `dispatch_once` just calls the block once and marks the token.
//!   - `dispatch_async` / `dispatch_sync` call the block immediately.
//!   - Queues are opaque handles — we don't actually schedule work.

use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::mem::{ConstVoidPtr, MutPtr, MutVoidPtr, Ptr};
use crate::Environment;
use std::collections::HashMap;

// MARK: - Types

/// `dispatch_once_t` is a `long` (i32 on 32-bit) used as a flag.
#[allow(non_camel_case_types)]
pub type dispatch_once_t = i32;

/// Opaque queue handle — we use a small integer sentinel.
#[allow(non_camel_case_types)]
pub type dispatch_queue_t = MutVoidPtr;

/// Opaque group handle.
#[allow(non_camel_case_types)]
pub type dispatch_group_t = MutVoidPtr;

/// Opaque semaphore handle.
#[allow(non_camel_case_types)]
pub type dispatch_semaphore_t = MutVoidPtr;

/// Opaque source handle.
#[allow(non_camel_case_types)]
pub type dispatch_source_t = MutVoidPtr;

/// Opaque data handle.
#[allow(non_camel_case_types)]
pub type dispatch_data_t = MutVoidPtr;

/// Block context passed to dispatch functions: `void (^block)(void)`.
#[allow(non_camel_case_types)]
pub type dispatch_block_t = MutVoidPtr;

/// Priority levels for `dispatch_get_global_queue`.
const DISPATCH_QUEUE_PRIORITY_HIGH: i32 = 2;
const DISPATCH_QUEUE_PRIORITY_DEFAULT: i32 = 0;
const DISPATCH_QUEUE_PRIORITY_LOW: i32 = -2;
const DISPATCH_QUEUE_PRIORITY_BACKGROUND: i32 = i32::MIN;

/// Queue attribute constants.
const DISPATCH_QUEUE_SERIAL: i32 = 0;
const DISPATCH_QUEUE_CONCURRENT: i32 = 1;

/// `dispatch_time_t` constants.
const DISPATCH_TIME_NOW: u64 = 0;
const DISPATCH_TIME_FOREVER: u64 = u64::MAX;

/// Sentinel pointer values we hand out as "queue" handles.
/// We use small odd numbers so they're never confused with real allocations.
const MAIN_QUEUE_PTR: u32 = 0x0000_0001;
const GLOBAL_QUEUE_PTR: u32 = 0x0000_0003;

// MARK: - Notification name constants

pub const DISPATCH_APPLY_AUTO: &str = "DISPATCH_APPLY_AUTO";

/// Apple's libdispatch exports `_dispatch_main_q` as a global symbol whose
/// address is the main queue's handle (the SDK header
/// `dispatch/queue.h` defines the macro
/// `dispatch_get_main_queue() (&_dispatch_main_q)`). Some compilers/linkers
/// emit a non-lazy binding to `_dispatch_main_q` directly instead of going
/// through `dispatch_get_main_queue()`.
///
/// On touchHLE the main queue is represented by the sentinel value
/// [MAIN_QUEUE_PTR] (`0x1`). Resolving the constant to that sentinel makes
/// `dispatch_async(_dispatch_main_q, …)` and similar binary-direct uses go
/// through the same code path as `dispatch_get_main_queue()`.
pub const CONSTANTS: ConstantExports = &[(
    "__dispatch_main_q",
    HostConstant::Custom(|_env| -> ConstVoidPtr {
        let p: MutVoidPtr = Ptr::from_bits(MAIN_QUEUE_PTR);
        p.cast_const()
    }),
)];

// MARK: - State

#[derive(Default)]
pub struct State {
    /// Set of `dispatch_once_t` guest addresses that have already fired.
    once_tokens: std::collections::HashSet<u32>,
    /// Semaphore values keyed by guest pointer bits.
    semaphores: HashMap<u32, i64>,
    /// Named queues keyed by label.
    named_queues: HashMap<String, u32>,
    /// Next queue handle to hand out.
    next_queue_handle: u32,
    /// Queue-specific values set via `dispatch_queue_set_specific`, keyed by
    /// `(queue handle bits, key pointer bits)` and storing the context
    /// pointer bits. See
    /// <https://developer.apple.com/documentation/dispatch/1452828-dispatch_queue_set_specific>.
    queue_specifics: HashMap<(u32, u32), u32>,
    /// Stack of queue handles currently executing a block, so that
    /// `dispatch_get_specific` can resolve against the running queue. The
    /// main queue is assumed when the stack is empty.
    current_queue_stack: Vec<u32>,
}

fn get_state(env: &mut Environment) -> &mut State {
    &mut env.libc_state.dispatch
}

// MARK: - dispatch_once

/// `dispatch_once(dispatch_once_t *predicate, dispatch_block_t block)`
///
/// Calls `block` exactly once for a given `predicate` address.
/// Since we're single-threaded this is straightforward.
fn dispatch_once(
    env: &mut Environment,
    predicate: MutPtr<dispatch_once_t>,
    block: dispatch_block_t,
) {
    let token_addr = predicate.to_bits();
    if env.libc_state.dispatch.once_tokens.contains(&token_addr) {
        return;
    }
    env.libc_state.dispatch.once_tokens.insert(token_addr);
    // Mark the predicate so native code that reads it directly sees "done".
    env.mem.write(predicate, -1i32); // ~0 == done on Apple platforms

    if !block.is_null() {
        call_void_block(env, block);
    }
}

/// `dispatch_once_f(dispatch_once_t *predicate, void *context, dispatch_function_t work)`
///
/// The function-pointer variant of `dispatch_once`. Apple documents it
/// in `<dispatch/once.h>` and uses it heavily inside the libdispatch
/// "C interface" thunks emitted by clang for non-block once-init code.
/// Semantics match `dispatch_once`: the `work` function is invoked at
/// most once for each `predicate` address, with `context` passed as its
/// single argument. <https://developer.apple.com/documentation/dispatch/1452833-dispatch_once_f>
fn dispatch_once_f(
    env: &mut Environment,
    predicate: MutPtr<dispatch_once_t>,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    let token_addr = predicate.to_bits();
    if env.libc_state.dispatch.once_tokens.contains(&token_addr) {
        return;
    }
    env.libc_state.dispatch.once_tokens.insert(token_addr);
    env.mem.write(predicate, -1i32); // ~0 == done on Apple platforms

    if work.addr_with_thumb_bit() != 0 {
        let _: () = work.call_from_host(env, (context,));
    }
}

// MARK: - Queue creation / retrieval

fn dispatch_get_main_queue(_env: &mut Environment) -> dispatch_queue_t {
    MutVoidPtr::from_bits(MAIN_QUEUE_PTR)
}

fn dispatch_get_global_queue(
    _env: &mut Environment,
    identifier: i32,
    _flags: u32,
) -> dispatch_queue_t {
    log_dbg!(
        "dispatch_get_global_queue(identifier={}) -> global queue stub",
        identifier
    );
    MutVoidPtr::from_bits(GLOBAL_QUEUE_PTR)
}

fn dispatch_queue_create(
    env: &mut Environment,
    label: ConstVoidPtr, // const char*
    attr: i32,           // DISPATCH_QUEUE_SERIAL / DISPATCH_QUEUE_CONCURRENT
) -> dispatch_queue_t {
    let label_str = if label.is_null() {
        "<anonymous>".to_string()
    } else {
        env.mem
            .cstr_at_utf8(label.cast())
            .unwrap_or_default()
            .to_owned()
    };
    log_dbg!(
        "dispatch_queue_create({:?}, attr={}) -> stub",
        label_str,
        attr
    );
    let st = get_state(env);
    st.next_queue_handle += 2; // keep odd
    let handle = st.next_queue_handle | 0x0100_0000;
    st.named_queues.insert(label_str, handle);
    MutVoidPtr::from_bits(handle)
}

fn dispatch_queue_attr_make_with_qos_class(
    _env: &mut Environment,
    attr: dispatch_queue_t,
    _qos_class: u32,
    _relative_priority: i32,
) -> dispatch_queue_t {
    // Apple docs (<dispatch/queue.h>):
    //   "Returns an attribute value which may be provided to
    //   dispatch_queue_create to hint the system to use the given QoS
    //   class. Since touchHLE doesn't model thread QoS — it runs
    //   everything on the main thread queue — the documented behaviour
    //   we can faithfully implement is to return the input attr
    //   unchanged: any subsequent dispatch_queue_create(label,
    //   returned_attr) then behaves identically to using the original
    //   attr (which is the only behaviour touchHLE supports today).
    //   Edge case per Apple: if qos_class == QOS_CLASS_UNSPECIFIED (0)
    //   AND attr == NULL, returns NULL; same null-pass-through
    //   behaviour falls out naturally from returning attr."
    attr
}

fn dispatch_queue_get_label(env: &mut Environment, _queue: dispatch_queue_t) -> ConstVoidPtr {
    // Return an empty string — label lookup not implemented.
    let empty = env.mem.alloc_and_write_cstr(b"");
    empty.cast_void().cast_const()
}

fn dispatch_get_current_queue(_env: &mut Environment) -> dispatch_queue_t {
    // We always run on the "main" queue.
    MutVoidPtr::from_bits(MAIN_QUEUE_PTR)
}

fn dispatch_queue_retain(_env: &mut Environment, queue: dispatch_queue_t) -> dispatch_queue_t {
    queue // no-op
}

fn dispatch_queue_release(_env: &mut Environment, _queue: dispatch_queue_t) {
    // no-op
}

fn dispatch_retain(_env: &mut Environment, _obj: MutVoidPtr) {
    // no-op for all dispatch objects
}

fn dispatch_release(_env: &mut Environment, _obj: MutVoidPtr) {
    // no-op
}

// MARK: - Dispatch async / sync

fn dispatch_async(env: &mut Environment, queue: dispatch_queue_t, block: dispatch_block_t) {
    log_dbg!("dispatch_async: calling block immediately (single-threaded)");
    if !block.is_null() {
        get_state(env).current_queue_stack.push(queue.to_bits());
        call_void_block(env, block);
        get_state(env).current_queue_stack.pop();
    }
}

fn dispatch_sync(env: &mut Environment, queue: dispatch_queue_t, block: dispatch_block_t) {
    log_dbg!("dispatch_sync: calling block immediately");
    if !block.is_null() {
        get_state(env).current_queue_stack.push(queue.to_bits());
        call_void_block(env, block);
        get_state(env).current_queue_stack.pop();
    }
}

fn dispatch_async_f(
    env: &mut Environment,
    _queue: dispatch_queue_t,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    if work != GuestFunction::null_ptr() {
        let _: () = work.call_from_host(env, (context,));
    }
}

fn dispatch_sync_f(
    env: &mut Environment,
    _queue: dispatch_queue_t,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    if work != GuestFunction::null_ptr() {
        let _: () = work.call_from_host(env, (context,));
    }
}

// MARK: - dispatch_apply

fn dispatch_apply(
    env: &mut Environment,
    iterations: u32,
    _queue: dispatch_queue_t,
    block: MutVoidPtr,
) {
    if block.is_null() {
        return;
    }
    let invoke_ptr = env.mem.read(block.cast::<u32>() + 3u32);
    if invoke_ptr == 0 {
        return;
    }
    let invoke = GuestFunction::from_addr_with_thumb_bit(invoke_ptr);
    for i in 0..iterations {
        let _: () = invoke.call_from_host(env, (block, i));
    }
}

fn dispatch_apply_f(
    env: &mut Environment,
    iterations: u32,
    _queue: dispatch_queue_t,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    if work == GuestFunction::null_ptr() {
        return;
    }
    for i in 0..iterations {
        let _: () = work.call_from_host(env, (context, i));
    }
}

// MARK: - dispatch_after

fn dispatch_after(
    env: &mut Environment,
    _when: u64, // dispatch_time_t
    _queue: dispatch_queue_t,
    block: dispatch_block_t,
) {
    log_dbg!("dispatch_after: calling block immediately (timer not supported)");
    if !block.is_null() {
        call_void_block(env, block);
    }
}

fn dispatch_after_f(
    env: &mut Environment,
    _when: u64,
    _queue: dispatch_queue_t,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    if work != GuestFunction::null_ptr() {
        let _: () = work.call_from_host(env, (context,));
    }
}

// MARK: - dispatch_time

fn dispatch_time(_env: &mut Environment, when: u64, delta: i64) -> u64 {
    if when == DISPATCH_TIME_FOREVER {
        return DISPATCH_TIME_FOREVER;
    }
    when.saturating_add(delta as u64)
}

fn dispatch_walltime(_env: &mut Environment, _spec: ConstVoidPtr, delta: i64) -> u64 {
    // Return a time `delta` nanoseconds from now (approximate).
    delta.max(0) as u64
}

// MARK: - dispatch_group

fn dispatch_group_create(_env: &mut Environment) -> dispatch_group_t {
    // Return a small non-null sentinel.
    MutVoidPtr::from_bits(0x0200_0001)
}

fn dispatch_group_async(
    env: &mut Environment,
    _group: dispatch_group_t,
    _queue: dispatch_queue_t,
    block: dispatch_block_t,
) {
    if !block.is_null() {
        call_void_block(env, block);
    }
}

fn dispatch_group_async_f(
    env: &mut Environment,
    _group: dispatch_group_t,
    _queue: dispatch_queue_t,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    if work != GuestFunction::null_ptr() {
        let _: () = work.call_from_host(env, (context,));
    }
}

fn dispatch_group_enter(_env: &mut Environment, _group: dispatch_group_t) {}
fn dispatch_group_leave(_env: &mut Environment, _group: dispatch_group_t) {}

fn dispatch_group_wait(_env: &mut Environment, _group: dispatch_group_t, _timeout: u64) -> i32 {
    0 // success — all work completed immediately
}

fn dispatch_group_notify(
    env: &mut Environment,
    _group: dispatch_group_t,
    _queue: dispatch_queue_t,
    block: dispatch_block_t,
) {
    if !block.is_null() {
        call_void_block(env, block);
    }
}

fn dispatch_group_notify_f(
    env: &mut Environment,
    _group: dispatch_group_t,
    _queue: dispatch_queue_t,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    if work != GuestFunction::null_ptr() {
        let _: () = work.call_from_host(env, (context,));
    }
}

// MARK: - dispatch_semaphore

fn dispatch_semaphore_create(env: &mut Environment, value: i64) -> dispatch_semaphore_t {
    let handle = {
        let st = get_state(env);
        st.next_queue_handle += 2;
        let h = st.next_queue_handle | 0x0300_0000;
        st.semaphores.insert(h, value);
        h
    };
    MutVoidPtr::from_bits(handle)
}

fn dispatch_semaphore_wait(env: &mut Environment, sem: dispatch_semaphore_t, _timeout: u64) -> i32 {
    let key = sem.to_bits();
    let st = get_state(env);
    if let Some(val) = st.semaphores.get_mut(&key) {
        *val -= 1;
        0 // success
    } else {
        log!("dispatch_semaphore_wait: unknown semaphore {:?}", sem);
        -1
    }
}

fn dispatch_semaphore_signal(env: &mut Environment, sem: dispatch_semaphore_t) -> i32 {
    let key = sem.to_bits();
    let st = get_state(env);
    if let Some(val) = st.semaphores.get_mut(&key) {
        *val += 1;
        0
    } else {
        log!("dispatch_semaphore_signal: unknown semaphore {:?}", sem);
        0
    }
}

// MARK: - dispatch_source (stub)

fn dispatch_source_create(
    _env: &mut Environment,
    _type_: ConstVoidPtr,
    _handle: usize,
    _mask: usize,
    _queue: dispatch_queue_t,
) -> dispatch_source_t {
    log!("dispatch_source_create: stubbed");
    MutVoidPtr::from_bits(0x0400_0001)
}

fn dispatch_source_set_event_handler(
    _env: &mut Environment,
    _source: dispatch_source_t,
    _handler: dispatch_block_t,
) {
    log_dbg!("dispatch_source_set_event_handler: stubbed");
}

fn dispatch_source_set_event_handler_f(
    _env: &mut Environment,
    _source: dispatch_source_t,
    _handler: GuestFunction,
) {
}

fn dispatch_source_set_cancel_handler(
    _env: &mut Environment,
    _source: dispatch_source_t,
    _handler: dispatch_block_t,
) {
}

fn dispatch_source_set_cancel_handler_f(
    _env: &mut Environment,
    _source: dispatch_source_t,
    _handler: GuestFunction,
) {
}

fn dispatch_source_cancel(_env: &mut Environment, _source: dispatch_source_t) {}

fn dispatch_source_testcancel(_env: &mut Environment, _source: dispatch_source_t) -> i32 {
    0
}

fn dispatch_source_get_handle(_env: &mut Environment, _source: dispatch_source_t) -> usize {
    0
}

fn dispatch_source_get_mask(_env: &mut Environment, _source: dispatch_source_t) -> usize {
    0
}

fn dispatch_source_get_data(_env: &mut Environment, _source: dispatch_source_t) -> usize {
    0
}

fn dispatch_source_merge_data(_env: &mut Environment, _source: dispatch_source_t, _value: usize) {}

fn dispatch_source_set_timer(
    _env: &mut Environment,
    _source: dispatch_source_t,
    _start: u64,
    _interval: u64,
    _leeway: u64,
) {
}

fn dispatch_resume(_env: &mut Environment, _object: MutVoidPtr) {}
fn dispatch_suspend(_env: &mut Environment, _object: MutVoidPtr) {}

// MARK: - dispatch_barrier

fn dispatch_barrier_async(env: &mut Environment, queue: dispatch_queue_t, block: dispatch_block_t) {
    dispatch_async(env, queue, block);
}

fn dispatch_barrier_sync(env: &mut Environment, queue: dispatch_queue_t, block: dispatch_block_t) {
    dispatch_sync(env, queue, block);
}

fn dispatch_barrier_async_f(
    env: &mut Environment,
    queue: dispatch_queue_t,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    dispatch_async_f(env, queue, context, work);
}

fn dispatch_barrier_sync_f(
    env: &mut Environment,
    queue: dispatch_queue_t,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    dispatch_sync_f(env, queue, context, work);
}

// MARK: - dispatch_set_context / dispatch_get_context

fn dispatch_set_context(_env: &mut Environment, _object: MutVoidPtr, _context: MutVoidPtr) {}

fn dispatch_get_context(_env: &mut Environment, _object: MutVoidPtr) -> MutVoidPtr {
    MutVoidPtr::null()
}

/// `void dispatch_queue_set_specific(dispatch_queue_t queue, const void *key,
///                                   void *context,
///                                   dispatch_function_t destructor)`
///
/// Associates `context` with `queue` under the unique pointer `key`. Passing
/// a NULL `context` clears any existing association (Apple documents NULL as
/// "remove the value"). The `destructor` is the cleanup callback GCD would run
/// when the value is replaced or the queue is destroyed; touchHLE does not own
/// the guest's context allocation, so it is intentionally not invoked.
/// <https://developer.apple.com/documentation/dispatch/1452828-dispatch_queue_set_specific>
fn dispatch_queue_set_specific(
    env: &mut Environment,
    queue: dispatch_queue_t,
    key: ConstVoidPtr,
    context: MutVoidPtr,
    _destructor: GuestFunction,
) {
    let qk = (queue.to_bits(), key.to_bits());
    let state = get_state(env);
    if context.is_null() {
        state.queue_specifics.remove(&qk);
    } else {
        state.queue_specifics.insert(qk, context.to_bits());
    }
}

/// `void *dispatch_queue_get_specific(dispatch_queue_t queue, const void *key)`
///
/// Returns the value previously associated with `key` on `queue`, or NULL.
/// <https://developer.apple.com/documentation/dispatch/1453028-dispatch_queue_get_specific>
fn dispatch_queue_get_specific(
    env: &mut Environment,
    queue: dispatch_queue_t,
    key: ConstVoidPtr,
) -> MutVoidPtr {
    let qk = (queue.to_bits(), key.to_bits());
    match get_state(env).queue_specifics.get(&qk) {
        Some(&bits) => MutVoidPtr::from_bits(bits),
        None => MutVoidPtr::null(),
    }
}

/// `void *dispatch_get_specific(const void *key)`
///
/// Returns the value associated with `key` on the queue currently executing
/// the caller. We track the running queue via `current_queue_stack`
/// (pushed/popped around `dispatch_sync`/`dispatch_async` block execution);
/// when nothing is running we resolve against the main queue, matching the
/// behaviour callers rely on for "am I on queue X?" re-entrancy checks.
/// <https://developer.apple.com/documentation/dispatch/1453099-dispatch_get_specific>
fn dispatch_get_specific(env: &mut Environment, key: ConstVoidPtr) -> MutVoidPtr {
    let state = get_state(env);
    let current = state
        .current_queue_stack
        .last()
        .copied()
        .unwrap_or(MAIN_QUEUE_PTR);
    match state.queue_specifics.get(&(current, key.to_bits())) {
        Some(&bits) => MutVoidPtr::from_bits(bits),
        None => MutVoidPtr::null(),
    }
}

fn dispatch_set_finalizer_f(
    _env: &mut Environment,
    _object: MutVoidPtr,
    _finalizer: GuestFunction,
) {
}

fn dispatch_set_target_queue(
    _env: &mut Environment,
    _object: MutVoidPtr,
    _queue: dispatch_queue_t,
) {
}

// MARK: - dispatch_main

fn dispatch_main(_env: &mut Environment) {
    log!("dispatch_main: stubbed (returning immediately)");
    // Cannot spin forever here since GuestRet requires (). Just return.
}

/// `void dispatch_debug(dispatch_object_t object, const char *message, ...)`
///
/// Per Apple docs: "Programmatically logs debug information about a dispatch
/// object." This is a debugging aid that apps rarely call in production,
/// but some Unity-based games (Frontline Commando 2) reference it. Since
/// we don't have real dispatch object internals to inspect, we simply no-op.
fn dispatch_debug(_env: &mut Environment, _object: MutVoidPtr, _message: ConstVoidPtr) {
    // No-op. The message is a printf-style format string with varargs that
    // we intentionally ignore.
}

// MARK: - Helpers

/// Invoke a GCD block (`void (^)(void)`).
/// On ARM32 a block is a struct whose second word is the invoke function
//pointer.
/// Layout: [isa, flags, reserved, invoke, descriptor, captures...]
fn call_void_block(env: &mut Environment, block: dispatch_block_t) {
    let invoke_ptr = env.mem.read(block.cast::<u32>() + 3u32);
    if invoke_ptr == 0 {
        return;
    }
    let invoke = GuestFunction::from_addr_with_thumb_bit(invoke_ptr);
    let _: () = invoke.call_from_host(env, (block,));
}

pub const FUNCTIONS: FunctionExports = &[
    // once
    export_c_func!(dispatch_once(_, _)),
    export_c_func!(dispatch_once_f(_, _, _)),
    // queues
    export_c_func!(dispatch_get_main_queue()),
    export_c_func!(dispatch_get_global_queue(_, _)),
    export_c_func!(dispatch_get_current_queue()),
    export_c_func!(dispatch_queue_create(_, _)),
    export_c_func!(dispatch_queue_attr_make_with_qos_class(_, _, _)),
    export_c_func!(dispatch_queue_get_label(_)),
    export_c_func!(dispatch_queue_retain(_)),
    export_c_func!(dispatch_queue_release(_)),
    export_c_func!(dispatch_retain(_)),
    export_c_func!(dispatch_release(_)),
    // async / sync
    export_c_func!(dispatch_async(_, _)),
    export_c_func!(dispatch_sync(_, _)),
    export_c_func!(dispatch_async_f(_, _, _)),
    export_c_func!(dispatch_sync_f(_, _, _)),
    // apply
    export_c_func!(dispatch_apply(_, _, _)),
    export_c_func!(dispatch_apply_f(_, _, _, _)),
    // after
    export_c_func!(dispatch_after(_, _, _)),
    export_c_func!(dispatch_after_f(_, _, _, _)),
    // time
    export_c_func!(dispatch_time(_, _)),
    export_c_func!(dispatch_walltime(_, _)),
    // group
    export_c_func!(dispatch_group_create()),
    export_c_func!(dispatch_group_async(_, _, _)),
    export_c_func!(dispatch_group_async_f(_, _, _, _)),
    export_c_func!(dispatch_group_enter(_)),
    export_c_func!(dispatch_group_leave(_)),
    export_c_func!(dispatch_group_wait(_, _)),
    export_c_func!(dispatch_group_notify(_, _, _)),
    export_c_func!(dispatch_group_notify_f(_, _, _, _)),
    // semaphore
    export_c_func!(dispatch_semaphore_create(_)),
    export_c_func!(dispatch_semaphore_wait(_, _)),
    export_c_func!(dispatch_semaphore_signal(_)),
    // source
    export_c_func!(dispatch_source_create(_, _, _, _)),
    export_c_func!(dispatch_source_set_event_handler(_, _)),
    export_c_func!(dispatch_source_set_event_handler_f(_, _)),
    export_c_func!(dispatch_source_set_cancel_handler(_, _)),
    export_c_func!(dispatch_source_set_cancel_handler_f(_, _)),
    export_c_func!(dispatch_source_cancel(_)),
    export_c_func!(dispatch_source_testcancel(_)),
    export_c_func!(dispatch_source_get_handle(_)),
    export_c_func!(dispatch_source_get_mask(_)),
    export_c_func!(dispatch_source_get_data(_)),
    export_c_func!(dispatch_source_merge_data(_, _)),
    export_c_func!(dispatch_source_set_timer(_, _, _, _)),
    // resume / suspend
    export_c_func!(dispatch_resume(_)),
    export_c_func!(dispatch_suspend(_)),
    // barrier
    export_c_func!(dispatch_barrier_async(_, _)),
    export_c_func!(dispatch_barrier_sync(_, _)),
    export_c_func!(dispatch_barrier_async_f(_, _, _)),
    export_c_func!(dispatch_barrier_sync_f(_, _, _)),
    // context
    export_c_func!(dispatch_set_context(_, _)),
    export_c_func!(dispatch_get_context(_)),
    export_c_func!(dispatch_queue_set_specific(_, _, _, _)),
    export_c_func!(dispatch_queue_get_specific(_, _)),
    export_c_func!(dispatch_get_specific(_)),
    export_c_func!(dispatch_set_finalizer_f(_, _)),
    export_c_func!(dispatch_set_target_queue(_, _)),
    // main
    export_c_func!(dispatch_main()),
    // debug
    export_c_func!(dispatch_debug(_, _)),
];
