/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `ptrace.h`

use crate::dyld::FunctionExports;
use crate::libc::unistd::pid_t;
use crate::mem::MutPtr;
use crate::{export_c_func, Environment};

fn ptrace(_env: &mut Environment, request: i32, pid: pid_t, addr: MutPtr<u8>, data: i32) -> i32 {
    log!(
        "Warning: ptrace({}, {}, {:?}, {}) called, returning -1",
        request,
        pid,
        addr,
        data
    );
    -1
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(ptrace(_, _, _, _))];
