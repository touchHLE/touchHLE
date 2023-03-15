/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `sys/sysctl.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{GuestUSize, MutPtr, MutVoidPtr};
use crate::Environment;

fn sysctl(
    env: &mut Environment,
    name: MutPtr<i32>,
    name_len: u32,
    oldp: MutVoidPtr,
    oldlenp: MutPtr<GuestUSize>,
    newp: MutVoidPtr,
    newlen: GuestUSize,
) -> i32 {
    logg!(
        "TODO: sysctl({:?}, {:#x}, {:?}, {:?}, {:?}, {:x})",
        name,
        name_len,
        oldp,
        oldlenp,
        newp,
        newlen
    );
    assert!(!oldp.is_null() && !oldlenp.is_null()); // TODO
    assert!(newp.is_null()); // TODO
    env.mem.write(oldlenp, 0);
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(sysctl(_, _, _, _, _, _))];
