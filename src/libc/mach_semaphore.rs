/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `mach/semaphore.h`
//!
//! Implemented as a wrapper around libc semaphore

#![allow(non_camel_case_types)]

use std::collections::HashMap;

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::libc::mach_init::MACH_TASK_SELF;
use crate::libc::mach_thread_info::{kern_return_t, KERN_SUCCESS};
use crate::libc::posix_io::{O_CREAT, O_EXCL};
use crate::libc::semaphore::{
    sem_close, sem_open, sem_post, sem_t, sem_unlink, sem_wait, SEM_FAILED,
};
use crate::mem::{ConstPtr, MutPtr};

type task = std::ffi::c_void;
type task_t = MutPtr<task>;

// Opaque type, can be anything we want. Reusing sem_t for convenience
type semaphore = sem_t;
type semaphore_t = MutPtr<semaphore>;

#[derive(Default)]
pub struct State {
    next_semaphore_id: u64,
    semaphores: HashMap<semaphore_t, MachSemaphoreHostObject>,
}
impl State {
    fn get(env: &Environment) -> &Self {
        &env.libc_state.mach_semaphore
    }
    fn get_mut(env: &mut Environment) -> &mut Self {
        &mut env.libc_state.mach_semaphore
    }
}

struct MachSemaphoreHostObject {
    libc_sem_name: ConstPtr<u8>,
}

fn semaphore_create(
    env: &mut Environment,
    task: task_t,
    semaphore: MutPtr<semaphore_t>,
    policy: i32,
    value: i32,
) -> kern_return_t {
    assert_eq!(task.to_bits(), MACH_TASK_SELF);
    assert_eq!(policy, 0);

    let next_semaphore_id = State::get(env).next_semaphore_id;

    let name = format!("mach_semaphore_{}", next_semaphore_id);
    let libc_sem_name = env.mem.alloc_and_write_cstr(name.as_ref()).cast_const();

    let open_semaphore: semaphore_t =
        sem_open(env, libc_sem_name, O_CREAT | O_EXCL, 0, value as u32);
    assert_ne!(open_semaphore, SEM_FAILED);

    State::get_mut(env)
        .semaphores
        .insert(open_semaphore, MachSemaphoreHostObject { libc_sem_name });
    State::get_mut(env).next_semaphore_id = next_semaphore_id + 1;

    env.mem.write(semaphore, open_semaphore);
    let result = KERN_SUCCESS;
    log_dbg!(
        "semaphore_create({:?}, {:?}, {:?}, {:?}) -> {:?}",
        task,
        semaphore,
        policy,
        value,
        result
    );
    result
}

fn semaphore_signal(env: &mut Environment, semaphore: semaphore_t) -> kern_return_t {
    assert_eq!(sem_post(env, semaphore), 0);
    let result = KERN_SUCCESS;
    log_dbg!("semaphore_signal({:?}) -> {:?}", semaphore, result);
    result
}

fn semaphore_wait(env: &mut Environment, semaphore: semaphore_t) -> kern_return_t {
    assert_eq!(sem_wait(env, semaphore), 0);
    let result = KERN_SUCCESS;
    log_dbg!("semaphore_wait({:?}) -> {:?}", semaphore, result);
    result
}

fn semaphore_destroy(env: &mut Environment, semaphore: semaphore_t) -> kern_return_t {
    let host_object = State::get_mut(env).semaphores.remove(&semaphore).unwrap();
    sem_unlink(env, host_object.libc_sem_name);
    sem_close(env, semaphore);
    env.mem.free(host_object.libc_sem_name.cast_mut().cast());
    let result = KERN_SUCCESS;
    log_dbg!("semaphore_destroy({:?}) -> {:?}", semaphore, result);
    result
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(semaphore_create(_, _, _, _)),
    export_c_func!(semaphore_signal(_)),
    export_c_func!(semaphore_wait(_)),
    export_c_func!(semaphore_destroy(_)),
];
