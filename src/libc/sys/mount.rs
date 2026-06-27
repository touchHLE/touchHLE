/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `sys/mount.h`, file system statistics

use crate::dyld::{export_c_func, FunctionExports};
use crate::fs::GuestPath;
use crate::libc::dirent::MAXPATHLEN;
use crate::libc::errno::{set_errno, EBADF, ENOENT};
use crate::libc::posix_io::stat::uid_t;
use crate::libc::posix_io::{FileDescriptor, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO};
use crate::mem::{ConstPtr, MutPtr, SafeRead};
use crate::Environment;

const MFSTYPENAMELEN: usize = 16;

#[allow(non_camel_case_types)]
#[derive(Default, Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct fsid_t {
    pub val: [i32; 2],
}

#[allow(non_camel_case_types)]
#[derive(Debug)]
#[repr(C, packed)]
pub struct statfs {
    pub f_bsize: u32,
    pub f_iosize: i32,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_fsid: fsid_t,
    pub f_owner: uid_t,
    pub f_type: u32,
    pub f_flags: u32,
    pub f_fssubtype: u32,
    pub f_fstypename: [u8; MFSTYPENAMELEN],
    pub f_mntonname: [u8; MAXPATHLEN],
    pub f_mntfromname: [u8; MAXPATHLEN],
    pub f_reserved: [u32; 8],
}
unsafe impl SafeRead for statfs {}

fn fake_statfs() -> statfs {
    // Values are taken from a test run of iOS 4.3 Simulator
    let mut statfs = statfs {
        f_bsize: 4096,
        f_iosize: 1048576,
        f_blocks: 16567314,
        f_bfree: 12461147,
        f_bavail: 12397147,
        f_files: 16567312,
        f_ffree: 12397147,
        f_fsid: fsid_t {
            val: [234881026, 17],
        },
        f_owner: 0,
        f_type: 17,
        f_flags: 75550720,
        f_fssubtype: 1,
        f_fstypename: [b'\0'; MFSTYPENAMELEN],
        f_mntonname: [b'\0'; MAXPATHLEN],
        f_mntfromname: [b'\0'; MAXPATHLEN],
        f_reserved: [0u32; 8],
    };
    statfs.f_fstypename[..3].copy_from_slice(b"hfs");
    statfs.f_mntonname[..1].copy_from_slice(b"/");
    statfs.f_mntfromname[..12].copy_from_slice(b"/dev/disk0s2");
    statfs
}

/// Internal helper for `statfs`, not a part of the API.
pub fn statfs_inner(env: &mut Environment, path: ConstPtr<u8>) -> Result<statfs, i32> {
    // FIXME does directory matter?
    assert!(env
        .mem
        .cstr_at_utf8(path)
        .is_ok_and(|path| path.starts_with(env.fs.home_directory().join("Documents").as_str())));

    // TODO: Handle additional errors
    let path = env.mem.cstr_at_utf8(path).unwrap();
    if !env.fs.exists(GuestPath::new(path)) {
        return Err(ENOENT);
    }

    Ok(fake_statfs())
}

fn statfs(env: &mut Environment, path: ConstPtr<u8>, buf: MutPtr<statfs>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let result = match statfs_inner(env, path) {
        Ok(statfs) => {
            env.mem.write(buf, statfs);
            0
        }
        Err(error) => {
            set_errno(env, error);
            -1
        }
    };

    log!(
        "TODO: statfs({:?}, {buf:?}) -> {result}",
        env.mem.cstr_at_utf8(path)
    );
    result
}

fn fstatfs(env: &mut Environment, fd: FileDescriptor, buf: MutPtr<statfs>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let result = if !matches!(fd, STDIN_FILENO | STDOUT_FILENO | STDERR_FILENO)
        && !env.libc_state.posix_io.is_fd_open(fd)
    {
        set_errno(env, EBADF);
        -1
    } else {
        env.mem.write(buf, fake_statfs());
        0
    };

    log!("TODO: fstatfs({fd}, {buf:?}) -> {result}");
    result
}

pub const FUNCTIONS: FunctionExports =
    &[export_c_func!(statfs(_, _)), export_c_func!(fstatfs(_, _))];
