/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Core mach types

#![allow(non_camel_case_types)]

use crate::mem::SafeRead;

pub type kern_return_t = i32;

pub type natural_t = u32;
pub type integer_t = i32;
pub type boolean_t = i32;

pub type vm_size_t = natural_t;

pub type policy_t = i32;

pub type mach_msg_type_name_t = natural_t;
pub type mach_msg_type_number_t = natural_t;

#[repr(C, packed)]
pub struct time_value_t {
    pub seconds: integer_t,
    pub microseconds: integer_t,
}
unsafe impl SafeRead for time_value_t {}

pub const KERN_SUCCESS: kern_return_t = 0;
