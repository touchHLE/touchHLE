/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `time.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{MutPtr, SafeRead};
use crate::Environment;
use std::time::SystemTime;

#[derive(Default)]
pub struct State {
    y2k38_warned: bool,
}

#[allow(non_camel_case_types)]
type time_t = i32;

#[repr(C, packed)]
struct timeval {
    tv_sec: time_t,
    tv_usec: u32,
}
unsafe impl SafeRead for timeval {}

#[repr(C, packed)]
struct timezone {
    tz_minuteswest: i32,
    tz_dsttime: i32,
}
unsafe impl SafeRead for timezone {}

fn time(env: &mut Environment, out: MutPtr<time_t>) -> time_t {
    let time64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let time = time64 as time_t;
    if !env.libc_state.time.y2k38_warned && time64 != time as u64 {
        env.libc_state.time.y2k38_warned = true;
        log!("Warning: system clock is beyond Y2K38 and might confuse the app");
    }
    if !out.is_null() {
        env.mem.write(out, time);
    }
    time
}

fn gettimeofday(
    _env: &mut Environment,
    _timeval: MutPtr<timeval>,
    _timezone: MutPtr<timezone>,
) -> i32 {
    // TODO: actually implement this
    return 0;
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(time(_)),
    export_c_func!(gettimeofday(_, _)),
];
