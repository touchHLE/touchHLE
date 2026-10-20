/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `sys/utsname.h`

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::libc::errno::set_errno;
use crate::mem::{MutPtr, SafeRead};

const SYS_NAMELEN: usize = 256;

#[allow(non_camel_case_types)]
#[derive(Debug)]
#[repr(C, packed)]
struct utsname {
    sysname: [u8; SYS_NAMELEN],
    nodename: [u8; SYS_NAMELEN],
    release: [u8; SYS_NAMELEN],
    version: [u8; SYS_NAMELEN],
    machine: [u8; SYS_NAMELEN],
}
unsafe impl SafeRead for utsname {}

fn uname(env: &mut Environment, name: MutPtr<utsname>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let mut uts_name = env.mem.read(name);
    // TODO: use shared constants with sysctlbyname
    // Same as sysctlbyname 'kern.ostype'
    let sysname = b"Darwin\0";
    uts_name.sysname[..sysname.len()].copy_from_slice(sysname);
    // Same as sysctlbyname 'kern.hostname'
    let nodename = b"touchHLE\0";
    uts_name.nodename[..nodename.len()].copy_from_slice(nodename);
    // Same as sysctlbyname 'kern.osrelease'
    let release = b"10.0.0d3\0";
    uts_name.release[..release.len()].copy_from_slice(release);
    // Same as sysctlbyname 'kern.osversion'
    let version = b"7A341\0";
    uts_name.version[..version.len()].copy_from_slice(version);
    // Same as sysctlbyname 'hw.machine'
    let machine = b"iPhone1,1\0";
    uts_name.machine[..machine.len()].copy_from_slice(machine);

    0 // Success
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(uname(_))];
