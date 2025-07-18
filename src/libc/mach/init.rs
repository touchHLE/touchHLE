/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `mach_init.h`
//!
//! There's not much documentation available for these.

use crate::dyld::{ConstantExports, HostConstant};
use crate::libc::mach::task::MACH_TASK_SELF;
use crate::libc::mach::vm::PAGE_SIZE;

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
