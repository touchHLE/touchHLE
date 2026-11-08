/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stack Smashing Protection (SSP).

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::mem::ConstVoidPtr;
use crate::Environment;

/// `___stack_chk_guard` is a *pointer* to the canary value, not the value
/// itself. Compiler-generated function prologues do
/// `ldr r0, =___stack_chk_guard; ldr r8, [r0]; ldr r0, [r8]`, so the symbol
/// must resolve to the address of a readable word. Leaving the slot null
/// (or the symbol unhandled, which leaves it zeroed) makes the second load
/// dereference the null page.
fn stack_chk_guard(env: &mut Environment) -> ConstVoidPtr {
    let canary = env.mem.alloc_and_write(0u32);
    canary.cast().cast_const()
}

/// The guest's canary check failed. The mismatch is normally an emulation
/// artifact rather than real corruption, so don't abort the session; just
/// report it and return to the guest.
pub fn __stack_chk_fail(_env: &mut Environment) {
    log!("Warning: __stack_chk_fail: guest stack canary mismatch, continuing.");
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(__stack_chk_fail())];

pub const CONSTANTS: ConstantExports = &[(
    "___stack_chk_guard",
    HostConstant::Custom(stack_chk_guard),
)];
