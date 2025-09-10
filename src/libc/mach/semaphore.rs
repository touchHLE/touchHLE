/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `mach/semaphore.h`
//!
//! Implemented as a wrapper around libc semaphore

#![allow(non_camel_case_types)]

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::libc::mach::init::MACH_TASK_SELF;
use crate::libc::mach::thread_info::{kern_return_t, KERN_SUCCESS};
use crate::libc::semaphore::{sem_destroy, sem_init, sem_post, sem_t, sem_wait};
use crate::mem::MutPtr;

type task = std::ffi::c_void;
type task_t = MutPtr<task>;

// Opaque type, can be anything we want. Reusing sem_t for convenience
type semaphore = sem_t;
type semaphore_t = MutPtr<semaphore>;

fn semaphore_create(
    env: &mut Environment,
    task: task_t,
    semaphore: MutPtr<semaphore_t>,
    policy: i32,
    value: i32,
) -> kern_return_t {
    assert_eq!(task.to_bits(), MACH_TASK_SELF);
    assert_eq!(policy, 0);

    let open_semaphore: semaphore_t = env.mem.alloc_and_write(0);
    let res = sem_init(env, open_semaphore, 0, value.try_into().unwrap());
    assert_eq!(res, 0);

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
    sem_destroy(env, semaphore);
    env.mem.free(semaphore.cast());
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
