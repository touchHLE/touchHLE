/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `mach_port.h`
//!
//! Right now we do not implement port rights, but some early Unity
//! based games would install an `exception handler` using below functions.
//! (See [mini-darwin.c](https://github.com/mono/mono/blob/62121afbb28f0b62f100ec9a942d10c5e0f4814f/mono/mini/mini-darwin.c#L171) from mono repo)
//!
//! We emulate minimal port allocation so that callers get valid port
//! names back and don't try to free garbage values later.

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::mach::core_types::natural_t;
use crate::libc::mach::init::MACH_TASK_SELF;
use crate::libc::mach::port::mach_port_t;
use crate::libc::mach::thread_info::{kern_return_t, KERN_SUCCESS};
use crate::mem::MutPtr;
use crate::Environment;

type ipc_space_t = mach_port_t;
type mach_port_name_t = natural_t;
type mach_port_right_t = natural_t;

type mach_msg_type_name_t = u32;

/// Counter for generating unique synthetic mach port names.
/// Real Darwin starts port names at low values and increments by 4 (they are
/// indices into a port table). We start at 0x100 to avoid collision with any
/// well-known port constants the guest might check.
static mut NEXT_PORT_NAME: mach_port_name_t = 0x100;

fn mach_port_allocate(
    env: &mut Environment,
    task: ipc_space_t,
    right: mach_port_right_t,
    name: MutPtr<mach_port_name_t>,
) -> kern_return_t {
    if task != MACH_TASK_SELF {
        log!(
            "Warning: mach_port_allocate called with unexpected task {:#x} (expected {:#x})",
            task,
            MACH_TASK_SELF
        );
    }

    // Generate a unique port name and write it to the output parameter
    let port_name = unsafe {
        let n = NEXT_PORT_NAME;
        NEXT_PORT_NAME += 4; // Darwin increments by 4
        n
    };

    if !name.is_null() {
        env.mem.write(name, port_name);
    }

    log_dbg!(
        "mach_port_allocate(task={:#x}, right={}, name={:?}) => port_name={}",
        task,
        right,
        name,
        port_name
    );

    KERN_SUCCESS
}

fn mach_port_deallocate(
    _env: &mut Environment,
    task: ipc_space_t,
    name: mach_port_name_t,
) -> kern_return_t {
    if task != MACH_TASK_SELF {
        log!(
            "Warning: mach_port_deallocate called with unexpected task {:#x}",
            task
        );
    }
    log_dbg!("mach_port_deallocate(task={:#x}, name={})", task, name);
    KERN_SUCCESS
}

fn mach_port_insert_right(
    _env: &mut Environment,
    task: ipc_space_t,
    name: mach_port_name_t,
    poly: mach_port_t,
    poly_poly: mach_msg_type_name_t,
) -> kern_return_t {
    if task != MACH_TASK_SELF {
        log!(
            "Warning: mach_port_insert_right called with unexpected task {:#x}",
            task
        );
    }
    log_dbg!(
        "mach_port_insert_right(task={:#x}, name={}, poly={}, poly_poly={})",
        task,
        name,
        poly,
        poly_poly
    );
    KERN_SUCCESS
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(mach_port_allocate(_, _, _)),
    export_c_func!(mach_port_deallocate(_, _)),
    export_c_func!(mach_port_insert_right(_, _, _, _)),
];
