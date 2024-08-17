/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `sys/sysctl.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::set_errno;
use crate::libc::sysctl::SysInfoType::String;
use crate::mem::{guest_size_of, ConstPtr, GuestUSize, MutPtr, MutVoidPtr};
use crate::Environment;

enum SysInfoType {
    String(&'static [u8]),
    Int32(i32),
    Int64(i64),
}

fn sysctl(
    env: &mut Environment,
    name: MutPtr<i32>,
    name_len: u32,
    oldp: MutVoidPtr,
    oldlenp: MutPtr<GuestUSize>,
    newp: MutVoidPtr,
    newlen: GuestUSize,
) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

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
    // TODO: handle errno properly
    set_errno(env, 0);

    let name_str = env.mem.cstr_at_utf8(name).unwrap();
    log_dbg!(
        "sysctlbyname({:?}, {:?}, {:?}, {:?}, {:x})",
        name_str,
        oldp,
        oldlenp,
        newp,
        newlen
    );

    assert!(newp.is_null());
    assert_eq!(newlen, 0);

    // Below values corresponds to the original iPhone.
    // Reference https://www.mail-archive.com/misc@openbsd.org/msg80988.html
    let val: SysInfoType = match name_str {
        // Generic CPU, I/O
        "hw.machine" => String(b"iPhone1,1"),
        "hw.model" => String(b"M68AP"),
        "hw.ncpu" => SysInfoType::Int32(1),
        "hw.cpufrequency" => SysInfoType::Int64(412000000),
        "hw.busfrequency" => SysInfoType::Int64(103000000),
        "hw.physmem" => SysInfoType::Int32(121634816), // not sure about this type
        "hw.usermem" => SysInfoType::Int32(93564928), // not sure about this type
        "hw.memsize" => SysInfoType::Int64(121634816),
        "hw.pagesize" => SysInfoType::Int64(4096),
        // High kernel limits
        "kern.ostype" => String(b"Darwin"),
        "kern.osrelease" => String(b"10.0.0d3"),
        "kern.osversion" => String(b"7A341"),
        "kern.hostname" => String(b"touchHLE"), // this is arbitrary
        "kern.version" => String(b"Darwin Kernel Version 10.0.0d3: Wed May 13 22:11:58 PDT 2009; root:xnu-1357.2.89~4/RELEASE_ARM_S5L8900X"),
        _str => unimplemented!("{}", _str)
    };
    let len: GuestUSize = match val {
        String(str) => str.len() as GuestUSize + 1,
        SysInfoType::Int32(_) => guest_size_of::<i32>(),
        SysInfoType::Int64(_) => guest_size_of::<i64>(),
    };
    if oldp.is_null() {
        env.mem.write(oldlenp, len);
        return 0;
    }
    assert!(!oldp.is_null() && !oldlenp.is_null());
    let oldlen = env.mem.read(oldlenp);
    if oldlen < len {
        // TODO: set errno
        // TODO: write partial data
        log!("sysctlbyname for '{}': the buffer of size {} is too low to fit the value of size {}, returning -1", name_str, oldlen, len);
        return -1;
    }
    match val {
        String(str) => {
            let sysctl_str = env.mem.alloc_and_write_cstr(str);
            env.mem.memmove(oldp, sysctl_str.cast().cast_const(), len);
            env.mem.free(sysctl_str.cast());
        }
        SysInfoType::Int32(num) => {
            env.mem.write(oldp.cast(), num);
        }
        SysInfoType::Int64(num) => {
            env.mem.write(oldp.cast(), num);
        }
    }
    env.mem.write(oldlenp, len);
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sysctl(_, _, _, _, _, _)),
    export_c_func!(sysctlbyname(_, _, _, _, _)),
];
