/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `ifaddrs.h` (interface addresses)

use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::libc::errno::set_errno;
use crate::mem::MutPtr;
use crate::Environment;

// TODO: struct definition
#[allow(non_camel_case_types)]
struct ifaddrs {}

fn getifaddrs(env: &mut Environment, _ifap: MutPtr<MutPtr<ifaddrs>>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    // TODO: implement
    -1
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(getifaddrs(_))];
