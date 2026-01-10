/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! POSIX `sys/statvfs.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::set_errno;
use crate::mem::{ConstPtr, MutPtr, SafeRead};
use crate::Environment;

#[allow(non_camel_case_types)]
pub type fsblkcnt_t = u32;
#[allow(non_camel_case_types)]
pub type fsfilcnt_t = u32;

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
    let result = 0;
    let mut statvfs = statvfs::default();
    statvfs.f_frsize = 4096;
    statvfs.f_blocks = 1024 * 1024;
    statvfs.f_bavail = 512 * 1024;
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
