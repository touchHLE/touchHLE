/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `mach/thread_info.h`
//!
//! This is extremely undocumented. :(

#![allow(non_camel_case_types)]

use crate::dyld::{export_c_func, FunctionExports};
use crate::environment::ThreadBlock;
use crate::libc::mach::core_types::{boolean_t, integer_t, natural_t};
use crate::libc::mach::port::{mach_port_t, MACH_PORT_DEAD, MACH_PORT_NULL};
use crate::mem::{guest_size_of, MutPtr, SafeRead};
use crate::Environment;

// TODO: Move these common definitions into separate modules
pub type kern_return_t = i32;
pub const KERN_SUCCESS: kern_return_t = 0;
/// Specified address is not currently valid.
pub const KERN_INVALID_ADDRESS: kern_return_t = 1;
pub const KERN_INVALID_ARGUMENT: kern_return_t = 4;

pub type thread_inspect_t = mach_port_t;
type thread_flavor_t = natural_t;
type thread_info_t = MutPtr<integer_t>;
pub type thread_state_flavor_t = i32;
#[allow(dead_code)]
pub type thread_state_t = MutPtr<natural_t>;
pub type mach_msg_type_number_t = natural_t;

type policy_t = i32;
const POLICY_TIMESHARE: policy_t = 1;

const THREAD_BASIC_INFO: thread_flavor_t = 3;
const THREAD_SCHED_TIMESHARE_INFO: thread_flavor_t = 10;

#[repr(C, packed)]
struct time_value_t {
    seconds: integer_t,
    microseconds: integer_t,
}
unsafe impl SafeRead for time_value_t {}

#[repr(C, packed)]
struct thread_basic_info {
    user_time: time_value_t,
    system_time: time_value_t,
    cpu_usage: integer_t,
    policy: policy_t,
    run_state: integer_t,
    flags: integer_t,
    suspend_count: integer_t,
    sleep_time: integer_t,
}
unsafe impl SafeRead for thread_basic_info {}

#[repr(C, packed)]
struct policy_timeshare_info {
    max_priority: integer_t,
    base_priority: integer_t,
    cur_priority: integer_t,
    depressed: boolean_t,
    depress_priority: integer_t,
}
unsafe impl SafeRead for policy_timeshare_info {}

const TH_STATE_RUNNING: integer_t = 1;
const TH_STATE_STOPPED: integer_t = 2;
const TH_STATE_WAITING: integer_t = 3;

/// Undocumented Darwin function that returns information about a thread.
///
/// I swear these are the correct type names, the API is just... like this.
fn thread_info(
    env: &mut Environment,
    target_act: thread_inspect_t,
    flavor: thread_flavor_t,
    thread_info_out: thread_info_t,
    thread_info_out_count: MutPtr<mach_msg_type_number_t>,
) -> kern_return_t {
    if target_act == MACH_PORT_NULL || target_act == MACH_PORT_DEAD {
        // Real Mach returns KERN_INVALID_ARGUMENT (4); we don't have that
        // const wired up here yet, but any non-zero kern_return_t works.
        log!(
            "Warning: thread_info() called with invalid target_act {:?}; returning KERN_INVALID_ARGUMENT.",
            target_act
        );
        return 4;
    }
    let Some(thread) = env.threads.get((target_act - 1) as usize) else {
        log!(
            "Warning: thread_info(): target_act {:?} does not correspond to a known thread; returning KERN_INVALID_ARGUMENT.",
            target_act
        );
        return 4;
    };

    let out_size_available = env.mem.read(thread_info_out_count);

    match flavor {
        THREAD_BASIC_INFO => {
            let out_size_expected =
                guest_size_of::<thread_basic_info>() / guest_size_of::<integer_t>();
            if out_size_expected > out_size_available {
                // Real Mach returns MIG_ARRAY_TOO_LARGE / KERN_INVALID_ARGUMENT
                // when the caller's buffer is too small. Don't crash the host
                // on a guest passing a wrong-sized buffer.
                log!(
                    "Warning: thread_info(THREAD_BASIC_INFO): caller buffer too small ({} < {}); returning KERN_INVALID_ARGUMENT.",
                    out_size_available,
                    out_size_expected
                );
                return 4;
            }
            env.mem.write(
                thread_info_out.cast(),
                thread_basic_info {
                    user_time: time_value_t {
                        seconds: 0,
                        microseconds: 0,
                    },
                    system_time: time_value_t {
                        seconds: 0,
                        microseconds: 0,
                    },
                    cpu_usage: 0,
                    policy: POLICY_TIMESHARE, // no idea if this is realistic
                    run_state: if thread.active {
                        match thread.blocked_by {
                            ThreadBlock::NotBlocked => TH_STATE_RUNNING,
                            ThreadBlock::Suspended(_, _) => TH_STATE_WAITING,
                            _ => TH_STATE_WAITING,
                        }
                    } else {
                        TH_STATE_STOPPED
                    },
                    flags: 0, // FIXME
                    suspend_count: match thread.blocked_by {
                        ThreadBlock::Suspended(count, _) => count.try_into().unwrap_or(0),
                        _ => 0,
                    },
                    sleep_time: 0,
                },
            );
            env.mem.write(thread_info_out_count, out_size_expected);
        }
        THREAD_SCHED_TIMESHARE_INFO => {
            let out_size_expected =
                guest_size_of::<policy_timeshare_info>() / guest_size_of::<integer_t>();
            if out_size_expected > out_size_available {
                log!(
                    "Warning: thread_info(THREAD_SCHED_TIMESHARE_INFO): caller buffer too small ({} < {}); returning KERN_INVALID_ARGUMENT.",
                    out_size_available,
                    out_size_expected
                );
                return 4;
            }
            env.mem.write(
                thread_info_out.cast(),
                policy_timeshare_info {
                    max_priority: 0,
                    base_priority: 0,
                    cur_priority: 0,
                    depressed: 0,
                    depress_priority: 0,
                },
            );
            env.mem.write(thread_info_out_count, out_size_expected);
        }
        _ => {
            // Unknown thread_info flavor: return KERN_INVALID_ARGUMENT rather
            // than panicking the host. The app can fall back to whatever it
            // does on real Mach when an unsupported flavor is requested.
            log!(
                "Warning: thread_info(): unsupported flavor {:?}; returning KERN_INVALID_ARGUMENT.",
                flavor
            );
            return 4;
        }
    }

    KERN_SUCCESS
}

type thread_t = mach_port_t;
type thread_policy_flavor_t = natural_t;
type thread_policy_t = MutPtr<integer_t>;

// Идентификаторы политик планировщика потоков
const THREAD_EXTENDED_POLICY: thread_policy_flavor_t = 1;
const THREAD_TIME_CONSTRAINT_POLICY: thread_policy_flavor_t = 2;
const THREAD_PRECEDENCE_POLICY: thread_policy_flavor_t = 3;
const THREAD_AFFINITY_POLICY: thread_policy_flavor_t = 4;
const THREAD_BACKGROUND_POLICY: thread_policy_flavor_t = 5;

#[repr(C, packed)]
struct thread_extended_policy {
    timeshare: boolean_t,
}
unsafe impl SafeRead for thread_extended_policy {}

#[repr(C, packed)]
struct thread_time_constraint_policy {
    period: natural_t,
    computation: natural_t,
    constraint: natural_t,
    preemptible: boolean_t,
}
unsafe impl SafeRead for thread_time_constraint_policy {}

#[repr(C, packed)]
struct thread_precedence_policy {
    importance: integer_t,
}
unsafe impl SafeRead for thread_precedence_policy {}

#[repr(C, packed)]
struct thread_affinity_policy {
    affinity_tag: integer_t,
}
unsafe impl SafeRead for thread_affinity_policy {}

#[repr(C, packed)]
struct thread_background_policy {
    priority: integer_t,
}
unsafe impl SafeRead for thread_background_policy {}

// This is actually from the thread policy file.
fn thread_policy_set(
    env: &mut Environment,
    thread: thread_t,
    flavor: thread_policy_flavor_t,
    policy_info: thread_policy_t,
    count: mach_msg_type_number_t,
) -> kern_return_t {
    // Читаем из памяти переданные приложением параметры политики,
    // чтобы эмуляция доступа к памяти была корректной.
    // Фактически применять приоритеты в touchHLE пока не нужно,
    // поэтому мы просто поглощаем запрос и рапортуем об успехе.
    match flavor {
        THREAD_EXTENDED_POLICY => {
            let _policy: thread_extended_policy = env.mem.read(policy_info.cast());
        }
        THREAD_TIME_CONSTRAINT_POLICY => {
            let _policy: thread_time_constraint_policy = env.mem.read(policy_info.cast());
        }
        THREAD_PRECEDENCE_POLICY => {
            let _policy: thread_precedence_policy = env.mem.read(policy_info.cast());
        }
        THREAD_AFFINITY_POLICY => {
            let _policy: thread_affinity_policy = env.mem.read(policy_info.cast());
        }
        THREAD_BACKGROUND_POLICY => {
            let _policy: thread_background_policy = env.mem.read(policy_info.cast());
        }
        _ => {
            log!(
                "TODO: thread_policy_set({}, {}, {:?}, {}) (ignored)",
                thread,
                flavor,
                policy_info,
                count
            );
        }
    }

    KERN_SUCCESS
}

/// `kern_return_t thread_resume(thread_act_t target_act)` — resumes a
/// suspended thread, decrementing its suspend count, per Apple's Mach
/// kernel API (osfmk/kern/thread_act.c in XNU). Counterpart of
/// `thread_suspend`, and the standard way to start a thread created
/// suspended via `pthread_create_suspended_np`.
///
/// `target_act` is the value returned by `pthread_mach_thread_np()`,
/// i.e. `thread_id + 1` in touchHLE's port-numbering convention.
fn thread_resume(env: &mut Environment, target_act: thread_inspect_t) -> kern_return_t {
    if target_act == MACH_PORT_NULL || target_act == MACH_PORT_DEAD {
        return KERN_INVALID_ARGUMENT;
    }
    let thread_id = (target_act - 1) as usize;
    if thread_id >= env.threads.len() {
        log!(
            "Warning: thread_resume({:?}): unknown thread; returning KERN_INVALID_ARGUMENT.",
            target_act
        );
        return KERN_INVALID_ARGUMENT;
    }
    log_dbg!("thread_resume({:?}) => thread {}", target_act, thread_id);
    env.resume_thread(thread_id);
    KERN_SUCCESS
}

/// `kern_return_t thread_suspend(thread_act_t target_act)` — suspends a
/// thread, incrementing its suspend count, per Apple's Mach kernel API.
/// Counterpart of `thread_resume`.
fn thread_suspend(env: &mut Environment, target_act: thread_inspect_t) -> kern_return_t {
    if target_act == MACH_PORT_NULL || target_act == MACH_PORT_DEAD {
        return KERN_INVALID_ARGUMENT;
    }
    let thread_id = (target_act - 1) as usize;
    if thread_id >= env.threads.len() {
        log!(
            "Warning: thread_suspend({:?}): unknown thread; returning KERN_INVALID_ARGUMENT.",
            target_act
        );
        return KERN_INVALID_ARGUMENT;
    }
    log_dbg!("thread_suspend({:?}) => thread {}", target_act, thread_id);
    env.suspend_thread(thread_id);
    KERN_SUCCESS
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(thread_info(_, _, _, _)),
    export_c_func!(thread_policy_set(_, _, _, _)),
    export_c_func!(thread_resume(_)),
    export_c_func!(thread_suspend(_)),
];
