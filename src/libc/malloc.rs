/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `malloc.h` memory management zones

use std::{cell::OnceCell, collections::HashMap};

use crate::{
    dyld::FunctionExports,
    environment::Environment,
    export_c_func,
    mem::{AllocatorID, ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr, SafeRead},
};

#[derive(Default)]
pub struct MallocZones {
    default_zone: OnceCell<MutPtr<malloc_zone_t>>,
    zone_to_allocator: HashMap<MutPtr<malloc_zone_t>, AllocatorID>,
}

#[repr(C, packed)]
#[allow(non_camel_case_types)]
pub struct malloc_zone_t {
    reserved1: MutVoidPtr,
    reserved2: MutVoidPtr,
    size: MutVoidPtr,
    malloc: MutVoidPtr,
    calloc: MutVoidPtr,
    valloc: MutVoidPtr,
    free: MutVoidPtr,
    realloc: MutVoidPtr,
    destroy: MutVoidPtr,
    zone_name: ConstPtr<u8>,
    batch_malloc: MutVoidPtr,
    batch_free: MutVoidPtr,
    introspect: MutVoidPtr,
    version: u32,
    memalign: MutVoidPtr,
}
unsafe impl SafeRead for malloc_zone_t {}

impl malloc_zone_t {
    pub fn new(env: &mut Environment) -> malloc_zone_t {
        malloc_zone_t {
            reserved1: Ptr::null(),
            reserved2: Ptr::null(),
            size: Self::get_function_address(env, "malloc_zone_size"),
            malloc: Self::get_function_address(env, "malloc_zone_malloc"),
            calloc: Ptr::null(),
            valloc: Ptr::null(),
            free: Self::get_function_address(env, "malloc_zone_free"),
            realloc: Self::get_function_address(env, "malloc_zone_realloc"),
            destroy: Self::get_function_address(env, "malloc_destroy_zone"),
            zone_name: Ptr::null(),
            batch_malloc: Ptr::null(),
            batch_free: Ptr::null(),
            introspect: Ptr::null(),
            version: 0,
            memalign: Ptr::null(),
        }
    }

    fn get_function_address(env: &mut Environment, function_name: &str) -> MutVoidPtr {
        // TODO: Should this be moved to dyld instead?
        let function_name = format!("_{function_name}");
        let address = env
            .dyld
            .create_proc_address(&mut env.mem, &mut env.cpu, &function_name)
            .unwrap_or_else(|_| {
                panic!("Attempted to get address of unimplemented function: {function_name}")
            });
        Ptr::from_bits(address.addr_with_thumb_bit())
    }
}

fn malloc_default_zone(env: &mut Environment) -> MutPtr<malloc_zone_t> {
    if env.malloc_zones.default_zone.get().is_none() {
        let zone_data = malloc_zone_t::new(env);
        let zone = env.mem.alloc_and_write(zone_data);
        env.malloc_zones.default_zone.set(zone).unwrap();
        assert!(env
            .malloc_zones
            .zone_to_allocator
            .insert(zone, env.mem.default_allocator())
            .is_none());
    }

    *env.malloc_zones.default_zone.get().unwrap()
}

fn malloc_create_zone(
    env: &mut Environment,
    start_size: GuestUSize,
    _flags: u32,
) -> MutPtr<malloc_zone_t> {
    let zone_data = malloc_zone_t::new(env);
    let zone = env.mem.alloc_and_write(zone_data);
    let allocator = env.mem.create_allocator(start_size);
    assert!(env
        .malloc_zones
        .zone_to_allocator
        .insert(zone, allocator)
        .is_none());
    zone
}

fn malloc_destroy_zone(env: &mut Environment, zone: MutPtr<malloc_zone_t>) {
    env.mem.free(zone.cast());
    let allocator = env
        .malloc_zones
        .zone_to_allocator
        .remove(&zone)
        .expect("Zone {zone:?} does not map to an allocator");
    env.mem.destroy_allocator(allocator);
}

fn malloc_zone_free(env: &mut Environment, zone: MutPtr<malloc_zone_t>, ptr: MutVoidPtr) {
    let allocator = get_allocator(env, zone);
    env.mem.free_in(allocator, ptr);
}

fn malloc_zone_malloc(
    env: &mut Environment,
    zone: MutPtr<malloc_zone_t>,
    size: GuestUSize,
) -> MutVoidPtr {
    let allocator = get_allocator(env, zone);
    env.mem.alloc_in(allocator, size)
}

fn malloc_zone_realloc(
    env: &mut Environment,
    zone: MutPtr<malloc_zone_t>,
    ptr: MutVoidPtr,
    size: GuestUSize,
) -> MutVoidPtr {
    let allocator = get_allocator(env, zone);

    if ptr.is_null() {
        return malloc_zone_malloc(env, zone, size);
    }
    env.mem.realloc_in(allocator, ptr, size)
}

fn malloc_zone_size(
    env: &mut Environment,
    zone: MutPtr<malloc_zone_t>,
    ptr: ConstVoidPtr,
) -> GuestUSize {
    let allocator = get_allocator(env, zone);
    env.mem.malloc_size_in(allocator, ptr)
}

fn get_allocator(env: &Environment, zone: MutPtr<malloc_zone_t>) -> AllocatorID {
    *env.malloc_zones
        .zone_to_allocator
        .get(&zone)
        .expect("Zone {zone:?} does not map to an allocator")
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(malloc_create_zone(_, _)),
    export_c_func!(malloc_default_zone()),
    export_c_func!(malloc_destroy_zone(_)),
    export_c_func!(malloc_zone_free(_, _)),
    export_c_func!(malloc_zone_malloc(_, _)),
    export_c_func!(malloc_zone_realloc(_, _, _)),
    export_c_func!(malloc_zone_size(_, _)),
];
