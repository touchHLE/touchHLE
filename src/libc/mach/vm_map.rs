/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Mach VM functions

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::mach::init::MACH_TASK_SELF;
use crate::libc::mach::port::mach_port_t;
use crate::libc::mach::thread_info::{kern_return_t, KERN_SUCCESS};
use crate::mem::{MutPtr, Ptr, PAGE_SIZE_ALIGN_MASK};
use crate::Environment;
use std::collections::HashMap;

type vm_map_t = mach_port_t;
type vm_purgable_t = i32;
type mach_vm_address_t = u32;
type mach_vm_size_t = u32;

const VM_FLAGS_ANYWHERE: i32 = 0x1;

#[derive(Default)]
pub struct State {
    /// Keeping track of `vm_allocate` allocations
    allocations: HashMap<mach_vm_address_t, mach_vm_size_t>,
}

pub fn vm_allocate(
    env: &mut Environment,
    target_task: vm_map_t,
    address_ptr: MutPtr<mach_vm_address_t>,
    size: mach_vm_size_t,
    flags: i32,
) -> kern_return_t {
    assert_eq!(target_task, MACH_TASK_SELF);
    // TODO: support more flags, this list is not complete
    assert_eq!(flags & !VM_FLAGS_ANYWHERE, 0);

    let address = (flags & VM_FLAGS_ANYWHERE == 0).then(|| env.mem.read(address_ptr));

    let allocated = env.mem.vm_alloc(address, size).unwrap();
    let address = allocated.to_bits();
    assert!(address & PAGE_SIZE_ALIGN_MASK == 0);
    env.mem.write(address_ptr, address);

    assert!(!env.libc_state.mach_vm.allocations.contains_key(&address));
    // Note: we keep track of the original size,
    // not the one what was actually allocated!
    env.libc_state.mach_vm.allocations.insert(address, size);

    KERN_SUCCESS
}

fn vm_deallocate(
    env: &mut Environment,
    target_task: vm_map_t,
    address: mach_vm_address_t,
    size: mach_vm_size_t,
) -> kern_return_t {
    assert_eq!(target_task, MACH_TASK_SELF);

    assert_eq!(
        *env.libc_state.mach_vm.allocations.get(&address).unwrap(),
        size
    );
    env.mem.vm_free(Ptr::from_bits(address), size);
    env.libc_state.mach_vm.allocations.remove(&address);

    KERN_SUCCESS
}

fn vm_purgable_control(
    _env: &mut Environment,
    target_task: vm_map_t,
    address: mach_vm_address_t,
    control: vm_purgable_t,
    state: MutPtr<vm_purgable_t>,
) -> kern_return_t {
    assert_eq!(target_task, MACH_TASK_SELF);
    log!("TODO: vm_purgable_control({target_task:#x}, {address:#x}, {control:#x}, {state:?})");
    KERN_SUCCESS
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(vm_allocate(_, _, _, _)),
    export_c_func!(vm_deallocate(_, _, _)),
    export_c_func!(vm_purgable_control(_, _, _, _)),
];
