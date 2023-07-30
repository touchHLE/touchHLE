/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Miscellaneous parts of `unistd.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::ConstPtr;
use crate::Environment;
use std::time::Duration;

#[allow(non_camel_case_types)]
type useconds_t = u32;

fn sleep(env: &mut Environment, seconds: u32) -> u32 {
    env.sleep(Duration::from_secs(seconds.into()));
    // sleep() returns the amount of time remaining that should have been slept,
    // but wasn't, if the thread was woken up early by a signal.
    // touchHLE never does that currently, so 0 is always correct here.
    0
}

fn usleep(env: &mut Environment, useconds: useconds_t) -> i32 {
    env.sleep(Duration::from_micros(useconds.into()));
    0 // success
}

fn chdir(env: &mut Environment, path: ConstPtr<u8>) -> i32 {
    let str = env.mem.cstr_at_utf8(path).unwrap();
    log_dbg!("chdir {}", str);
    env.fs.change_current_directory(str).unwrap();
    // TODO: error handling
    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sleep(_)),
    export_c_func!(usleep(_)),
    export_c_func!(chdir(_)),
];
