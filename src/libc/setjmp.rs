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

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{MutPtr, SafeRead};
use crate::{abi, Environment};

#[repr(C, packed)]
#[derive(Debug)]
struct JmpBuf {
    r4: u32,
    r5: u32,
    r6: u32,
    fp: u32,
    r8: u32,
    r10: u32,
    r11: u32,
    sp: u32,
    lr: u32,
}

unsafe impl SafeRead for JmpBuf {}

/// The signature of this is incomplete because it's a stub (see module docs).
fn setjmp(env: &mut Environment, jmp_buf: MutPtr<JmpBuf>) -> i32 {
    let lr = env.cpu.regs()[crate::cpu::Cpu::LR];
    log_dbg!("TODO: setjmp() at {:#x}", lr);
    let buf = JmpBuf {
        r4: env.cpu.regs()[4],
        r5: env.cpu.regs()[5],
        r6: env.cpu.regs()[6],
        fp: env.cpu.regs()[abi::FRAME_POINTER],
        r8: env.cpu.regs()[8],
        r10: env.cpu.regs()[10],
        r11: env.cpu.regs()[11],
        sp: env.cpu.regs()[crate::cpu::Cpu::SP],
        lr: env.cpu.regs()[crate::cpu::Cpu::LR],
    };
    env.mem.write(jmp_buf, buf);
    0 // no longjmp() was performed
}

fn longjmp(env: &mut Environment, jmp_buf: MutPtr<JmpBuf>, status: u32) {
    let lr = env.cpu.regs()[crate::cpu::Cpu::LR];
    let fp = env.cpu.regs()[abi::FRAME_POINTER];

    let buf = env.mem.read(jmp_buf);
    let cur_stack = env.stack_for_longjmp(lr, fp);
    let other_stack = env.stack_for_longjmp(buf.lr, buf.fp);
    if cur_stack.last() != other_stack.last() {
        log_dbg!(
            "Warning: tolerating a longjump across host stack frames, current {:?}, other {:?}",
            cur_stack,
            other_stack
        );
    }
    env.cpu.regs_mut()[0] = status;
    env.cpu.regs_mut()[4] = buf.r4;
    env.cpu.regs_mut()[5] = buf.r5;
    env.cpu.regs_mut()[6] = buf.r6;
    env.cpu.regs_mut()[abi::FRAME_POINTER] = buf.fp;
    env.cpu.regs_mut()[8] = buf.r8;
    env.cpu.regs_mut()[10] = buf.r10;
    env.cpu.regs_mut()[11] = buf.r11;
    env.cpu.regs_mut()[crate::cpu::Cpu::SP] = buf.sp;
    env.cpu.regs_mut()[crate::cpu::Cpu::LR] = buf.lr;
    env.cpu
        .branch(GuestFunction::from_addr_with_thumb_bit(buf.lr));
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(setjmp(_)), export_c_func!(longjmp(_, _))];
