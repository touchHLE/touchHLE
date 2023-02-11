/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `time.h` (C) and `sys/time.h` (POSIX)

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{MutPtr, MutVoidPtr, SafeRead};
use crate::Environment;
use std::time::SystemTime;

#[derive(Default)]
pub struct State {
    y2k38_warned: bool,
}

// time.h (C)

#[allow(non_camel_case_types)]
type time_t = i32;

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

// sys/time.h (POSIX)

#[allow(non_camel_case_types)]
type suseconds_t = i32;

#[allow(non_camel_case_types)]
#[repr(C, packed)]
struct timeval {
    tv_sec: time_t,
    tv_usec: suseconds_t,
}
unsafe impl SafeRead for timeval {}

fn gettimeofday(
    env: &mut Environment,
    timeval_ptr: MutPtr<timeval>,
    timezone_ptr: MutVoidPtr, // deprecated, always NULL
) -> i32 {
    assert!(timezone_ptr.is_null());

    if timeval_ptr.is_null() {
        return 0; // success
    }

    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let time_s_64: u64 = time.as_secs();
    let tv_sec = time_s_64 as time_t;
    if !env.libc_state.time.y2k38_warned && time_s_64 != tv_sec as u64 {
        env.libc_state.time.y2k38_warned = true;
        log!("Warning: system clock is beyond Y2K38 and might confuse the app");
    }
    let tv_usec: suseconds_t = time.subsec_micros().try_into().unwrap();

    env.mem.write(timeval_ptr, timeval { tv_sec, tv_usec });

    0 // success
}

pub const FUNCTIONS: FunctionExports =
    &[export_c_func!(time(_)), export_c_func!(gettimeofday(_, _))];
