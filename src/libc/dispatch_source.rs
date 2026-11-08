/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Grand Central Dispatch source support for timers and merged-data events.

use crate::abi::{CallFromHost, GuestFunction};
use crate::frameworks::foundation::ns_dictionary::dict_from_keys_and_objects;
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::mem::{ConstVoidPtr, MutVoidPtr};
use crate::objc::{id, msg, msg_class, nil, release, retain, Class};
use crate::Environment;

use super::dispatch::{
    call_void_block, dispatch_after_delay_secs, dispatch_block_t, dispatch_queue_t,
    dispatch_source_t, get_state, State, DISPATCH_TIME_FOREVER, MAIN_QUEUE_PTR,
};

const SOURCE_TYPE_READ: u32 = 0x4453_0001;
const SOURCE_TYPE_WRITE: u32 = 0x4453_0002;
const SOURCE_TYPE_TIMER: u32 = 0x4453_0003;
const SOURCE_TYPE_DATA_ADD: u32 = 0x4453_0004;
const SOURCE_TYPE_DATA_OR: u32 = 0x4453_0005;
const SOURCE_TYPE_DATA_REPLACE: u32 = 0x4453_0006;
const SOURCE_TYPE_SIGNAL: u32 = 0x4453_0007;
const SOURCE_TYPE_PROC: u32 = 0x4453_0008;
const SOURCE_TYPE_VNODE: u32 = 0x4453_0009;
const SOURCE_TYPE_MACH_SEND: u32 = 0x4453_000a;
const SOURCE_TYPE_MACH_RECV: u32 = 0x4453_000b;
const SOURCE_TYPE_MEMORYPRESSURE: u32 = 0x4453_000c;
const SOURCE_ID_KEY: &str = "_touchHLE_dispatch_source";
const SOURCE_TIMER_SELECTOR: &str = "_touchHLE_dispatchSourceTimerFired:";
const SOURCE_EVENT_SELECTOR: &str = "_touchHLE_dispatchSourceEventFired:";
const SOURCE_TARGET_CLASS: &str = "_touchHLE_DispatchAfter";

#[allow(dead_code)]
pub(crate) fn source_type_tag(name: &str) -> u32 {
    match name {
        "__dispatch_source_type_read" => SOURCE_TYPE_READ,
        "__dispatch_source_type_write" => SOURCE_TYPE_WRITE,
        "__dispatch_source_type_timer" => SOURCE_TYPE_TIMER,
        "__dispatch_source_type_data_add" => SOURCE_TYPE_DATA_ADD,
        "__dispatch_source_type_data_or" => SOURCE_TYPE_DATA_OR,
        "__dispatch_source_type_data_replace" => SOURCE_TYPE_DATA_REPLACE,
        "__dispatch_source_type_signal" => SOURCE_TYPE_SIGNAL,
        "__dispatch_source_type_proc" => SOURCE_TYPE_PROC,
        "__dispatch_source_type_vnode" => SOURCE_TYPE_VNODE,
        "__dispatch_source_type_mach_send" => SOURCE_TYPE_MACH_SEND,
        "__dispatch_source_type_mach_recv" => SOURCE_TYPE_MACH_RECV,
        "__dispatch_source_type_memorypressure" => SOURCE_TYPE_MEMORYPRESSURE,
        _ => 0,
    }
}

#[derive(Clone, Copy)]
enum SourceHandler {
    Block(dispatch_block_t),
    Function(GuestFunction),
}

pub(super) struct DispatchSource {
    source_type: u32,
    handle: usize,
    mask: usize,
    queue: dispatch_queue_t,
    context: MutVoidPtr,
    event_handler: Option<SourceHandler>,
    cancel_handler: Option<SourceHandler>,
    cancel_handler_called: bool,
    pending_data: usize,
    delivering_data: usize,
    delivering: bool,
    cancelled: bool,
    suspension_count: u32,
    retain_count: u32,
    timer_start: u64,
    timer_interval: u64,
    timer_leeway: u64,
    timer: id,
    event_timer: id,
}

fn source_id(source: u32) -> MutVoidPtr {
    MutVoidPtr::from_bits(source)
}

fn schedule_timer(
    env: &mut Environment,
    source: u32,
    selector_name: &str,
    delay: f64,
    repeats: bool,
) -> id {
    let source_number: id = msg_class![env; NSNumber numberWithUnsignedInt:source];
    let key: id = get_static_str(env, SOURCE_ID_KEY);
    let user_info = dict_from_keys_and_objects(env, &[(key, source_number)]);
    let selector = env.objc.lookup_selector(selector_name).unwrap();
    let target: Class = env.objc.get_known_class(SOURCE_TARGET_CLASS, &mut env.mem);
    let timer: id = msg_class![env;
        NSTimer scheduledTimerWithTimeInterval:delay
                                       target:target
                                     selector:selector
                                     userInfo:user_info
                                      repeats:repeats
    ];
    if timer != nil {
        retain(env, timer);
    }
    timer
}

fn invalidate_and_release_timer(env: &mut Environment, timer: id) {
    if timer != nil {
        let _: () = msg![env; timer invalidate];
        release(env, timer);
    }
}

fn schedule_source_timer(env: &mut Environment, source: u32) {
    let timer_parameters = {
        let state: &mut State = get_state(env);
        let Some(source) = state.sources.get(&source) else {
            return;
        };
        if source.cancelled
            || source.suspension_count != 0
            || source.source_type != SOURCE_TYPE_TIMER
            || source.timer_start == DISPATCH_TIME_FOREVER
            || source.timer != nil
        {
            return;
        }
        (
            source.timer_start,
            source.timer_interval,
            source.timer_leeway,
        )
    };
    let (start, interval_nanos, _leeway_nanos) = timer_parameters;
    let delay = dispatch_after_delay_secs(start);
    let repeats = interval_nanos != 0;
    let interval = if repeats {
        ((interval_nanos as f64) / 1_000_000_000.0).clamp(0.001, 3600.0)
    } else {
        delay
    };
    let timer = schedule_timer(
        env,
        source,
        SOURCE_TIMER_SELECTOR,
        interval.max(delay),
        repeats,
    );
    if timer == nil {
        return;
    }
    let keep_timer = {
        let state = get_state(env);
        if let Some(source) = state.sources.get_mut(&source) {
            if !source.cancelled && source.suspension_count == 0 && source.timer == nil {
                source.timer = timer;
                true
            } else {
                false
            }
        } else {
            false
        }
    };
    if !keep_timer {
        invalidate_and_release_timer(env, timer);
    }
}

fn source_has_event(source: &DispatchSource) -> bool {
    !source.cancelled
        && source.suspension_count == 0
        && source.event_handler.is_some()
        && source.pending_data != 0
        && source.event_timer == nil
}

fn schedule_source_event(env: &mut Environment, source: u32) {
    let should_schedule = get_state(env)
        .sources
        .get(&source)
        .is_some_and(source_has_event);
    if !should_schedule {
        return;
    }
    let timer = schedule_timer(env, source, SOURCE_EVENT_SELECTOR, 0.0, false);
    if timer == nil {
        dispatch_source_fire_event(env, source);
        return;
    }
    let keep_timer = {
        let state = get_state(env);
        if let Some(source) = state.sources.get_mut(&source) {
            if source_has_event(source) {
                source.event_timer = timer;
                true
            } else {
                false
            }
        } else {
            false
        }
    };
    if !keep_timer {
        invalidate_and_release_timer(env, timer);
    }
}

fn invoke_source_handler(
    env: &mut Environment,
    handler: SourceHandler,
    queue: dispatch_queue_t,
    context: MutVoidPtr,
) {
    get_state(env).current_queue_stack.push(queue.to_bits());
    match handler {
        SourceHandler::Block(block) => call_void_block(env, block),
        SourceHandler::Function(function) => {
            if function != GuestFunction::null_ptr() {
                let _: () = function.call_from_host(env, (context,));
            }
        }
    }
    get_state(env).current_queue_stack.pop();
}

fn dispatch_source_fire_event(env: &mut Environment, source: u32) {
    let event = {
        let Some(source) = get_state(env).sources.get_mut(&source) else {
            return;
        };
        if !source_has_event(source) {
            return;
        }
        source.delivering_data = source.pending_data;
        source.pending_data = 0;
        source.delivering = true;
        (source.event_handler.unwrap(), source.queue, source.context)
    };
    invoke_source_handler(env, event.0, event.1, event.2);
    if let Some(source) = get_state(env).sources.get_mut(&source) {
        source.delivering = false;
        source.delivering_data = 0;
    }
    schedule_source_event(env, source);
}

fn release_handler(env: &mut Environment, handler: Option<SourceHandler>) {
    if let Some(SourceHandler::Block(block)) = handler {
        crate::libc::blocks::_Block_release(env, block.cast_void().cast_const());
    }
}

fn copy_block_handler(env: &mut Environment, block: dispatch_block_t) -> Option<SourceHandler> {
    if block.is_null() {
        None
    } else {
        let copied = crate::libc::blocks::_Block_copy(env, block.cast_void().cast_const());
        Some(SourceHandler::Block(MutVoidPtr::from_bits(
            copied.to_bits(),
        )))
    }
}

fn set_event_handler(
    env: &mut Environment,
    source: dispatch_source_t,
    handler: Option<SourceHandler>,
) {
    let old_handler = {
        let Some(source) = get_state(env).sources.get_mut(&source.to_bits()) else {
            release_handler(env, handler);
            return;
        };
        std::mem::replace(&mut source.event_handler, handler)
    };
    release_handler(env, old_handler);
    schedule_source_event(env, source.to_bits());
}

fn set_cancel_handler(
    env: &mut Environment,
    source: dispatch_source_t,
    handler: Option<SourceHandler>,
) {
    let (old_handler, invoke) = {
        let Some(source) = get_state(env).sources.get_mut(&source.to_bits()) else {
            release_handler(env, handler);
            return;
        };
        let old = std::mem::replace(&mut source.cancel_handler, handler);
        let invoke = if source.cancelled && !source.cancel_handler_called {
            source.cancel_handler_called = true;
            source
                .cancel_handler
                .map(|handler| (handler, source.queue, source.context))
        } else {
            None
        };
        (old, invoke)
    };
    release_handler(env, old_handler);
    if let Some((handler, queue, context)) = invoke {
        invoke_source_handler(env, handler, queue, context);
    }
}

pub(super) fn dispatch_source_create(
    env: &mut Environment,
    source_type: ConstVoidPtr,
    handle: usize,
    mask: usize,
    queue: dispatch_queue_t,
) -> dispatch_source_t {
    if source_type.is_null() {
        return MutVoidPtr::null();
    }
    let type_tag: u32 = env.mem.read(source_type.cast());
    if !matches!(
        type_tag,
        SOURCE_TYPE_READ
            | SOURCE_TYPE_WRITE
            | SOURCE_TYPE_TIMER
            | SOURCE_TYPE_DATA_ADD
            | SOURCE_TYPE_DATA_OR
            | SOURCE_TYPE_DATA_REPLACE
            | SOURCE_TYPE_SIGNAL
            | SOURCE_TYPE_PROC
            | SOURCE_TYPE_VNODE
            | SOURCE_TYPE_MACH_SEND
            | SOURCE_TYPE_MACH_RECV
            | SOURCE_TYPE_MEMORYPRESSURE
    ) {
        log!(
            "dispatch_source_create: unsupported source type tag {:#x}",
            type_tag
        );
        return MutVoidPtr::null();
    }
    let source = {
        let state = get_state(env);
        let mut source_handle = state.next_source_handle.max(0x0400_0001);
        while state.sources.contains_key(&source_handle) {
            source_handle = source_handle.wrapping_add(4);
            if source_handle < 0x0400_0001 {
                source_handle = 0x0400_0001;
            }
        }
        state.next_source_handle = source_handle.wrapping_add(4);
        state.sources.insert(
            source_handle,
            DispatchSource {
                source_type: type_tag,
                handle,
                mask,
                queue: if queue.is_null() {
                    MutVoidPtr::from_bits(MAIN_QUEUE_PTR)
                } else {
                    queue
                },
                context: MutVoidPtr::null(),
                event_handler: None,
                cancel_handler: None,
                cancel_handler_called: false,
                pending_data: 0,
                delivering_data: 0,
                delivering: false,
                cancelled: false,
                suspension_count: 1,
                retain_count: 1,
                timer_start: DISPATCH_TIME_FOREVER,
                timer_interval: 0,
                timer_leeway: 0,
                timer: nil,
                event_timer: nil,
            },
        );
        source_handle
    };
    source_id(source)
}

pub(super) fn dispatch_source_set_event_handler(
    env: &mut Environment,
    source: dispatch_source_t,
    handler: dispatch_block_t,
) {
    let handler = copy_block_handler(env, handler);
    set_event_handler(env, source, handler);
}

pub(super) fn dispatch_source_set_event_handler_f(
    env: &mut Environment,
    source: dispatch_source_t,
    handler: GuestFunction,
) {
    set_event_handler(
        env,
        source,
        (handler != GuestFunction::null_ptr()).then_some(SourceHandler::Function(handler)),
    );
}

pub(super) fn dispatch_source_set_cancel_handler(
    env: &mut Environment,
    source: dispatch_source_t,
    handler: dispatch_block_t,
) {
    let handler = copy_block_handler(env, handler);
    set_cancel_handler(env, source, handler);
}

pub(super) fn dispatch_source_set_cancel_handler_f(
    env: &mut Environment,
    source: dispatch_source_t,
    handler: GuestFunction,
) {
    set_cancel_handler(
        env,
        source,
        (handler != GuestFunction::null_ptr()).then_some(SourceHandler::Function(handler)),
    );
}

pub(super) fn dispatch_source_cancel(env: &mut Environment, source: dispatch_source_t) {
    let (timer, event_timer, cancel_handler) = {
        let Some(source) = get_state(env).sources.get_mut(&source.to_bits()) else {
            return;
        };
        if source.cancelled {
            return;
        }
        source.cancelled = true;
        let handler = if source.cancel_handler_called {
            None
        } else {
            source.cancel_handler_called = true;
            source
                .cancel_handler
                .map(|handler| (handler, source.queue, source.context))
        };
        (
            std::mem::replace(&mut source.timer, nil),
            std::mem::replace(&mut source.event_timer, nil),
            handler,
        )
    };
    invalidate_and_release_timer(env, timer);
    invalidate_and_release_timer(env, event_timer);
    if let Some((handler, queue, context)) = cancel_handler {
        invoke_source_handler(env, handler, queue, context);
    }
}

pub(super) fn dispatch_source_testcancel(env: &mut Environment, source: dispatch_source_t) -> i32 {
    get_state(env)
        .sources
        .get(&source.to_bits())
        .is_some_and(|source| source.cancelled) as i32
}

pub(super) fn dispatch_source_get_handle(
    env: &mut Environment,
    source: dispatch_source_t,
) -> usize {
    get_state(env)
        .sources
        .get(&source.to_bits())
        .map_or(0, |source| source.handle)
}

pub(super) fn dispatch_source_get_mask(env: &mut Environment, source: dispatch_source_t) -> usize {
    get_state(env)
        .sources
        .get(&source.to_bits())
        .map_or(0, |source| source.mask)
}

pub(super) fn dispatch_source_get_data(env: &mut Environment, source: dispatch_source_t) -> usize {
    get_state(env)
        .sources
        .get(&source.to_bits())
        .map_or(0, |source| {
            if source.delivering {
                source.delivering_data
            } else {
                source.pending_data
            }
        })
}

pub(super) fn dispatch_source_merge_data(
    env: &mut Environment,
    source: dispatch_source_t,
    value: usize,
) {
    let Some(source_state) = get_state(env).sources.get_mut(&source.to_bits()) else {
        return;
    };
    if source_state.cancelled {
        return;
    }
    match source_state.source_type {
        SOURCE_TYPE_DATA_ADD => {
            source_state.pending_data = source_state.pending_data.saturating_add(value)
        }
        SOURCE_TYPE_DATA_OR => source_state.pending_data |= value,
        SOURCE_TYPE_DATA_REPLACE => source_state.pending_data = value,
        _ => {
            log!("dispatch_source_merge_data: source type does not merge data");
            return;
        }
    }
    schedule_source_event(env, source.to_bits());
}

pub(super) fn dispatch_source_set_timer(
    env: &mut Environment,
    source: dispatch_source_t,
    start: u64,
    interval: u64,
    leeway: u64,
) {
    let old_timer = {
        let Some(source) = get_state(env).sources.get_mut(&source.to_bits()) else {
            return;
        };
        if source.source_type != SOURCE_TYPE_TIMER {
            log!("dispatch_source_set_timer: source is not a timer");
            return;
        }
        source.timer_start = start;
        source.timer_interval = interval;
        source.timer_leeway = leeway;
        std::mem::replace(&mut source.timer, nil)
    };
    invalidate_and_release_timer(env, old_timer);
    schedule_source_timer(env, source.to_bits());
}

pub(super) fn dispatch_resume(env: &mut Environment, object: MutVoidPtr) {
    let source = object.to_bits();
    let should_start = {
        let Some(source) = get_state(env).sources.get_mut(&source) else {
            return;
        };
        if source.suspension_count == 0 {
            return;
        }
        source.suspension_count -= 1;
        source.suspension_count == 0
    };
    if should_start {
        schedule_source_timer(env, source);
        schedule_source_event(env, source);
    }
}

pub(super) fn dispatch_suspend(env: &mut Environment, object: MutVoidPtr) {
    let source = object.to_bits();
    let (timer, event_timer) = {
        let Some(source) = get_state(env).sources.get_mut(&source) else {
            return;
        };
        source.suspension_count = source.suspension_count.saturating_add(1);
        if source.suspension_count == 1 {
            (
                std::mem::replace(&mut source.timer, nil),
                std::mem::replace(&mut source.event_timer, nil),
            )
        } else {
            (nil, nil)
        }
    };
    invalidate_and_release_timer(env, timer);
    invalidate_and_release_timer(env, event_timer);
}

pub(super) fn retain_source(env: &mut Environment, object: MutVoidPtr) -> bool {
    let Some(source) = get_state(env).sources.get_mut(&object.to_bits()) else {
        return false;
    };
    source.retain_count = source.retain_count.saturating_add(1);
    true
}

pub(super) fn release_source(env: &mut Environment, object: MutVoidPtr) -> bool {
    let source_id = object.to_bits();
    let removed = {
        let state = get_state(env);
        let Some(source) = state.sources.get_mut(&source_id) else {
            return false;
        };
        if source.retain_count > 1 {
            source.retain_count -= 1;
            return true;
        }
        state.sources.remove(&source_id).unwrap()
    };
    invalidate_and_release_timer(env, removed.timer);
    invalidate_and_release_timer(env, removed.event_timer);
    release_handler(env, removed.event_handler);
    release_handler(env, removed.cancel_handler);
    true
}

pub(super) fn set_context(env: &mut Environment, object: MutVoidPtr, context: MutVoidPtr) -> bool {
    let Some(source) = get_state(env).sources.get_mut(&object.to_bits()) else {
        return false;
    };
    source.context = context;
    true
}

pub(super) fn get_context(env: &mut Environment, object: MutVoidPtr) -> Option<MutVoidPtr> {
    get_state(env)
        .sources
        .get(&object.to_bits())
        .map(|source| source.context)
}

pub(super) fn source_timer_fired(env: &mut Environment, timer: id) {
    let info: id = msg![env; timer userInfo];
    let key: id = get_static_str(env, SOURCE_ID_KEY);
    let number: id = msg![env; info objectForKey:key];
    if number == nil {
        return;
    }
    let source: u32 = msg![env; number unsignedIntValue];
    let old_timer = {
        let Some(source_state) = get_state(env).sources.get_mut(&source) else {
            return;
        };
        if source_state.cancelled || source_state.suspension_count != 0 {
            return;
        }
        source_state.pending_data = source_state.pending_data.saturating_add(1);
        if source_state.timer_interval == 0 {
            std::mem::replace(&mut source_state.timer, nil)
        } else {
            nil
        }
    };
    if old_timer != nil {
        release(env, old_timer);
    }
    schedule_source_event(env, source);
}

pub(super) fn source_event_fired(env: &mut Environment, timer: id) {
    let info: id = msg![env; timer userInfo];
    let key: id = get_static_str(env, SOURCE_ID_KEY);
    let number: id = msg![env; info objectForKey:key];
    if number == nil {
        return;
    }
    let source: u32 = msg![env; number unsignedIntValue];
    let event_timer = get_state(env)
        .sources
        .get_mut(&source)
        .map(|source| std::mem::replace(&mut source.event_timer, nil));
    if let Some(event_timer) = event_timer {
        release(env, event_timer);
    }
    dispatch_source_fire_event(env, source);
}
