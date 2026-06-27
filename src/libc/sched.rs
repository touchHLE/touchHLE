/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `sched.h`.

use crate::dyld::{export_c_func, FunctionExports};
use crate::Environment;
use std::time::Duration;

fn sched_yield(env: &mut Environment) -> i32 {
    // Apps that synchronise threads without `pthread_cond` (e.g. Angry Birds
    // 1.0) typically use a `while (!flag) sched_yield();` busy-wait loop on
    // one thread while another thread sets `flag`. Treating `sched_yield` as
    // a no-op leaves the polling thread holding 100% of touchHLE's
    // cooperative scheduler, so the producer thread never gets to run and the
    // app deadlocks. A zero-duration sleep gives the scheduler an opportunity
    // to pick another runnable thread, which is what `sched_yield(2)`
    // promises.
    log_dbg!("sched_yield by thread {}", env.current_thread);
    env.sleep(Duration::ZERO);
    0 // success
}

// Scheduling policy constants (matches iOS/macOS values)
const SCHED_OTHER: i32 = 1;
const SCHED_FIFO: i32 = 4;
const SCHED_RR: i32 = 2;

fn sched_get_priority_min(_env: &mut Environment, policy: i32) -> i32 {
    // On iOS: SCHED_OTHER/SCHED_RR/SCHED_FIFO all have min priority 0.
    log_dbg!("sched_get_priority_min({}) -> 0", policy);
    0
}

fn sched_get_priority_max(_env: &mut Environment, policy: i32) -> i32 {
    // On iOS: SCHED_FIFO/SCHED_RR max = 63, SCHED_OTHER max = 0.
    let max = match policy {
        SCHED_FIFO | SCHED_RR => 63,
        _ => 0,
    };
    log_dbg!("sched_get_priority_max({}) -> {}", policy, max);
    max
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sched_yield()),
    export_c_func!(sched_get_priority_min(_)),
    export_c_func!(sched_get_priority_max(_)),
];
