/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFType` (type-generic functions etc).

use crate::dyld::{export_c_func, FunctionExports};
use crate::objc;
use crate::Environment;

pub type CFTypeRef = objc::id;

pub fn CFRetain(env: &mut Environment, object: CFTypeRef) -> CFTypeRef {
    assert!(!object.is_null()); // not allowed, unlike for normal objc objects
    objc::retain(env, object)
}
pub fn CFRelease(env: &mut Environment, object: CFTypeRef) {
    objc::release(env, object);
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(CFRetain(_)), export_c_func!(CFRelease(_))];
