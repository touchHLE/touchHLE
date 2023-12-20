/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Conditional variables.

use super::mutex::pthread_mutex_t;
use crate::dyld::FunctionExports;
use crate::libc::pthread::mutex::pthread_mutex_unlock;
use crate::mem::{ConstPtr, MutPtr, SafeRead};
use crate::{export_c_func, Environment};
use std::collections::HashMap;

use crate::environment::ThreadBlock;

#[repr(C, packed)]
struct pthread_condattr_t {}
unsafe impl SafeRead for pthread_condattr_t {}

#[repr(C, packed)]
pub struct OpaqueCond {
    _unused: i32,
}
unsafe impl SafeRead for OpaqueCond {}

pub type pthread_cond_t = MutPtr<OpaqueCond>;

#[derive(Default)]
pub struct State {
    pub condition_variables: HashMap<pthread_cond_t, CondHostObject>,
    pub mutexes: HashMap<pthread_cond_t, pthread_mutex_t>,
}
impl State {
    fn get(env: &Environment) -> &Self {
        &env.libc_state.pthread.cond
    }
    fn get_mut(env: &mut Environment) -> &mut Self {
        &mut env.libc_state.pthread.cond
    }
}

pub struct CondHostObject {
    pub done: bool,
}

fn pthread_cond_init(
    env: &mut Environment,
    cond: MutPtr<pthread_cond_t>,
    attr: ConstPtr<pthread_condattr_t>,
) -> i32 {
    assert!(attr.is_null());
    let opaque = env.mem.alloc_and_write(OpaqueCond { _unused: 0 });
    env.mem.write(cond, opaque);

    assert!(!State::get(env).condition_variables.contains_key(&opaque));
    State::get_mut(env)
        .condition_variables
        .insert(opaque, CondHostObject { done: false });
    0 // success
}

fn pthread_cond_wait(
    env: &mut Environment,
    cond: MutPtr<pthread_cond_t>,
    mutex: MutPtr<pthread_mutex_t>,
) -> i32 {
    let res = pthread_mutex_unlock(env, mutex);
    assert_eq!(res, 0);
    assert!(matches!(
        env.threads[env.current_thread].blocked_by,
        ThreadBlock::NotBlocked
    ));
    log_dbg!(
        "Thread {} is blocking on condition variable {:?}",
        env.current_thread,
        cond
    );
    let cond_var = env.mem.read(cond);
    env.threads[env.current_thread].blocked_by = ThreadBlock::Condition(cond_var);
    assert!(!State::get(env).mutexes.contains_key(&cond_var));
    let mutex_val = env.mem.read(mutex);
    State::get_mut(env).mutexes.insert(cond_var, mutex_val);
    0 // success
}

fn pthread_cond_signal(env: &mut Environment, cond: MutPtr<pthread_cond_t>) -> i32 {
    let cond_var = env.mem.read(cond);
    log_dbg!(
        "Thread {} unblocks one thread waiting on condition variable {:?}",
        env.current_thread,
        cond
    );
    State::get_mut(env)
        .condition_variables
        .get_mut(&cond_var)
        .unwrap()
        .done = true;
    0 // success
}

fn pthread_cond_destroy(env: &mut Environment, cond: MutPtr<pthread_cond_t>) -> i32 {
    let cond_var = env.mem.read(cond);
    State::get_mut(env).condition_variables.remove(&cond_var);
    State::get_mut(env).mutexes.remove(&cond_var);
    env.mem.free(cond_var.cast());
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(pthread_cond_init(_, _)),
    export_c_func!(pthread_cond_wait(_, _)),
    export_c_func!(pthread_cond_signal(_)),
    export_c_func!(pthread_cond_destroy(_)),
];
