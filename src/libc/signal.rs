/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::set_errno;
use crate::mem::{ConstVoidPtr, MutVoidPtr, Ptr};
use crate::Environment;

fn sigaction(env: &mut Environment, signum: i32, act: ConstVoidPtr, old_act: MutVoidPtr) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    log!("TODO: sigaction({:?}, {:?}, {:?})", signum, act, old_act);
    0
}

fn signal(env: &mut Environment, signum: i32, handler: MutVoidPtr) -> MutVoidPtr {
    // TODO: handle errno properly
    set_errno(env, 0);

    log!("TODO: signal({:?}, {:?})", signum, handler);
    Ptr::null()
}

fn sigprocmask(env: &mut Environment, how: i32, set: ConstVoidPtr, old_set: MutVoidPtr) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    log!("TODO: sigprocmask({}, {:?}, {:?})", how, set, old_set);
    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sigaction(_, _, _)),
    export_c_func!(signal(_, _)),
    export_c_func!(sigprocmask(_, _, _)),
];
