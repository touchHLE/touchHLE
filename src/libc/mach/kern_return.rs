/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `kern_return.h` and some other related functions

#![allow(non_camel_case_types)]

pub type kern_return_t = i32;

pub const KERN_SUCCESS: kern_return_t = 0;
