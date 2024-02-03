#![allow(non_camel_case_types)]

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::libc::semaphore::SemaphoreHostObject;
use crate::mem::MutPtr;

use super::semaphore::sem_t;

type task = std::ffi::c_void;
type task_t = MutPtr<task>;

// Opaque type, can be anything we want. Reusing sem_t for convenience
type semaphore = sem_t;
type semaphore_t = MutPtr<semaphore>;

type kern_return_t = i32;
const KERN_SUCCESS: kern_return_t = 0;

fn semaphore_create(
    env: &mut Environment,
    task: task_t,
    semaphore: MutPtr<semaphore_t>,
    policy: i32,
    value: i32,
) -> kern_return_t {
    assert!(task.is_null());
    assert_eq!(policy, 0);
    assert_eq!(value, 0);
    let sem = env.mem.alloc_and_write(0);
    let host_sem_rc = Rc::new(RefCell::new(SemaphoreHostObject {
        value: 0,
        waiting: HashSet::new(),
        guest_sem: Some(sem),
    }));
    crate::libc::semaphore::State::get_mut(env)
        .open_semaphores
        .insert(sem, host_sem_rc);
    env.mem.write(semaphore, sem);
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
    env.sem_increment(semaphore);
    let result = KERN_SUCCESS;
    log_dbg!("semaphore_signal({:?}) -> {:?}", semaphore, result);
    result
}

fn semaphore_wait(env: &mut Environment, semaphore: semaphore_t) -> kern_return_t {
    assert!(env.sem_decrement(semaphore, true));
    let result = KERN_SUCCESS;
    log_dbg!("semaphore_wait({:?}) -> {:?}", semaphore, result);
    result
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(semaphore_create(_, _, _, _)),
    export_c_func!(semaphore_signal(_)),
    export_c_func!(semaphore_wait(_)),
];
