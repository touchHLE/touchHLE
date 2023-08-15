/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `setjmp.h`.
//!
//! We don't have a real implementation for this right now. It could be quite
//! tricky to write one, considering that we would need to unwind through host
//! code, and somehow do so selectively since we have a mix of stack frames from
//! different guest threads. For the moment, we simply pray the app never throws
//! exceptions.
//!
//! Note that `setjmp` and `longjmp` are defined as macros in the C standard,
//! but it seems like the implementation of these on iPhone OS uses real
//! functions, at least for the former.

use crate::dyld::{export_c_func, FunctionExports};
use crate::Environment;

/// The signature of this is incomplete because it's a stub (see module docs).
fn setjmp(env: &mut Environment) -> i32 {
    log_dbg!(
        "TODO: setjmp() at {:#x}",
        env.cpu.regs()[crate::cpu::Cpu::LR]
    );
    0 // no longjmp() was performed
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(setjmp())];
