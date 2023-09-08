/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `semaphore.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::posix_io::stat::mode_t;
use crate::libc::posix_io::O_CREAT;
use crate::mem::{ConstPtr, MutPtr};
use crate::{Environment, ThreadId};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct State {
    pub semaphores: HashMap<MutPtr<sem_t>, SemaphoreHostObject>,
    named_semaphores: HashMap<String, MutPtr<sem_t>>,
}
impl State {
    fn get(env: &Environment) -> &Self {
        &env.libc_state.pthread.semaphore
    }
    fn get_mut(env: &mut Environment) -> &mut Self {
        &mut env.libc_state.pthread.semaphore
    }
}

#[allow(non_camel_case_types)]
pub type sem_t = i32;

pub struct SemaphoreHostObject {
    pub value: i32,
    pub waiting: HashSet<ThreadId>,
}

fn sem_open(
    env: &mut Environment,
    name: ConstPtr<u8>,
    oflag: i32,
    _mode: mode_t,
    value: u32,
) -> MutPtr<sem_t> {
    assert_eq!(O_CREAT, oflag);

    let sem_name = env.mem.cstr_at_utf8(name).unwrap();
    let sem_name_str = sem_name.to_string();
    if State::get(env).named_semaphores.contains_key(sem_name) {
        return *State::get(env).named_semaphores.get(sem_name).unwrap();
    }

    let sem = env.mem.alloc_and_write(0);

    assert!(!State::get(env).semaphores.contains_key(&sem));
    State::get_mut(env).semaphores.insert(
        sem,
        SemaphoreHostObject {
            value: value as i32,
            waiting: HashSet::new(),
        },
    );
    State::get_mut(env)
        .named_semaphores
        .insert(sem_name_str, sem);

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

fn sem_unlink(env: &mut Environment, name: ConstPtr<u8>) -> i32 {
    let sem_name = env.mem.cstr_at_utf8(name).unwrap();
    let sem_name_str = sem_name.to_string();
    State::get_mut(env).named_semaphores.remove(&sem_name_str);
    0 // success
}

fn sem_close(env: &mut Environment, sem: MutPtr<sem_t>) -> i32 {
    State::get_mut(env).semaphores.remove(&sem);
    env.mem.free(sem.cast());
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sem_open(_, _, _, _)),
    export_c_func!(sem_post(_)),
    export_c_func!(sem_wait(_)),
    export_c_func!(sem_trywait(_)),
    export_c_func!(sem_unlink(_)),
    export_c_func!(sem_close(_)),
];
