/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Mach thread actions for ARM arch.

use crate::dyld::{export_c_func, FunctionExports};
use crate::environment::ThreadBlock;
use crate::libc::mach::core_types::integer_t;
use crate::libc::mach::port::{mach_port_t, MACH_PORT_DEAD, MACH_PORT_NULL};
use crate::libc::mach::thread_info::{
    kern_return_t, mach_msg_type_number_t, thread_inspect_t, thread_state_flavor_t, thread_state_t,
    KERN_SUCCESS,
};
use crate::mem::{guest_size_of, MutPtr, SafeRead};
use crate::{Environment, ThreadId};

type thread_act_t = mach_port_t;

const ARM_THREAD_STATE: thread_state_flavor_t = 1;
const MACHINE_THREAD_STATE: thread_state_flavor_t = ARM_THREAD_STATE;

#[repr(C, packed)]
struct arm_thread_state {
    r: [u32; 13], // General purpose registers
    sp: u32,      // Stack pointer
    lr: u32,      // Link register
    pc: u32,      // Program counter
    cpsr: u32,    // Current program status register
}
unsafe impl SafeRead for arm_thread_state {}

fn thread_suspend(env: &mut Environment, target_thread: thread_inspect_t) -> kern_return_t {
    assert!(target_thread != MACH_PORT_NULL && target_thread != MACH_PORT_DEAD);
    // Expected `thread send right` is thread_id + 1. See `mach_thread_self()`
    env.suspend_thread((target_thread - 1) as ThreadId);
    KERN_SUCCESS
}

fn thread_resume(env: &mut Environment, target_thread: thread_inspect_t) -> kern_return_t {
    assert!(target_thread != MACH_PORT_NULL && target_thread != MACH_PORT_DEAD);
    // Expected `thread send right` is thread_id + 1. See `mach_thread_self()`
    env.resume_thread((target_thread - 1) as ThreadId);
    KERN_SUCCESS
}

fn thread_get_state(
    env: &mut Environment,
    target_thread: thread_act_t,
    flavor: thread_state_flavor_t,
    old_state: thread_state_t,
    old_state_count: MutPtr<mach_msg_type_number_t>,
) -> kern_return_t {
    assert!(target_thread != MACH_PORT_NULL && target_thread != MACH_PORT_DEAD);
    assert_eq!(flavor, MACHINE_THREAD_STATE);

    let out_size_available = env.mem.read(old_state_count);
    let out_size_expected = guest_size_of::<arm_thread_state>() / guest_size_of::<integer_t>();
    assert!(out_size_expected <= out_size_available);

    // Expected `thread send right` is thread_id + 1. See `mach_thread_self()`
    let thread_id = (target_thread - 1) as ThreadId;
    // TODO: what happen if thread_get_state() is called on unsuspended thread?
    assert!(matches!(
        env.threads[thread_id].blocked_by,
        ThreadBlock::Suspended(_, _)
    ));
    let ctx = env.threads[thread_id].guest_context.as_ref().unwrap();
    let state = arm_thread_state {
        r: ctx.regs[..13].try_into().unwrap(),
        sp: ctx.regs[crate::cpu::Cpu::SP],
        lr: ctx.regs[crate::cpu::Cpu::LR],
        pc: ctx.regs[crate::cpu::Cpu::PC],
        cpsr: ctx.cpsr,
    };
    env.mem.write(old_state.cast(), state);
    env.mem.write(old_state_count, out_size_expected);

    KERN_SUCCESS
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(thread_suspend(_)),
    export_c_func!(thread_resume(_)),
    export_c_func!(thread_get_state(_, _, _, _)),
];
