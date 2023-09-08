/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `semaphore.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::posix_io::stat::mode_t;
use crate::libc::posix_io::O_EXCL;
use crate::mem::{ConstPtr, MutPtr};
use crate::{Environment, ThreadId};
use std::collections::{HashMap, HashSet};

// SEM_FAILED is defined as -1 while having a type of sem_t *
const SEM_FAILED: MutPtr<sem_t> = MutPtr::from_bits(u32::MAX);

#[derive(Default)]
pub struct State {
    pub names_to_host_semaphores: HashMap<String, SemaphoreHostObject>,
    pub semaphores_to_names: HashMap<MutPtr<sem_t>, String>,
}
impl State {
    fn get(env: &Environment) -> &Self {
        &env.libc_state.semaphore
    }
    fn get_mut(env: &mut Environment) -> &mut Self {
        &mut env.libc_state.semaphore
    }
}

#[allow(non_camel_case_types)]
pub type sem_t = i32;

pub struct SemaphoreHostObject {
    pub value: i32,
    pub waiting: HashSet<ThreadId>,
    guest_sem: Option<MutPtr<sem_t>>,
}

fn sem_open(
    env: &mut Environment,
    name: ConstPtr<u8>,
    oflag: i32,
    _mode: mode_t,
    value: u32,
) -> MutPtr<sem_t> {
    assert_ne!(oflag, O_EXCL);

    let sem_name = env.mem.cstr_at_utf8(name).unwrap();
    let sem_name_str = sem_name.to_string();
    if let Some(existing_host_sem) = State::get(env).names_to_host_semaphores.get(sem_name) {
        // TODO: what to do about a closed one?
        let existing_sem = existing_host_sem.guest_sem.unwrap();
        if oflag & O_EXCL == 0 {
            // TODO: set errno
            return SEM_FAILED;
        }
        return existing_sem;
    }

    let sem = env.mem.alloc_and_write(0);

    assert!(!State::get(env)
        .names_to_host_semaphores
        .contains_key(&sem_name_str));
    State::get_mut(env).names_to_host_semaphores.insert(
        sem_name_str.clone(),
        SemaphoreHostObject {
            value: value as i32,
            waiting: HashSet::new(),
            guest_sem: Some(sem),
        },
    );
    State::get_mut(env)
        .semaphores_to_names
        .insert(sem, sem_name_str.clone());

    sem
}

fn sem_post(env: &mut Environment, sem: MutPtr<sem_t>) -> i32 {
    env.sem_increment(sem);
    0 // success
}

fn sem_wait(env: &mut Environment, sem: MutPtr<sem_t>) -> i32 {
    env.sem_decrement(sem, true);
    0 // success
}

fn sem_trywait(env: &mut Environment, sem: MutPtr<sem_t>) -> i32 {
    if env.sem_decrement(sem, false) {
        0 // success
    } else {
        -1
    }
}

fn sem_close(env: &mut Environment, sem: MutPtr<sem_t>) -> i32 {
    let name = env
        .libc_state
        .semaphore
        .semaphores_to_names
        .get(&sem)
        .unwrap();
    let host_sem: &mut _ = env
        .libc_state
        .semaphore
        .names_to_host_semaphores
        .get_mut(name)
        .unwrap();
    env.mem.free(host_sem.guest_sem.unwrap().cast());
    host_sem.guest_sem = None;
    0 // success
}

fn sem_unlink(env: &mut Environment, name: ConstPtr<u8>) -> i32 {
    let sem_name = env.mem.cstr_at_utf8(name).unwrap();
    env.libc_state
        .semaphore
        .names_to_host_semaphores
        .remove(sem_name);
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sem_open(_, _, _, _)),
    export_c_func!(sem_post(_)),
    export_c_func!(sem_wait(_)),
    export_c_func!(sem_trywait(_)),
    export_c_func!(sem_close(_)),
    export_c_func!(sem_unlink(_)),
];
