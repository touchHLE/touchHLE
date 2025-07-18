/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `mach_vm.h` and some other related functions

#![allow(non_camel_case_types)]

use crate::libc::mach::types::natural_t;

pub type vm_size_t = natural_t;
pub const PAGE_SIZE: vm_size_t = 4096;
