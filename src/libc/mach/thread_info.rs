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

pub type thread_inspect_t = mach_port_t;
type thread_flavor_t = natural_t;
type thread_info_t = MutPtr<integer_t>;
pub type thread_state_flavor_t = i32;
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
    assert!(target_act != MACH_PORT_NULL && target_act != MACH_PORT_DEAD);
    let thread = env.threads.get((target_act - 1) as usize).unwrap();

    let out_size_available = env.mem.read(thread_info_out_count);

    match flavor {
        THREAD_BASIC_INFO => {
            let out_size_expected =
                guest_size_of::<thread_basic_info>() / guest_size_of::<integer_t>();
            assert!(out_size_expected <= out_size_available);
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
                            ThreadBlock::Suspended(count, _) => {
                                assert!(count > 0);
                                TH_STATE_WAITING
                            }
                            _ => TH_STATE_WAITING,
                        }
                    } else {
                        TH_STATE_STOPPED
                    },
                    flags: 0, // FIXME
                    suspend_count: match thread.blocked_by {
                        ThreadBlock::Suspended(count, _) => {
                            assert!(count > 0);
                            count.try_into().unwrap()
                        }
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
            assert!(out_size_expected <= out_size_available);
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
        _ => unimplemented!("TODO: flavor {:?}", flavor),
    }

    KERN_SUCCESS
}

type thread_t = mach_port_t;
type thread_policy_flavor_t = natural_t;
type thread_policy_t = MutPtr<integer_t>;

// This is actually from the thread policy file.
fn thread_policy_set(
    _env: &mut Environment,
    thread: thread_t,
    flavor: thread_policy_flavor_t,
    policy_info: thread_policy_t,
    count: mach_msg_type_number_t,
) -> kern_return_t {
    log!(
        "TODO: thread_policy_set({}, {}, {:?}, {}) (ignored)",
        thread,
        flavor,
        policy_info,
        count
    );
    KERN_SUCCESS
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(thread_info(_, _, _, _)),
    export_c_func!(thread_policy_set(_, _, _, _)),
];
