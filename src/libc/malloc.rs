/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `malloc.h` memory management zones

use std::collections::HashMap;

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::libc::string::strdup;
use crate::mem::{
    ConstPtr, ConstVoidPtr, GuestUSize, HeapAllocator, Mem, MutPtr, MutVoidPtr, Ptr, SafeRead,
};

#[derive(Default)]
pub struct State {
    default_zone: Option<MutPtr<malloc_zone_t>>,
    zone_to_heap: HashMap<MutPtr<malloc_zone_t>, HeapAllocator>,
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
            size: env
                .dyld
                .create_function_address(&mut env.mem, &mut env.cpu, "malloc_zone_size")
                .unwrap(),
            malloc: env
                .dyld
                .create_function_address(&mut env.mem, &mut env.cpu, "malloc_zone_malloc")
                .unwrap(),
            calloc: Ptr::null(),
            valloc: Ptr::null(),
            free: env
                .dyld
                .create_function_address(&mut env.mem, &mut env.cpu, "malloc_zone_free")
                .unwrap(),
            realloc: env
                .dyld
                .create_function_address(&mut env.mem, &mut env.cpu, "malloc_zone_realloc")
                .unwrap(),
            destroy: env
                .dyld
                .create_function_address(&mut env.mem, &mut env.cpu, "malloc_destroy_zone")
                .unwrap(),
            zone_name: Ptr::null(),
            batch_malloc: Ptr::null(),
            batch_free: Ptr::null(),
            introspect: Ptr::null(),
            version: 0,
            memalign: Ptr::null(),
        }
    }
}

fn malloc_default_zone(env: &mut Environment) -> MutPtr<malloc_zone_t> {
    if env.libc_state.malloc.default_zone.is_none() {
        let zone_data = malloc_zone_t::new(env);
        let zone = env.mem.alloc_and_write(zone_data);
        env.libc_state.malloc.default_zone = Some(zone);
    }

    env.libc_state.malloc.default_zone.unwrap()
}

fn malloc_create_zone(
    env: &mut Environment,
    start_size: GuestUSize,
    flags: u32,
) -> MutPtr<malloc_zone_t> {
    assert_eq!(flags, 0);
    let zone_data = malloc_zone_t::new(env);
    let zone = env.mem.alloc_and_write(zone_data);
    let heap = env.mem.create_heap(start_size);
    assert!(env
        .libc_state
        .malloc
        .zone_to_heap
        .insert(zone, heap)
        .is_none());
    zone
}

fn malloc_destroy_zone(env: &mut Environment, zone: MutPtr<malloc_zone_t>) {
    if zone == malloc_default_zone(env) {
        panic!("Attempted to destroy default zone");
    } else {
        let heap = env
            .libc_state
            .malloc
            .zone_to_heap
            .remove(&zone)
            .unwrap_or_else(|| panic!("Zone {zone:?} does not map to an allocator"));
        env.mem.destroy_heap(heap);
        let zone_name = env.mem.read(zone).zone_name;
        if !zone_name.is_null() {
            env.mem.free(zone_name.cast_mut().cast());
        }
        env.mem.free(zone.cast());
    }
}

fn malloc_zone_free(env: &mut Environment, zone: MutPtr<malloc_zone_t>, ptr: MutVoidPtr) {
    with_zone(env, zone, |mem, heap| mem.free_in_heap(heap, ptr))
}

fn malloc_zone_malloc(
    env: &mut Environment,
    zone: MutPtr<malloc_zone_t>,
    size: GuestUSize,
) -> MutVoidPtr {
    with_zone(env, zone, |mem, heap| mem.alloc_in_heap(heap, size))
}

fn malloc_zone_realloc(
    env: &mut Environment,
    zone: MutPtr<malloc_zone_t>,
    ptr: MutVoidPtr,
    size: GuestUSize,
) -> MutVoidPtr {
    with_zone(env, zone, |mem, heap| mem.realloc_in_heap(heap, ptr, size))
}

fn malloc_zone_size(
    env: &mut Environment,
    zone: MutPtr<malloc_zone_t>,
    ptr: ConstVoidPtr,
) -> GuestUSize {
    with_zone(env, zone, |mem, heap| mem.malloc_size_in_heap(heap, ptr))
}

/// Not a part of the API. However as such a function needs to be accessible in
/// the zone struct, it has to be exported.
fn malloc_set_zone_name(env: &mut Environment, zone: MutPtr<malloc_zone_t>, name: ConstPtr<u8>) {
    let name = strdup(env, name).cast_const();
    let mut zone_data = env.mem.read(zone);
    zone_data.zone_name = name;
    env.mem.write(zone, zone_data);
}

/// Call a function using the heap corresponding to `zone`. Zone helper
/// not a part of the API.
fn with_zone<R>(
    env: &mut Environment,
    zone: MutPtr<malloc_zone_t>,
    f: impl FnOnce(&mut Mem, Option<&mut HeapAllocator>) -> R,
) -> R {
    let default = malloc_default_zone(env);
    let heap = if zone == default {
        None
    } else {
        Some(
            env.libc_state
                .malloc
                .zone_to_heap
                .get_mut(&zone)
                .unwrap_or_else(|| panic!("Zone {zone:?} does not map to an allocator")),
        )
    };
    f(&mut env.mem, heap)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(malloc_create_zone(_, _)),
    export_c_func!(malloc_default_zone()),
    export_c_func!(malloc_destroy_zone(_)),
    export_c_func!(malloc_zone_free(_, _)),
    export_c_func!(malloc_zone_malloc(_, _)),
    export_c_func!(malloc_zone_realloc(_, _, _)),
    export_c_func!(malloc_zone_size(_, _)),
    export_c_func!(malloc_set_zone_name(_, _)),
];
