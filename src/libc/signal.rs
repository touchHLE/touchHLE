/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::mem::{ConstVoidPtr, MutVoidPtr, Ptr};

fn sigaction(_env: &mut Environment, signum: i32, act: ConstVoidPtr, oldact: MutVoidPtr) -> i32 {
    log!("TODO: sigaction({:?}, {:?}, {:?})", signum, act, oldact);
    0
}

fn signal(_env: &mut Environment, signum: i32, handler: MutVoidPtr) -> MutVoidPtr {
    log!("TODO: signal({:?}, {:?})", signum, handler);
    Ptr::null()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sigaction(_, _, _)),
    export_c_func!(signal(_, _)),
];
