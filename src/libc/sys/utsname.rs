/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `sys/utsname.h`

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::mem::MutPtr;

// TODO: struct definition
#[allow(non_camel_case_types)]
struct utsname {}

fn uname(_env: &mut Environment, name: MutPtr<utsname>) -> i32 {
    log!("TODO: uname({:?}), returning -1", name);
    // TODO: set errno
    -1
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(uname(_))];
