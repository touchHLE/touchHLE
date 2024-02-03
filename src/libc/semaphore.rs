/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `semaphore.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::set_errno;
use crate::libc::posix_io::stat::mode_t;
use crate::libc::posix_io::{O_CREAT, O_EXCL};
use crate::mem::{ConstPtr, MutPtr};
use crate::{Environment, ThreadId};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

// SEM_FAILED is defined as -1 while having a type of sem_t *
pub const SEM_FAILED: MutPtr<sem_t> = MutPtr::from_bits(u32::MAX);

#[derive(Default)]
pub struct State {
    named_semaphores: HashMap<String, Rc<RefCell<SemaphoreHostObject>>>,
    pub open_semaphores: HashMap<MutPtr<sem_t>, Rc<RefCell<SemaphoreHostObject>>>,
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

pub fn sem_open(
    env: &mut Environment,
    name: ConstPtr<u8>,
    oflag: i32,
    _mode: mode_t,
    value: u32,
) -> MutPtr<sem_t> {
    // TODO: handle errno properly
    set_errno(env, 0);

    let sem_name = env.mem.cstr_at_utf8(name).unwrap();
    let sem_name_str = sem_name.to_string();
    let host_sem_rc =
        if let Some(existing_host_sem_rc) = State::get(env).named_semaphores.get(sem_name) {
            if (oflag & O_EXCL) == 0 {
                // TODO: set errno
                return SEM_FAILED;
            }
            let existing_host_sem = (*existing_host_sem_rc).borrow();
            if let Some(existing_sem) = existing_host_sem.guest_sem {
                return existing_sem;
            }
            existing_host_sem_rc.clone()
        } else {
            if (oflag & O_CREAT) == 0 {
                // TODO: set errno
                return SEM_FAILED;
            }
            let host_sem_rc = Rc::new(RefCell::new(SemaphoreHostObject {
                value: value as i32,
                waiting: HashSet::new(),
                guest_sem: None,
            }));
            State::get_mut(env)
                .named_semaphores
                .insert(sem_name_str, Rc::clone(&host_sem_rc));
            host_sem_rc
        };

    let sem = env.mem.alloc_and_write(0);
    (*host_sem_rc).borrow_mut().guest_sem = Some(sem);
    State::get_mut(env).open_semaphores.insert(sem, host_sem_rc);

    sem
}

pub fn sem_post(env: &mut Environment, sem: MutPtr<sem_t>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    env.sem_increment(sem);
    0 // success
}

pub fn sem_wait(env: &mut Environment, sem: MutPtr<sem_t>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    env.sem_decrement(sem, true);
    0 // success
}

fn sem_trywait(env: &mut Environment, sem: MutPtr<sem_t>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    if env.sem_decrement(sem, false) {
        0 // success
    } else {
        -1
    }
}

pub fn sem_close(env: &mut Environment, sem: MutPtr<sem_t>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let host_sem_rc = env
        .libc_state
        .semaphore
        .open_semaphores
        .remove(&sem)
        .unwrap();
    let mut host_sem = (*host_sem_rc).borrow_mut();
    env.mem.free(host_sem.guest_sem.unwrap().cast());
    host_sem.guest_sem = None;
    0 // success
}

pub fn sem_unlink(env: &mut Environment, name: ConstPtr<u8>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let sem_name = env.mem.cstr_at_utf8(name).unwrap();
    env.libc_state.semaphore.named_semaphores.remove(sem_name);
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
