/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `mach_init.h`
//!
//! There's not much documentation available for these.

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::environment::Environment;
use crate::libc::mach::port::mach_port_t;
use crate::mem::PAGE_SIZE;

// Unique mock value so we can assert against itself
pub const MACH_TASK_SELF: mach_port_t = 0x7461736b;

pub fn mach_task_self(_env: &mut Environment) -> mach_port_t {
    MACH_TASK_SELF
}

fn mach_thread_self(env: &mut Environment) -> mach_port_t {
    // TODO: implement port rights
    // for now, just return the thread id + 1.
    // (Plus 1 is to avoid having MACH_PORT_NULL for the main thread)
    (env.current_thread + 1).try_into().unwrap()
}

pub const CONSTANTS: ConstantExports = &[
    (
        "_mach_task_self_",
        HostConstant::Custom(|env| {
            env.mem
                .alloc_and_write(MACH_TASK_SELF)
                .cast_void()
                .cast_const()
        }),
    ),
    (
        "_vm_page_size",
        HostConstant::Custom(|env| env.mem.alloc_and_write(PAGE_SIZE).cast_void().cast_const()),
    ),
];

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(mach_task_self()),
    export_c_func!(mach_thread_self()),
];
