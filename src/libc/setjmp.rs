/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `setjmp.h`.
//!
//! Note that `setjmp` and `longjmp` are defined as macros in the C standard,
//! but it seems like the implementation of these on iPhone OS uses real
//! functions.

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

fn setjmp(env: &mut Environment, jmp_buf: MutPtr<JmpBuf>) -> i32 {
    let regs = env.cpu.regs();
    let lr = regs[crate::cpu::Cpu::LR];
    log_dbg!("setjmp() at {:#x}", lr);
    let buf = JmpBuf {
        r4: regs[4],
        r5: regs[5],
        r6: regs[6],
        fp: regs[abi::FRAME_POINTER],
        r8: regs[8],
        r10: regs[10],
        r11: regs[11],
        sp: regs[crate::cpu::Cpu::SP],
        lr: regs[crate::cpu::Cpu::LR],
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
        panic!("longjmp across host stack frames, current {cur_stack:?}, other {other_stack:?}");
    }
    let regs = env.cpu.regs_mut();
    regs[0] = status;
    regs[4] = buf.r4;
    regs[5] = buf.r5;
    regs[6] = buf.r6;
    regs[abi::FRAME_POINTER] = buf.fp;
    regs[8] = buf.r8;
    regs[10] = buf.r10;
    regs[11] = buf.r11;
    regs[crate::cpu::Cpu::SP] = buf.sp;
    regs[crate::cpu::Cpu::LR] = buf.lr;
    env.cpu
        .branch(GuestFunction::from_addr_with_thumb_bit(buf.lr));
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(setjmp(_)), export_c_func!(longjmp(_, _))];
