/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! POSIX `sys/statvfs.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::set_errno;
use crate::libc::sys::mount::statfs_inner;
use crate::mem::{ConstPtr, MutPtr, SafeRead};
use crate::Environment;

#[allow(non_camel_case_types)]
pub type fsblkcnt_t = u32;
#[allow(non_camel_case_types)]
pub type fsfilcnt_t = u32;

pub const ST_RDONLY: u32 = 1;
pub const ST_NOSUID: u32 = 2;

#[allow(non_camel_case_types)]
#[derive(Default)]
#[repr(C, packed)]
pub struct statvfs {
    f_bsize: u32,
    f_frsize: u32,
    f_blocks: fsblkcnt_t,
    f_bfree: fsblkcnt_t,
    f_bavail: fsblkcnt_t,
    f_files: fsfilcnt_t,
    f_ffree: fsfilcnt_t,
    f_favail: fsfilcnt_t,
    f_fsid: u32,
    f_flag: u32,
    f_namemax: u32,
}
unsafe impl SafeRead for statvfs {}

fn statvfs(env: &mut Environment, path: ConstPtr<u8>, buf: MutPtr<statvfs>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);
    let (result, statfs) = statfs_inner(env, path);
    let statvfs = statvfs {
        // From the manpage:
        // "Corresponds to the f_iosize member of struct statfs."
        f_bsize: statfs.f_iosize.try_into().unwrap(),
        // From the manpage:
        // "This corresponds to the f_bsize member of struct statfs."
        f_frsize: statfs.f_bsize,
        f_blocks: statfs.f_blocks.try_into().unwrap(),
        f_bfree: statfs.f_bfree.try_into().unwrap(),
        f_bavail: statfs.f_bavail.try_into().unwrap(),
        f_files: statfs.f_files.try_into().unwrap(),
        f_ffree: statfs.f_ffree.try_into().unwrap(),
        f_favail: statfs.f_ffree.try_into().unwrap(), // TODO: Is this right?
        // From the manpage: "Not meaningful in this implementation"
        f_fsid: 0,
        // In the manpage: "There are two flags defined for the f_flag member"
        // ST_RDONLY and ST_NOSUID
        f_flag: statfs.f_flags & ST_RDONLY & ST_NOSUID,
        f_namemax: 255,
    };
    env.mem.write(buf, statvfs);
    log!(
        "TODO: statvfs({:?} {:?}, {:?}) -> {}",
        path,
        env.mem.cstr_at_utf8(path),
        buf,
        result
    );
    result
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(statvfs(_, _))];
