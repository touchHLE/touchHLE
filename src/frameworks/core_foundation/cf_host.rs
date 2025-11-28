/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFHost`. Currently there is no actual support for this type.

use super::cf_string::CFStringRef;
use super::CFTypeRef;
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::Ptr;
use crate::Environment;

fn CFHostCreateWithName(
    _env: &mut Environment,
    _allocator: CFTypeRef,
    _host_name: CFStringRef,
) -> CFTypeRef {
    println!("CFHostCreateWithName called: returning stubbed null pointer");
    Ptr::null()
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(CFHostCreateWithName(_, _))];
