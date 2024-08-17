/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `errno.h`

use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::mem::{ConstPtr, MutPtr};
use crate::Environment;
use std::io::Write;

pub const EPERM: i32 = 1;
pub const EBADF: i32 = 9;
pub const EDEADLK: i32 = 11;
pub const EBUSY: i32 = 16;
pub const EEXIST: i32 = 17;
pub const EINVAL: i32 = 22;

#[derive(Default)]
pub struct State {
    errnos: std::collections::HashMap<crate::ThreadId, MutPtr<i32>>,
}
impl State {
    fn errno_ptr_for_thread(
        &mut self,
        mem: &mut crate::mem::Mem,
        thread: crate::ThreadId,
    ) -> MutPtr<i32> {
        *self
            .errnos
            .entry(thread)
            .or_insert_with(|| mem.alloc_and_write(0i32))
    }

    pub fn set_errno_for_thread(
        &mut self,
        mem: &mut crate::mem::Mem,
        thread: crate::ThreadId,
        val: i32,
    ) {
        let ptr = self.errno_ptr_for_thread(mem, thread);
        mem.write(ptr, val);
    }
}

/// Helper function, not a part of libc errno
pub fn set_errno(env: &mut Environment, val: i32) {
    env.libc_state
        .errno
        .set_errno_for_thread(&mut env.mem, env.current_thread, val);
}

fn __error(env: &mut Environment) -> MutPtr<i32> {
    env.libc_state
        .errno
        .errno_ptr_for_thread(&mut env.mem, env.current_thread)
}

fn perror(env: &mut Environment, s: ConstPtr<u8>) {
    // TODO: errno mapping
    let errno_msg = "<TODO: errno>\n";
    let msg = if !s.is_null() {
        if let Ok(str) = env.mem.cstr_at_utf8(s) {
            format!("{}: {}", str, errno_msg)
        } else {
            errno_msg.to_string()
        }
    } else {
        errno_msg.to_string()
    };
    let _ = std::io::stderr().write_all(msg.as_bytes());
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(__error()), export_c_func!(perror(_))];
