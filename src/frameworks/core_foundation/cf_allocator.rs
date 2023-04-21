/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFAllocator`. Currently there is no actual support for multiple allocators.

use super::CFTypeRef;
use crate::dyld::{ConstantExports, HostConstant};
use crate::mem::Ptr;

pub type CFAllocatorRef = CFTypeRef;

pub const kCFAllocatorDefault: CFAllocatorRef = Ptr::null();

pub const CONSTANTS: ConstantExports = &[("_kCFAllocatorDefault", HostConstant::NullPtr)];
