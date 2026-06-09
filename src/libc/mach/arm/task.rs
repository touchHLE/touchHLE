/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Mach task functions for ARM arch.

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::mach::core_types::natural_t;
use crate::libc::mach::init::MACH_TASK_SELF;
use crate::libc::mach::port::mach_port_t;
use crate::libc::mach::thread_info::{kern_return_t, thread_state_flavor_t, KERN_SUCCESS};
use crate::libc::mach::vm_map::vm_allocate;
use crate::mem::{guest_size_of, GuestUSize, MutPtr};
use crate::Environment;

pub type task_t = mach_port_t;

type thread_act_t = mach_port_t;
type thread_act_array_t = MutPtr<thread_act_t>;

type mach_msg_type_number_t = natural_t;

type exception_mask_t = u32;
type exception_behavior_t = i32;

fn task_threads(
    env: &mut Environment,
    task: task_t,
    thread_list: MutPtr<thread_act_array_t>,
    thread_count_: MutPtr<mach_msg_type_number_t>,
) -> kern_return_t {
    assert_eq!(task, MACH_TASK_SELF);
    let thread_count = env.threads.len() as GuestUSize;
    // It is not explicitly stated that vm_allocate() should be used,
    // but some doc says that the caller `may wish` to free resulted
    // array with vm_deallocate()
    let res = vm_allocate(
        env,
        task,
        thread_list.cast(),
        thread_count * guest_size_of::<thread_act_t>(),
        1, // TRUE
    );
    assert_eq!(res, KERN_SUCCESS);
    let arr: MutPtr<thread_act_t> = env.mem.read(thread_list.cast());
    for i in 0..thread_count {
        // TODO: implement port rights
        // For now, use thread id + 1
        // (Plus 1 is to avoid having MACH_PORT_NULL for the main thread)
        env.mem.write(arr + i, i + 1);
    }
    env.mem.write(thread_count_, thread_count);
    KERN_SUCCESS
}

// Our internal type, Mach just uses int.
type MachExceptionType = i32;
const EXC_BAD_ACCESS: MachExceptionType = 1;

// Our internal type, Mach just uses unsigned int.
type MachExceptionMaskType = u32;
const EXC_MASK_BAD_ACCESS: MachExceptionMaskType = 1 << EXC_BAD_ACCESS;

// Our internal type, Mach just uses int.
type MachExceptionBehaviourType = i32;
const EXCEPTION_DEFAULT: MachExceptionBehaviourType = 1;

fn task_set_exception_ports(
    _env: &mut Environment,
    task: task_t,
    exception_mask: exception_mask_t,
    new_port: mach_port_t,
    behavior: exception_behavior_t,
    new_flavor: thread_state_flavor_t,
) -> kern_return_t {
    assert_eq!(task, MACH_TASK_SELF);
    assert_eq!(exception_mask, EXC_MASK_BAD_ACCESS);
    assert_eq!(behavior, EXCEPTION_DEFAULT);
    // This function is used by Unity to install an `exception handler`.
    // (See mono's [mini-darwin.c](https://github.com/mono/mono/blob/62121afbb28f0b62f100ec9a942d10c5e0f4814f/mono/mini/mini-darwin.c#L188))
    // We would prefer to crash on exception anyway,
    // so it should be fine to just have a stub.
    log!(
        "TODO: task_set_exception_ports({:#x}, {}, {}, {}, {})",
        task,
        exception_mask,
        new_port,
        behavior,
        new_flavor
    );
    KERN_SUCCESS
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(task_threads(_, _, _)),
    export_c_func!(task_set_exception_ports(_, _, _, _, _)),
];
