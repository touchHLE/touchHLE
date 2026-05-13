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
use bitflags::bitflags;
use std::collections::HashMap;

type vm_map_t = mach_port_t;
type mach_vm_address_t = u32;
type mach_vm_size_t = u32;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct VmFlags : i32 {
        const ANYWHERE = 0x1;
        const PURGABLE = 0x2;
    }
}

const VM_TAG_MASK: i32 = 0xFF00_0000u32 as i32;

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
    const HANDLED_FLAGS: VmFlags = VmFlags::ANYWHERE;

    let tag = (flags >> 24) as u8;
    let flags = VmFlags::from_bits(flags & !VM_TAG_MASK).unwrap();

    let unhandled = flags.difference(HANDLED_FLAGS);
    if !unhandled.is_empty() {
        log!("Warning: vm_allocate ignoring flags {unhandled:?}");
    }

    if tag != 0 {
        log_dbg!("Ignoring vm_allocate tag {tag}");
    }

    let address = (!flags.contains(VmFlags::ANYWHERE)).then(|| env.mem.read(address_ptr));

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

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(vm_allocate(_, _, _, _)),
    export_c_func!(vm_deallocate(_, _, _)),
];
