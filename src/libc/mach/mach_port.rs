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
//! We would prefer to crash on exception anyway,
//! so it should be fine to just have stubs.

// TODO: implement port rights

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

fn mach_port_allocate(
    _env: &mut Environment,
    task: ipc_space_t,
    right: mach_port_right_t,
    name: MutPtr<mach_port_name_t>,
) -> kern_return_t {
    assert_eq!(task, MACH_TASK_SELF);
    log!(
        "TODO: mach_port_allocate({:#x}, {}, {:?})",
        task,
        right,
        name
    );
    KERN_SUCCESS
}

fn mach_port_deallocate(
    _env: &mut Environment,
    task: ipc_space_t,
    name: mach_port_name_t,
) -> kern_return_t {
    assert_eq!(task, MACH_TASK_SELF);
    log_dbg!("TODO: mach_port_deallocate({:#x}, {})", task, name);
    KERN_SUCCESS
}

fn mach_port_insert_right(
    _env: &mut Environment,
    task: ipc_space_t,
    name: mach_port_name_t,
    poly: mach_port_t,               // called `right` in some docs
    poly_poly: mach_msg_type_name_t, // called `right_type` in some docs
) -> kern_return_t {
    assert_eq!(task, MACH_TASK_SELF);
    log!(
        "TODO: mach_port_insert_right({:#x}, {}, {}, {})",
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
