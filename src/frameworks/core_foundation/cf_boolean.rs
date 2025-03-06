/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFBoolean`.

use super::CFTypeRef;
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::Environment;

/// CFBooleanRef is just an opaque pointer (CFTypeRef)
pub type CFBooleanRef = CFTypeRef;

/// Reads the boolean value stored at the CFBooleanRef pointer.
/// This is like dereferencing the constant true/false.
fn CFBooleanGetValue(env: &mut Environment, boolean: CFBooleanRef) -> bool {
    env.mem.read::<bool, true>(boolean.cast())
}

pub const CONSTANTS: ConstantExports = &[
    (
        "_kCFBooleanTrue",
        HostConstant::Custom(|mem, _| mem.alloc_and_write(true).cast_void().cast_const()),
    ),
    (
        "_kCFBooleanFalse",
        HostConstant::Custom(|mem, _| mem.alloc_and_write(false).cast_void().cast_const()),
    ),
];

pub const FUNCTIONS: FunctionExports = &[export_c_func!(CFBooleanGetValue(_))];
