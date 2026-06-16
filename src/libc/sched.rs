/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `sched.h`.

use crate::dyld::{export_c_func, FunctionExports};
use crate::Environment;

fn sched_yield(env: &mut Environment) -> i32 {
    log_dbg!(
        "TODO: thread {} requested processor yield, ignoring",
        env.current_thread
    );
    0 // success
}

fn sched_get_priority_max(_env: &mut Environment, _priority: i32) -> i32 {
    // iOS ignores the priority value and always returns 0
    log_dbg!(
        "sched_get_priority_max: ignoring priority level {}",
        _priority
    );
    0
}

fn sched_get_priority_min(_env: &mut Environment, _priority: i32) -> i32 {
    // iOS ignores the priority value and always returns 0
    log_dbg!(
        "sched_get_priority_min: ignoring priority level {}",
        _priority
    );
    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sched_yield()),
    export_c_func!(sched_get_priority_min(_)),
    export_c_func!(sched_get_priority_max(_)),
];
