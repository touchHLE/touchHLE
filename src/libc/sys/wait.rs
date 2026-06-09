/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `sys/wait.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::{set_errno, ECHILD};
use crate::libc::unistd::{getpid, pid_t};
use crate::mem::MutPtr;
use crate::Environment;

const WNOHANG: i32 = 1;

fn waitpid(env: &mut Environment, pid: pid_t, stat_loc: MutPtr<i32>, options: i32) -> i32 {
    log_dbg!(
        "waitpid(pid {}, stat_loc {:?}, options {}) -> -1",
        pid,
        stat_loc,
        options
    );
    // Why would process want to wait on their own pid? Glad you've asked!
    // Apparently, Unity 1.0 iPhone is built atop of mono 2.0,
    // which have a bug of waiting on pid without checking their value
    // against own process first [link](https://github.com/mono/mono/blob/a1f3cf39287ceaca189ae1b4c06ad1677c8988cf/mono/io-layer/processes.c#L266).
    // Allegedly, it was fixed in mono 2.11 with [this commit](https://github.com/mono/mono/commit/afb1937e56100e368bc339045a685ef3c3b58e81).
    // There probably more nuisances here, but to answer your original question:
    // "It's because of mono bug!" ;-)
    assert_eq!(pid, getpid(env));
    assert_eq!(options, WNOHANG); // TODO

    set_errno(env, ECHILD);
    -1
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(waitpid(_, _, _))];
