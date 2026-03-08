/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Miscellaneous parts of `unistd.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::fs::GuestPath;
use crate::libc::errno::{set_errno, EACCES, ENOENT, EROFS};
use crate::libc::posix_io::{FileDescriptor, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO};
use crate::mem::{ConstPtr, GuestUSize, MutPtr, PAGE_SIZE};
use crate::Environment;
use std::time::Duration;

#[allow(non_camel_case_types)]
type useconds_t = u32;

const F_OK: i32 = 0; // file existence
const X_OK: i32 = 1; // execute/search permission
const W_OK: i32 = 2; // write permission
const R_OK: i32 = 4; // read permission

fn sleep(env: &mut Environment, seconds: u32) -> u32 {
    env.sleep(Duration::from_secs(seconds.into()), true);
    // sleep() returns the amount of time remaining that should have been slept,
    // but wasn't, if the thread was woken up early by a signal.
    // touchHLE never does that currently, so 0 is always correct here.
    0
}

fn usleep(env: &mut Environment, useconds: useconds_t) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    env.sleep(Duration::from_micros(useconds.into()), true);
    0 // success
}

#[allow(non_camel_case_types)]
pub type pid_t = i32;
#[allow(non_camel_case_types)]
type gid_t = u32;

fn getpid(_env: &mut Environment) -> pid_t {
    // Not a real value, since touchHLE only simulates a single process.
    // PID 0 would be init, which is a bit unrealistic, so let's go with 1.
    1
}
fn getppid(_env: &mut Environment) -> pid_t {
    // Included just for completeness. Surely no app ever calls this.
    0
}
fn getgid(_env: &mut Environment) -> gid_t {
    // Not a real value
    0
}

fn isatty(env: &mut Environment, fd: FileDescriptor) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    if [STDIN_FILENO, STDOUT_FILENO, STDERR_FILENO].contains(&fd) {
        1
    } else {
        0
    }
}

fn access(env: &mut Environment, path: ConstPtr<u8>, mode: i32) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let binding = env.mem.cstr_at_utf8(path).unwrap();
    let guest_path = GuestPath::new(&binding);
    let (exists, read, write, execute) = env.fs.access(guest_path);
    // TODO: support ORing
    match mode {
        F_OK => {
            if exists {
                0
            } else {
                set_errno(env, ENOENT);
                -1
            }
        }
        X_OK => {
            if execute {
                0
            } else {
                set_errno(env, EACCES);
                -1
            }
        }
        W_OK => {
            if write {
                0
            } else {
                set_errno(env, EROFS);
                -1
            }
        }
        R_OK => {
            if read {
                0
            } else {
                // TODO: is it the correct error?
                set_errno(env, EACCES);
                -1
            }
        }
        _ => unimplemented!("{}", mode),
    }
}

fn unlink(env: &mut Environment, path: ConstPtr<u8>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    log_dbg!("unlink({:?} '{:?}')", path, env.mem.cstr_at_utf8(path));

    let path_str = env.mem.cstr_at_utf8(path).unwrap();
    let guest_path = GuestPath::new(&path_str);
    match env.fs.remove(guest_path) {
        Ok(()) => 0,
        Err(_) => {
            log!(
                "unlink({:?} '{:?}') failed",
                path,
                env.mem.cstr_at_utf8(path)
            );
            -1
        }
    }
}

fn gethostname(env: &mut Environment, name: MutPtr<u8>, namelen: GuestUSize) -> i32 {
    // TODO: define unique hostname once networking is supported
    let hostname = "touchHLE";
    let len: GuestUSize = hostname.len().try_into().unwrap();
    // TODO: check against HOST_NAME_MAX
    assert!(namelen > len);
    env.mem
        .bytes_at_mut(name, len)
        .copy_from_slice(hostname.as_bytes());
    env.mem.write(name + len, b'\0');
    0 // Success
}

fn getpagesize(_env: &mut Environment) -> i32 {
    PAGE_SIZE.try_into().unwrap()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sleep(_)),
    export_c_func!(usleep(_)),
    export_c_func!(getpid()),
    export_c_func!(getppid()),
    export_c_func!(isatty(_)),
    export_c_func!(access(_, _)),
    export_c_func!(unlink(_)),
    export_c_func!(gethostname(_, _)),
    export_c_func!(getpagesize()),
    export_c_func!(getgid()),
];
