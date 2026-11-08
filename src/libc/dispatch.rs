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
use crate::dyld::{
    export_c_func, export_c_func_aliased, ConstantExports, FunctionExports, HostConstant,
};
use crate::frameworks::foundation::ns_dictionary::dict_from_keys_and_objects;
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::libc::semaphore::{host_create_semaphore, host_destroy_semaphore, sem_t};
use crate::mem::{ConstVoidPtr, MutPtr, MutVoidPtr, Ptr};
use crate::objc::{id, msg, msg_class, objc_classes, Class, ClassExports};
use crate::Environment;
use std::collections::HashMap;

use super::dispatch_source::{
    self, dispatch_resume, dispatch_source_cancel, dispatch_source_create,
    dispatch_source_get_data, dispatch_source_get_handle, dispatch_source_get_mask,
    dispatch_source_merge_data, dispatch_source_set_cancel_handler,
    dispatch_source_set_cancel_handler_f, dispatch_source_set_event_handler,
    dispatch_source_set_event_handler_f, dispatch_source_set_timer, dispatch_source_testcancel,
    dispatch_suspend,
};

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
#[allow(non_camel_case_types, dead_code)]
pub type dispatch_data_t = MutVoidPtr;

/// Block context passed to dispatch functions: `void (^block)(void)`.
#[allow(non_camel_case_types)]
pub type dispatch_block_t = MutVoidPtr;

/// Priority levels for `dispatch_get_global_queue`.
#[allow(dead_code)]
const DISPATCH_QUEUE_PRIORITY_HIGH: i32 = 2;
#[allow(dead_code)]
const DISPATCH_QUEUE_PRIORITY_DEFAULT: i32 = 0;
#[allow(dead_code)]
const DISPATCH_QUEUE_PRIORITY_LOW: i32 = -2;
#[allow(dead_code)]
const DISPATCH_QUEUE_PRIORITY_BACKGROUND: i32 = i32::MIN;

/// Queue attribute constants.
#[allow(dead_code)]
const DISPATCH_QUEUE_SERIAL: i32 = 0;
#[allow(dead_code)]
const DISPATCH_QUEUE_CONCURRENT: i32 = 1;

/// `dispatch_time_t` constants.
const DISPATCH_TIME_NOW: u64 = 0;
pub(super) const DISPATCH_TIME_FOREVER: u64 = u64::MAX;

/// Sentinel pointer values we hand out as "queue" handles.
/// We use small odd numbers so they're never confused with real allocations.
pub(crate) const MAIN_QUEUE_PTR: u32 = 0x0000_0001;
const GLOBAL_QUEUE_PTR: u32 = 0x0000_0003;

// MARK: - Notification name constants

#[allow(dead_code)]
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
    /// Backing POSIX semaphore for each dispatch semaphore handle. Dispatch
    /// semaphores must really block, so they are backed by the same host
    /// semaphore objects as `sem_t` rather than a bare counter.
    semaphores: HashMap<u32, MutPtr<sem_t>>,
    /// Named queues keyed by label.
    named_queues: HashMap<String, u32>,
    /// Cached guest C-string labels keyed by queue handle (`0` unused).
    queue_labels: HashMap<u32, crate::mem::ConstPtr<u8>>,
    /// Cached empty guest C-string for unknown queue handles.
    empty_label: Option<crate::mem::ConstPtr<u8>>,
    /// Next queue handle to hand out.
    next_queue_handle: u32,
    /// Queue-specific values set via `dispatch_queue_set_specific`, keyed by
    /// `(queue handle bits, key pointer bits)` and storing the context
    /// pointer bits. See
    /// <https://developer.apple.com/documentation/dispatch/1452828-dispatch_queue_set_specific>.
    queue_specifics: HashMap<(u32, u32), u32>,
    /// Stack of queue handles currently executing a block, so that
    /// `dispatch_get_specific` can resolve against the running queue. When
    /// the stack is empty, see [current_queue].
    pub(super) current_queue_stack: Vec<u32>,
    /// Active dispatch sources keyed by their opaque guest handles.
    pub(super) sources: HashMap<u32, dispatch_source::DispatchSource>,
    /// Next aligned opaque source handle.
    pub(super) next_source_handle: u32,
}

pub(super) fn get_state(env: &mut Environment) -> &mut State {
    &mut env.libc_state.dispatch
}

/// The queue running the caller. Outside of a submitted block, Apple's
/// `dispatch_get_current_queue` "returns the main queue if the call is
/// executed from the main thread. If the call is made from any other thread,
/// this function returns the default concurrent queue."
/// <https://developer.apple.com/documentation/dispatch/dispatch_get_current_queue()>
/// <https://opensource.apple.com/source/libdispatch/libdispatch-84.5/man/dispatch_queue_create.3.auto.html>
///
/// Cut the Rope loads textures on an `NSThread` and only makes GL calls when
/// `dispatch_get_current_queue() == &_dispatch_main_q`, so answering "main
/// queue" there made it call GL on a thread with no EAGLContext.
fn current_queue(env: &mut Environment) -> u32 {
    let fallback = if env.current_thread == 0 {
        MAIN_QUEUE_PTR
    } else {
        GLOBAL_QUEUE_PTR
    };
    get_state(env)
        .current_queue_stack
        .last()
        .copied()
        .unwrap_or(fallback)
}

// MARK: - dispatch_once

/// `dispatch_once(dispatch_once_t *predicate, dispatch_block_t block)`
///
/// Calls `block` at most once per initialized `dispatch_once_t` (`0` -> `~0`).
/// Reading the guest `predicate` word directly matches Clang's inline fast path
/// (`*predicate == ~0l`) and allows heap/ivar `dispatch_once_t` slots to work
/// when freed memory is later reallocated at the same address.
fn dispatch_once(
    env: &mut Environment,
    predicate: MutPtr<dispatch_once_t>,
    block: dispatch_block_t,
) {
    if predicate.is_null() || env.mem.read(predicate) == -1 {
        return;
    }
    // Mark the predicate so native code that reads it directly sees "done".
    env.mem.write(predicate, -1i32); // ~0 == done on Apple platforms

    if !block.is_null() {
        call_void_block(env, block);
    }
}

/// `dispatch_once_f(dispatch_once_t *predicate, void *context, dispatch_function_t work)`
fn dispatch_once_f(
    env: &mut Environment,
    predicate: MutPtr<dispatch_once_t>,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    if predicate.is_null() || env.mem.read(predicate) == -1 {
        return;
    }
    env.mem.write(predicate, -1i32); // ~0 == done on Apple platforms

    if work.addr_with_thumb_bit() != 0 {
        let _: () = work.call_from_host(env, (context,));
    }
}
/// Swift's `swift_once` has the same once-token semantics as
/// `dispatch_once_f`: the Swift runtime passes a token, a context, and a
/// no-return-value callback. The ABI is documented in Swift's open-source
/// runtime (`stdlib/public/runtime/Once.cpp`).
fn swift_once(
    env: &mut Environment,
    predicate: MutPtr<dispatch_once_t>,
    function: GuestFunction,
    context: MutVoidPtr,
) {
    dispatch_once_f(env, predicate, context, function);
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
    let raw_label = if label.is_null() {
        String::new()
    } else {
        env.mem
            .cstr_at_utf8(label.cast())
            .unwrap_or_default()
            .to_owned()
    };
    log_dbg!(
        "dispatch_queue_create({:?}, attr={}) -> stub",
        if raw_label.is_empty() {
            "<anonymous>"
        } else {
            raw_label.as_str()
        },
        attr
    );
    let label_cstr = env.mem.alloc_and_write_cstr(raw_label.as_bytes()).cast_const();
    let st = get_state(env);
    st.next_queue_handle += 2; // keep odd
    let handle = st.next_queue_handle | 0x0100_0000;
    if !raw_label.is_empty() {
        st.named_queues.insert(raw_label, handle);
    }
    st.queue_labels.insert(handle, label_cstr);
    MutVoidPtr::from_bits(handle)
}

fn dispatch_queue_attr_make_with_qos_class(
    _env: &mut Environment,
    attr: dispatch_queue_t,
    _qos_class: u32,
    _relative_priority: i32,
) -> dispatch_queue_t {
    attr
}

fn dispatch_queue_get_label(env: &mut Environment, queue: dispatch_queue_t) -> ConstVoidPtr {
    // DISPATCH_CURRENT_QUEUE_LABEL is NULL (0): resolve to the running queue.
    let handle = if queue.is_null() {
        current_queue(env)
    } else {
        queue.to_bits()
    };
    if let Some(&ptr) = get_state(env).queue_labels.get(&handle) {
        return ptr.cast_void();
    }
    if handle == MAIN_QUEUE_PTR {
        let ptr = env
            .mem
            .alloc_and_write_cstr(b"com.apple.main-thread")
            .cast_const();
        get_state(env).queue_labels.insert(MAIN_QUEUE_PTR, ptr);
        return ptr.cast_void();
    }
    if handle == GLOBAL_QUEUE_PTR {
        let ptr = env
            .mem
            .alloc_and_write_cstr(b"com.apple.root.default-priority")
            .cast_const();
        get_state(env).queue_labels.insert(GLOBAL_QUEUE_PTR, ptr);
        return ptr.cast_void();
    }
    if let Some(empty) = get_state(env).empty_label {
        return empty.cast_void();
    }
    let empty = env.mem.alloc_and_write_cstr(b"").cast_const();
    get_state(env).empty_label = Some(empty);
    empty.cast_void()
}

fn dispatch_get_current_queue(env: &mut Environment) -> dispatch_queue_t {
    let current = current_queue(env);
    MutVoidPtr::from_bits(current)
}

fn dispatch_queue_retain(_env: &mut Environment, queue: dispatch_queue_t) -> dispatch_queue_t {
    queue // no-op
}

fn dispatch_queue_release(_env: &mut Environment, _queue: dispatch_queue_t) {
    // no-op
}

fn dispatch_retain(env: &mut Environment, obj: MutVoidPtr) {
    dispatch_source::retain_source(env, obj);
}

fn dispatch_release(env: &mut Environment, obj: MutVoidPtr) {
    if !release_if_semaphore(env, obj) {
        dispatch_source::release_source(env, obj);
    }
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
    queue: dispatch_queue_t,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    if work != GuestFunction::null_ptr() {
        get_state(env).current_queue_stack.push(queue.to_bits());
        let _: () = work.call_from_host(env, (context,));
        get_state(env).current_queue_stack.pop();
    }
}

fn dispatch_sync_f(
    env: &mut Environment,
    queue: dispatch_queue_t,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    if work != GuestFunction::null_ptr() {
        get_state(env).current_queue_stack.push(queue.to_bits());
        let _: () = work.call_from_host(env, (context,));
        get_state(env).current_queue_stack.pop();
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

/// Convert a `dispatch_time_t` into a delay in seconds, relative to now.
///
/// touchHLE's `dispatch_time(DISPATCH_TIME_NOW, delta)` and
/// `dispatch_walltime(_, delta)` both return the nanosecond `delta` directly
/// (because `DISPATCH_TIME_NOW` is 0), so a `when` produced by them is simply
/// "nanoseconds from now". We clamp to a sane range: never negative, and at
/// most 60s so a bogus/absolute value can't wedge a timer decades out.
pub(super) fn dispatch_after_delay_secs(when: u64) -> f64 {
    if when == DISPATCH_TIME_FOREVER {
        return f64::INFINITY;
    }
    let secs = (when as f64) / 1_000_000_000.0;
    secs.clamp(0.0, 86_400.0)
}

/// `dispatch_after(when, queue, block)`
///
/// Crucially this must be *asynchronous*: the block has to run on a later run
/// loop turn, not inline. Calling it synchronously (as touchHLE used to) breaks
/// the extremely common pattern of a method re-arming itself with
/// `dispatch_after` (blink/poll/retry animations) — that recurses into itself
/// without bound and either overflows the native stack or, with the
/// objc_msgSend recursion guard, corrupts state and starves the render loop.
/// Doodle Jump's `-[JumpAppDelegate blinkTournamentButton:]` is one such case.
///
/// We schedule the (heap-copied) block on an NSTimer, mirroring the existing
/// `+[UIView animateWithDuration:…completion:]` deferral mechanism.
fn dispatch_after(
    env: &mut Environment,
    when: u64, // dispatch_time_t
    _queue: dispatch_queue_t,
    block: dispatch_block_t,
) {
    if block.is_null() || when == DISPATCH_TIME_FOREVER {
        return;
    }
    let delay = dispatch_after_delay_secs(when);
    // Promote the (possibly stack-allocated) block to the heap so it survives
    // until the timer fires. Balanced by `_Block_release` in the fire method.
    let block = crate::libc::blocks::_Block_copy(env, block.cast_void().cast_const());
    log_dbg!("dispatch_after: scheduling block {:?} in {}s", block, delay);
    let num: id = msg_class![env; NSNumber numberWithUnsignedInt:(block.to_bits())];
    let key: id = get_static_str(env, "_touchHLE_dispatch_after_block");
    let dict: id = dict_from_keys_and_objects(env, &[(key, num)]);
    let sel = env
        .objc
        .lookup_selector("_touchHLE_dispatchAfterFireBlock:")
        .unwrap();
    let cls: Class = env.objc.get_known_class("_touchHLE_DispatchAfter", &mut env.mem);
    let _: id = msg_class![env;
        NSTimer scheduledTimerWithTimeInterval:delay
                                       target:cls
                                     selector:sel
                                     userInfo:dict
                                      repeats:false
    ];
}

fn dispatch_after_f(
    env: &mut Environment,
    when: u64,
    _queue: dispatch_queue_t,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    if work == GuestFunction::null_ptr() || when == DISPATCH_TIME_FOREVER {
        return;
    }
    let delay = dispatch_after_delay_secs(when);
    log_dbg!("dispatch_after_f: scheduling fn {:?} in {}s", work, delay);
    let ctx_num: id = msg_class![env; NSNumber numberWithUnsignedInt:(context.to_bits())];
    let work_num: id = msg_class![env; NSNumber numberWithUnsignedInt:(work.addr_with_thumb_bit())];
    let ctx_key: id = get_static_str(env, "_touchHLE_dispatch_after_ctx");
    let work_key: id = get_static_str(env, "_touchHLE_dispatch_after_work");
    let dict: id = dict_from_keys_and_objects(env, &[(ctx_key, ctx_num), (work_key, work_num)]);
    let sel = env
        .objc
        .lookup_selector("_touchHLE_dispatchAfterFireFunc:")
        .unwrap();
    let cls: Class = env.objc.get_known_class("_touchHLE_DispatchAfter", &mut env.mem);
    let _: id = msg_class![env;
        NSTimer scheduledTimerWithTimeInterval:delay
                                       target:cls
                                     selector:sel
                                     userInfo:dict
                                      repeats:false
    ];
}

/// Host-only helper class that owns the NSTimer callbacks used to implement
/// an asynchronous `dispatch_after`.
pub const CLASSES: ClassExports = objc_classes! {
(env, this, _cmd);

@implementation _touchHLE_DispatchAfter: NSObject

+ (())_touchHLE_dispatchAfterFireBlock:(id)timer {
    let dict: id = msg![env; timer userInfo];
    let key: id = get_static_str(env, "_touchHLE_dispatch_after_block");
    let queue_key: id = get_static_str(env, "_touchHLE_dispatch_after_queue");
    let num: id = msg![env; dict objectForKey:key];
    if num == crate::objc::nil {
        return;
    }
    let queue_num: id = msg![env; dict objectForKey:queue_key];
    let queue_bits: u32 = if queue_num != crate::objc::nil {
        msg![env; queue_num unsignedIntValue]
    } else {
        MAIN_QUEUE_PTR
    };
    let bits: u32 = msg![env; num unsignedIntValue];
    let block: dispatch_block_t = Ptr::from_bits(bits);
    if !block.is_null() {
        get_state(env).current_queue_stack.push(queue_bits);
        call_void_block(env, block);
        get_state(env).current_queue_stack.pop();
        crate::libc::blocks::_Block_release(env, block.cast_void().cast_const());
    }
}

+ (())_touchHLE_dispatchAfterFireFunc:(id)timer {
    let dict: id = msg![env; timer userInfo];
    let ctx_key: id = get_static_str(env, "_touchHLE_dispatch_after_ctx");
    let work_key: id = get_static_str(env, "_touchHLE_dispatch_after_work");
    let queue_key: id = get_static_str(env, "_touchHLE_dispatch_after_queue");
    let ctx_num: id = msg![env; dict objectForKey:ctx_key];
    let work_num: id = msg![env; dict objectForKey:work_key];
    if work_num == crate::objc::nil {
        return;
    }
    let queue_num: id = msg![env; dict objectForKey:queue_key];
    let queue_bits: u32 = if queue_num != crate::objc::nil {
        msg![env; queue_num unsignedIntValue]
    } else {
        MAIN_QUEUE_PTR
    };
    let ctx_bits: u32 = msg![env; ctx_num unsignedIntValue];
    let work_bits: u32 = msg![env; work_num unsignedIntValue];
    let context: MutVoidPtr = Ptr::from_bits(ctx_bits);
    let work = GuestFunction::from_addr_with_thumb_bit(work_bits);
    if work != GuestFunction::null_ptr() {
        get_state(env).current_queue_stack.push(queue_bits);
        let _: () = work.call_from_host(env, (context,));
        get_state(env).current_queue_stack.pop();
    }
}

+ (())_touchHLE_dispatchSourceTimerFired:(id)timer {
    super::dispatch_source::source_timer_fired(env, timer);
}

+ (())_touchHLE_dispatchSourceEventFired:(id)timer {
    super::dispatch_source::source_event_fired(env, timer);
}

@end
};

// MARK: - dispatch_time

fn dispatch_time(_env: &mut Environment, when: u64, delta: i64) -> u64 {
    if when == DISPATCH_TIME_FOREVER {
        return DISPATCH_TIME_FOREVER;
    }
    if delta >= 0 {
        when.saturating_add(delta as u64)
            .min(DISPATCH_TIME_FOREVER - 1)
    } else {
        when.saturating_sub(delta.unsigned_abs())
    }
}

fn dispatch_walltime(_env: &mut Environment, _spec: ConstVoidPtr, delta: i64) -> u64 {
    // Return a time `delta` nanoseconds from now (approximate).
    delta.max(0) as u64
}

// MARK: - dispatch_group

fn dispatch_group_create(env: &mut Environment) -> dispatch_group_t {
    let st = get_state(env);
    st.next_queue_handle += 2;
    let handle = st.next_queue_handle | 0x0200_0000;
    MutVoidPtr::from_bits(handle)
}

fn dispatch_group_async(
    env: &mut Environment,
    _group: dispatch_group_t,
    queue: dispatch_queue_t,
    block: dispatch_block_t,
) {
    if !block.is_null() {
        get_state(env).current_queue_stack.push(queue.to_bits());
        call_void_block(env, block);
        get_state(env).current_queue_stack.pop();
    }
}

fn dispatch_group_async_f(
    env: &mut Environment,
    _group: dispatch_group_t,
    queue: dispatch_queue_t,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    if work != GuestFunction::null_ptr() {
        get_state(env).current_queue_stack.push(queue.to_bits());
        let _: () = work.call_from_host(env, (context,));
        get_state(env).current_queue_stack.pop();
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
    queue: dispatch_queue_t,
    block: dispatch_block_t,
) {
    if !block.is_null() {
        get_state(env).current_queue_stack.push(queue.to_bits());
        call_void_block(env, block);
        get_state(env).current_queue_stack.pop();
    }
}

fn dispatch_group_notify_f(
    env: &mut Environment,
    _group: dispatch_group_t,
    queue: dispatch_queue_t,
    context: MutVoidPtr,
    work: GuestFunction,
) {
    if work != GuestFunction::null_ptr() {
        get_state(env).current_queue_stack.push(queue.to_bits());
        let _: () = work.call_from_host(env, (context,));
        get_state(env).current_queue_stack.pop();
    }
}

// MARK: - dispatch_semaphore

fn dispatch_semaphore_create(env: &mut Environment, value: i64) -> dispatch_semaphore_t {
    // A dispatch semaphore has to be able to actually block a waiter, so back
    // it with the same host semaphore object `sem_t` uses: that machinery is
    // already integrated with the thread scheduler
    // (`ThreadBlock::Semaphore`). A bare counter cannot block, which turns a
    // worker thread's "wait for work" into a busy spin and deadlocks any guest
    // that hands work to a thread pool.
    let backing = host_create_semaphore(env, value.max(0) as u32);
    let handle = {
        let st = get_state(env);
        st.next_queue_handle += 2;
        let h = st.next_queue_handle | 0x0300_0000;
        st.semaphores.insert(h, backing);
        h
    };
    MutVoidPtr::from_bits(handle)
}

fn dispatch_semaphore_wait(env: &mut Environment, sem: dispatch_semaphore_t, timeout: u64) -> i32 {
    let key = sem.to_bits();
    let Some(&backing) = get_state(env).semaphores.get(&key) else {
        log!("dispatch_semaphore_wait: unknown semaphore {:?}", sem);
        return -1;
    };
    // `DISPATCH_TIME_NOW` means "poll": return non-zero instead of blocking
    // when the semaphore is unavailable. Any other timeout blocks; we do not
    // model a finite timeout yet, so those wait until signalled (documented
    // below) rather than spinning.
    if timeout == DISPATCH_TIME_NOW {
        if env.sem_decrement(backing, false) {
            0
        } else {
            // Non-zero return == timed out, per dispatch_semaphore_wait(3).
            -1
        }
    } else if timeout == DISPATCH_TIME_FOREVER {
        // Block until signalled. (Must stay a real block: Telltale's
        // JobScheduler counts jobs via dispatch semaphores and does not
        // tolerate a wait returning without a real signal.)
        if false {
            log!(
                "DSEM t={} wait-enter sem={:#x} lr={:#x}",
                env.current_thread, sem.to_bits(), env.cpu.regs()[14]
            );
        }
        env.sem_decrement(backing, true);
        if false {
            log!("DSEM t={} wait-exit sem={:#x}", env.current_thread, sem.to_bits());
        }
        0
    } else {
        // Finite timeout: wait up to `timeout` and return non-zero (timed out)
        // if no signal arrives. This MUST be honored, not treated as infinite:
        // touchHLE's `dispatch_time(DISPATCH_TIME_NOW, ns)` returns the
        // relative nanosecond delta (it adds to a zero base), so `timeout` is
        // that duration. Telltale's `PlatformSemaphore::TimedWait` — the
        // render thread's frame handshake — depends on the wait actually
        // timing out so the render loop can poll and advance; treating it as
        // infinite deadlocked `RenderThread::FinishFrame` (the frozen-spinner
        // hang). Poll like semaphore_timedwait does.
        if false {
            log!(
                "DSEM t={} timedwait-enter sem={:#x} timeout_ns={} lr={:#x}",
                env.current_thread, sem.to_bits(), timeout, env.cpu.regs()[14]
            );
        }
        let timeout_ns = timeout.min(3_600_000_000_000); // cap at 1 hour
        let deadline = std::time::Instant::now() + std::time::Duration::from_nanos(timeout_ns);
        const POLL: std::time::Duration = std::time::Duration::from_millis(1);
        loop {
            if env.sem_decrement(backing, false) {
                return 0;
            }
            let now = std::time::Instant::now();
            if now >= deadline {
                // Timed out: dispatch_semaphore_wait returns non-zero.
                return 1;
            }
            env.sleep((deadline - now).min(POLL));
        }
    }
}

fn dispatch_semaphore_signal(env: &mut Environment, sem: dispatch_semaphore_t) -> i32 {
    let key = sem.to_bits();
    let Some(&backing) = get_state(env).semaphores.get(&key) else {
        log!("dispatch_semaphore_signal: unknown semaphore {:?}", sem);
        return 0;
    };
    if false {
        log!(
            "DSEM t={} signal sem={:#x} lr={:#x}",
            env.current_thread, sem.to_bits(), env.cpu.regs()[14]
        );
    }
    env.sem_increment(backing);
    // Returns non-zero if a thread was woken. We do not track that here, and
    // every in-tree caller ignores the result.
    0
}

/// Release the backing semaphore for a dispatch semaphore handle, if `obj` is
/// one. Returns whether it was.
fn release_if_semaphore(env: &mut Environment, obj: MutVoidPtr) -> bool {
    let key = obj.to_bits();
    let Some(backing) = get_state(env).semaphores.remove(&key) else {
        return false;
    };
    host_destroy_semaphore(env, backing);
    true
}

// MARK: - dispatch_source

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

fn dispatch_set_context(env: &mut Environment, object: MutVoidPtr, context: MutVoidPtr) {
    dispatch_source::set_context(env, object, context);
}

fn dispatch_get_context(env: &mut Environment, object: MutVoidPtr) -> MutVoidPtr {
    dispatch_source::get_context(env, object).unwrap_or_else(MutVoidPtr::null)
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
/// when nothing is running we resolve against the main queue on the main
/// thread (and the default global queue elsewhere, see [current_queue]),
/// matching the behaviour callers rely on for "am I on queue X?" checks.
/// <https://developer.apple.com/documentation/dispatch/1453099-dispatch_get_specific>
fn dispatch_get_specific(env: &mut Environment, key: ConstVoidPtr) -> MutVoidPtr {
    let current = current_queue(env);
    match get_state(env)
        .queue_specifics
        .get(&(current, key.to_bits()))
    {
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
pub(super) fn call_void_block(env: &mut Environment, block: dispatch_block_t) {
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
    export_c_func_aliased!("swift_once", swift_once(_, _, _)),
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
