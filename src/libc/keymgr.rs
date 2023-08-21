/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `keymgr.h` (KeyMgr).
//!
//! KeyMgr's only documentation seems to be in its [public source code](https://github.com/apple-opensource/keymgr).

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{MutPtr, MutVoidPtr, Ptr};
use crate::{Environment, ThreadId};
use std::collections::hash_map::{Entry, HashMap};

#[derive(Default)]
pub struct State {
    processwide_ptrs: HashMap<i32, (MutVoidPtr, Option<ThreadId>)>,
}

fn get_and_lock_processwide_ptr_inner(env: &mut Environment, key: i32) -> Result<MutVoidPtr, i32> {
    match env.libc_state.keymgr.processwide_ptrs.entry(key) {
        Entry::Vacant(entry) => {
            entry.insert((Ptr::null(), Some(env.current_thread)));
            Ok(Ptr::null())
        }
        Entry::Occupied(mut entry) => {
            let entry = entry.get_mut();

            // TODO: waiting to unlock. This should share code with whatever
            // solution we eventually pick for mutexes.
            assert!(entry.1.is_none());

            entry.1 = Some(env.current_thread);
            Ok(entry.0)
        }
    }
}

fn _keymgr_get_and_lock_processwide_ptr_2(
    env: &mut Environment,
    key: i32,
    result: MutPtr<MutVoidPtr>,
) -> i32 {
    match get_and_lock_processwide_ptr_inner(env, key) {
        Ok(ptr) => {
            env.mem.write(result, ptr);
            0 // success
        }
        Err(err) => err,
    }
}

fn _keymgr_get_and_lock_processwide_ptr(env: &mut Environment, key: i32) -> MutVoidPtr {
    match get_and_lock_processwide_ptr_inner(env, key) {
        Ok(ptr) => ptr,
        Err(_) => Ptr::null(),
    }
}

fn _keymgr_set_and_unlock_processwide_ptr(env: &mut Environment, key: i32, ptr: MutVoidPtr) -> i32 {
    let entry = env
        .libc_state
        .keymgr
        .processwide_ptrs
        .get_mut(&key)
        .unwrap();
    // TODO: error handling
    assert_eq!(entry.1, Some(env.current_thread));
    *entry = (ptr, None);
    0 // success
}

fn _keymgr_unlock_processwide_ptr(env: &mut Environment, key: i32) -> i32 {
    let entry = env
        .libc_state
        .keymgr
        .processwide_ptrs
        .get_mut(&key)
        .unwrap();
    // TODO: error handling
    assert_eq!(entry.1, Some(env.current_thread));
    entry.1 = None;
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(_keymgr_get_and_lock_processwide_ptr_2(_, _)),
    export_c_func!(_keymgr_get_and_lock_processwide_ptr(_)),
    export_c_func!(_keymgr_set_and_unlock_processwide_ptr(_, _)),
    export_c_func!(_keymgr_unlock_processwide_ptr(_)),
];
