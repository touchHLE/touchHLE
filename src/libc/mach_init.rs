/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `mach_init.h`
//!
//! There's not much documentation available for these.

use crate::dyld::{ConstantExports, HostConstant};

use crate::libc::mach_thread_info::mach_port_t;

// Unique mock value so we can assert against itself
pub const MACH_TASK_SELF: mach_port_t = 0x7461736b;

pub const CONSTANTS: ConstantExports = &[(
    "_mach_task_self_",
    HostConstant::Custom(|mem| mem.alloc_and_write(MACH_TASK_SELF).cast_void().cast_const()),
)];
