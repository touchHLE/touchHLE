/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `sys/sysctl.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{guest_size_of, GuestUSize, MutPtr, MutVoidPtr};
use crate::Environment;
use std::ops::Add;

const CTL_HW: i32 = 6;
const HW_PAGESIZE: i32 = 7;

fn sysctl(
    env: &mut Environment,
    name: MutPtr<i32>,
    name_len: u32,
    oldp: MutVoidPtr,
    oldlenp: MutPtr<GuestUSize>,
    newp: MutVoidPtr,
    newlen: GuestUSize,
) -> i32 {
    if name_len < 2 {
        return -1; // TODO: set errno to EINVAL;
    }
    let mut name_vals = vec![0; name_len as usize];
    for (i, v) in name_vals.iter_mut().enumerate() {
        *v = env.mem.read(name.add(i as GuestUSize));
    }
    match name_vals[0] {
        CTL_HW => match name_vals[1] {
            HW_PAGESIZE => sysctl_pagesize(env, oldp, oldlenp, newp, newlen),
            _ => sysctl_todo(env, &name_vals, name_len, oldp, oldlenp, newp, newlen),
        },
        _ => sysctl_todo(env, &name_vals, name_len, oldp, oldlenp, newp, newlen),
    }
}

fn sysctl_pagesize(
    env: &mut Environment,
    oldp: MutVoidPtr,
    oldlenp: MutPtr<GuestUSize>,
    newp: MutVoidPtr,
    newlen: GuestUSize,
) -> i32 {
    if !newp.is_null() || newlen != 0 {
        return -1; // TODO: set errno to EPERM
    }
    let oldlen = env.mem.read(oldlenp);
    if oldlen < guest_size_of::<GuestUSize>() {
        return -1; // TODO: set errno to ENOMEM
    }
    env.mem.write(oldp.cast(), 4096 as GuestUSize);
    0
}

fn sysctl_todo(
    env: &mut Environment,
    name_vals: &[i32],
    name_len: u32,
    oldp: MutVoidPtr,
    oldlenp: MutPtr<GuestUSize>,
    newp: MutVoidPtr,
    newlen: GuestUSize,
) -> i32 {
    log!(
        "TODO: sysctl({:?}, {:#x}, {:?}, {:?}, {:?}, {:x})",
        name_vals,
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
