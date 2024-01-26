/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `sys/sysctl.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, GuestUSize, MutPtr, MutVoidPtr};
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
    log!(
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

fn sysctlbyname(
    env: &mut Environment,
    name: ConstPtr<u8>,
    oldp: MutVoidPtr,
    oldlenp: MutPtr<GuestUSize>,
    newp: MutVoidPtr,
    newlen: GuestUSize,
) -> i32 {
    let name_str = env.mem.cstr_at_utf8(name).unwrap();
    log_dbg!(
        "TODO: sysctlbyname({:?}, {:?}, {:?}, {:?}, {:x})",
        name_str,
        oldp,
        oldlenp,
        newp,
        newlen
    );
    assert_eq!(name_str, "hw.machine");
    if oldp.is_null() && newp.is_null() {
        // "iPhone1,1"
        env.mem.write(oldlenp, 10);
        return 0;
    }
    assert!(!oldp.is_null() && !oldlenp.is_null());
    assert!(newp.is_null());
    let hw_machine_str = env.mem.alloc_and_write_cstr(b"iPhone1,1");
    assert_eq!(env.mem.read(oldlenp), 10);
    env.mem
        .memmove(oldp, hw_machine_str.cast().cast_const(), 10);
    env.mem.free(hw_machine_str.cast());
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sysctl(_, _, _, _, _, _)),
    export_c_func!(sysctlbyname(_, _, _, _, _)),
];
