/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFBoolean`.

use crate::abi::{impl_GuestRet_for_large_struct, GuestArg};
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::mem::SafeRead;
use crate::Environment;

#[derive(Debug, PartialEq)]
pub struct CFBoolean(bool);

impl From<&str> for CFBoolean {
    fn from(value: &str) -> Self {
        CFBoolean(matches!(value.to_lowercase().as_str(), "true"))
    }
}

impl From<i8> for CFBoolean {
    fn from(value: i8) -> Self {
        CFBoolean(value != 0) // Nonzero i8 values are true, 0 is false
    }
}

impl From<u8> for CFBoolean {
    fn from(value: u8) -> Self {
        CFBoolean(value != 0) // Nonzero u8 values are true, 0 is false
    }
}

unsafe impl SafeRead for CFBoolean {}

impl GuestArg for CFBoolean {
    const REG_COUNT: usize = 1;
    fn from_regs(regs: &[u32]) -> Self {
        CFBoolean(<u32 as GuestArg>::from_regs(regs) != 0)
    }
    fn to_regs(self, regs: &mut [u32]) {
        <u32 as GuestArg>::to_regs(self.0 as u32, regs)
    }
}
impl_GuestRet_for_large_struct!(CFBoolean);

pub const kCFBooleanTrue: CFBoolean = CFBoolean(true);
pub const kCFBooleanFalse: CFBoolean = CFBoolean(false);

fn CFBooleanGetValue(_env: &mut Environment, boolean: CFBoolean) -> u8 {
    match boolean {
        kCFBooleanTrue => 1,
        kCFBooleanFalse => 0,
    }
}

pub const CONSTANTS: ConstantExports = &[
    (
        "_kCFBooleanTrue",
        HostConstant::Custom(|mem, _dyld| {
            mem.alloc_and_write(kCFBooleanTrue).cast_void().cast_const()
        }),
    ),
    (
        "_kCFBooleanFalse",
        HostConstant::Custom(|mem, _dyld| {
            mem.alloc_and_write(kCFBooleanFalse)
                .cast_void()
                .cast_const()
        }),
    ),
];

pub const FUNCTIONS: FunctionExports = &[export_c_func!(CFBooleanGetValue(_))];
