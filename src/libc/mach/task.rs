/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `task.h` and some other related functions

#![allow(non_camel_case_types)]

use crate::{libc::mach::port::mach_port_t, mem::MutPtr};

type task = std::ffi::c_void;
pub type task_t = MutPtr<task>;

// Unique mock value so we can assert against itself
pub const MACH_TASK_SELF: mach_port_t = 0x7461736b;
