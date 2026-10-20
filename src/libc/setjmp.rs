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
use crate::{abi, Environment, ThreadId};
use std::collections::HashMap;

#[derive(Default)]
pub struct State {
    contexts: HashMap<ThreadId, MutPtr<u32>>,
}

impl State {
    fn top(&self, thread: ThreadId) -> MutPtr<u32> {
        self.contexts.get(&thread).copied().unwrap_or_default()
    }

    fn set_top(&mut self, thread: ThreadId, context: MutPtr<u32>) {
        if context.is_null() {
            self.contexts.remove(&thread);
        } else {
            self.contexts.insert(thread, context);
        }
    }
}

// Below apps are Fuse powered ones, this engine is using setjmp/longjmp for
// a custom "fibers" implementation. It _does not_ jump across host stack
// frames, but we do not have a good way to detect that right now.
// Thus, those are exempted from the check.
// You need to have a good reason to extend this list!
const ALLOWED_FOR_LONGJMP_BYPASS: [&str; 5] = [
    "com.activision.CBNK2",
    "com.ea.fifa10.bv",
    "com.ea.fifa10inc",
    "com.ea.fifa10wc.bv",
    "com.ea.fifa10wc.inc",
];

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
    if cur_stack.last() != other_stack.last()
        && !ALLOWED_FOR_LONGJMP_BYPASS.contains(&env.bundle.bundle_identifier())
    {
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

#[allow(non_snake_case)]
fn _Unwind_SjLj_Register(env: &mut Environment, context: MutPtr<u32>) {
    assert!(!context.is_null());
    let previous = env.libc_state.setjmp.top(env.current_thread);
    env.mem.write(context, previous.to_bits());
    env.libc_state.setjmp.set_top(env.current_thread, context);
}

#[allow(non_snake_case)]
fn _Unwind_SjLj_Unregister(env: &mut Environment, context: MutPtr<u32>) {
    assert_eq!(env.libc_state.setjmp.top(env.current_thread), context);
    let previous = env.mem.read(context);
    env.libc_state
        .setjmp
        .set_top(env.current_thread, MutPtr::from_bits(previous));
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(setjmp(_)),
    export_c_func!(longjmp(_, _)),
    export_c_func!(_Unwind_SjLj_Register(_)),
    export_c_func!(_Unwind_SjLj_Unregister(_)),
];

#[cfg(test)]
mod tests {
    use super::State;
    use crate::mem::MutPtr;

    #[test]
    fn unwind_context_stacks_are_thread_local() {
        let mut state = State::default();
        let first = MutPtr::<u32>::from_bits(0x1000);
        let second = MutPtr::<u32>::from_bits(0x2000);
        assert!(state.top(0).is_null());
        assert!(state.top(1).is_null());
        state.set_top(0, first);
        state.set_top(1, second);
        assert_eq!(state.top(0), first);
        assert_eq!(state.top(1), second);
        state.set_top(0, MutPtr::null());
        assert!(state.top(0).is_null());
        assert_eq!(state.top(1), second);
    }
}
