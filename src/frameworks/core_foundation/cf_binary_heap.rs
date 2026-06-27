/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFBinaryHeap` — Core Foundation priority queue.
//!
//! Real implementation backed by a Rust `Vec` because we cannot execute
//! the guest-provided comparison callback from host code in the general
//! case. We store values in insertion order and return the first element
//! as the "minimum". This is sufficient for simple heaps (e.g. timers
//! inserted in chronological order) and prevents NULL-deref crashes
//! when apps expect a working heap object.

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstVoidPtr, MutVoidPtr, Ptr};
use crate::Environment;
use std::collections::HashMap;

pub struct CFBinaryHeapState {
    next_id: u32,
    heaps: HashMap<u32, Vec<ConstVoidPtr>>,
}

impl Default for CFBinaryHeapState {
    fn default() -> Self {
        CFBinaryHeapState {
            next_id: 1,
            heaps: HashMap::new(),
        }
    }
}

impl CFBinaryHeapState {
    fn get(env: &mut Environment) -> &mut Self {
        // We attach the state to the CoreFoundation framework state.
        // Since there's no dedicated field, we use a lazy-static-like
        // approach via the environment's framework_state. For now
        // we store it in the core_foundation cf_dictionary state bag
        // as a hack, but a cleaner way is to add a field to
        // `core_foundation::State`. We'll do that instead.
        &mut env.framework_state.core_foundation.cf_binary_heap
    }
}

// --- C API ---

fn CFBinaryHeapCreate(
    env: &mut Environment,
    _allocator: MutVoidPtr,
    _capacity: i32,
    _call_backs: ConstVoidPtr,
    _compare_context: ConstVoidPtr,
) -> MutVoidPtr {
    let id = env.framework_state.core_foundation.cf_binary_heap.next_id;
    env.framework_state.core_foundation.cf_binary_heap.next_id += 1;
    env.framework_state
        .core_foundation
        .cf_binary_heap
        .heaps
        .insert(id, Vec::new());
    // Return the id as a "pointer" — we never dereference it, we just
    // key off the low bits. Use a guest-allocated sentinel so the
    // guest sees a non-NULL pointer.
    let ptr = env.mem.alloc(4).cast::<u32>();
    env.mem.write(ptr, id);
    ptr.cast_void()
}

fn CFBinaryHeapAddValue(env: &mut Environment, heap: MutVoidPtr, value: ConstVoidPtr) {
    if heap.is_null() {
        log!("Warning: CFBinaryHeapAddValue called with NULL heap");
        return;
    }
    let id: u32 = env.mem.read(heap.cast::<u32>());
    let Some(heap_vec) = env
        .framework_state
        .core_foundation
        .cf_binary_heap
        .heaps
        .get_mut(&id)
    else {
        log!("Warning: CFBinaryHeapAddValue called with invalid heap {:?}", heap);
        return;
    };
    heap_vec.push(value);
}

fn CFBinaryHeapGetCount(env: &mut Environment, heap: MutVoidPtr) -> i32 {
    if heap.is_null() {
        return 0;
    }
    let id: u32 = env.mem.read(heap.cast::<u32>());
    env.framework_state
        .core_foundation
        .cf_binary_heap
        .heaps
        .get(&id)
        .map(|v| v.len() as i32)
        .unwrap_or(0)
}

fn CFBinaryHeapGetMinimum(env: &mut Environment, heap: MutVoidPtr) -> ConstVoidPtr {
    if heap.is_null() {
        return Ptr::null();
    }
    let id: u32 = env.mem.read(heap.cast::<u32>());
    let Some(heap_vec) = env
        .framework_state
        .core_foundation
        .cf_binary_heap
        .heaps
        .get(&id)
    else {
        return Ptr::null();
    };
    heap_vec.first().copied().unwrap_or(Ptr::null())
}

fn CFBinaryHeapRemoveMinimum(env: &mut Environment, heap: MutVoidPtr) {
    if heap.is_null() {
        return;
    }
    let id: u32 = env.mem.read(heap.cast::<u32>());
    let Some(heap_vec) = env
        .framework_state
        .core_foundation
        .cf_binary_heap
        .heaps
        .get_mut(&id)
    else {
        return;
    };
    if !heap_vec.is_empty() {
        heap_vec.remove(0);
    }
}

fn CFBinaryHeapRemoveAllValues(env: &mut Environment, heap: MutVoidPtr) {
    if heap.is_null() {
        return;
    }
    let id: u32 = env.mem.read(heap.cast::<u32>());
    if let Some(heap_vec) = env
        .framework_state
        .core_foundation
        .cf_binary_heap
        .heaps
        .get_mut(&id)
    {
        heap_vec.clear();
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFBinaryHeapCreate(_, _, _, _)),
    export_c_func!(CFBinaryHeapAddValue(_, _)),
    export_c_func!(CFBinaryHeapGetCount(_)),
    export_c_func!(CFBinaryHeapGetMinimum(_)),
    export_c_func!(CFBinaryHeapRemoveMinimum(_)),
    export_c_func!(CFBinaryHeapRemoveAllValues(_)),
];
