/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! POSIX `sys/stat.h`

use super::{close, off_t, open_direct, FileDescriptor};
use crate::dyld::{export_c_func, FunctionExports};
use crate::fs::{FsError, GuestFile, GuestPath};
use crate::libc::errno::{set_errno, EEXIST};
use crate::libc::time::timespec;
use crate::mem::{ConstPtr, MutPtr, SafeRead};
use crate::Environment;
use std::io::{Seek, SeekFrom};

#[allow(non_camel_case_types)]
pub type dev_t = u32;
#[allow(non_camel_case_types)]
pub type mode_t = u16;
#[allow(non_camel_case_types)]
pub type nlink_t = u16;
#[allow(non_camel_case_types)]
pub type ino_t = u64;
#[allow(non_camel_case_types)]
pub type uid_t = u32;
#[allow(non_camel_case_types)]
pub type gid_t = u32;
#[allow(non_camel_case_types)]
pub type blkcnt_t = u64;
#[allow(non_camel_case_types)]
pub type blksize_t = u32;

// enum values sourced from ```man 2 stat```
pub const S_IFDIR: mode_t = 0o0040000;
pub const S_IFREG: mode_t = 0o0100000;

#[allow(non_camel_case_types)]
#[derive(Default)]
#[repr(C, packed)]
pub struct stat {
    st_dev: dev_t,
    st_mode: mode_t,
    st_nlink: nlink_t,
    st_ino: ino_t,
    st_uid: uid_t,
    st_gid: gid_t,
    st_rdev: dev_t,
    st_atimespec: timespec,
    st_mtimespec: timespec,
    st_ctimespec: timespec,
    st_birthtimespec: timespec,
    st_size: off_t,
    st_blocks: blkcnt_t,
    st_blksize: blksize_t,
    st_flags: u32,
    st_gen: u32,
    st_lspare: i32,
    st_qspare: [i64; 2],
}
unsafe impl SafeRead for stat {}

fn mkdir(env: &mut Environment, path: ConstPtr<u8>, mode: mode_t) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let path_str = env.mem.cstr_at_utf8(path).unwrap();
    // TODO: respect the mode
    match env.fs.create_dir(GuestPath::new(&path_str)) {
        Ok(()) => {
            log_dbg!("mkdir({:?} {:?}, {:#x}) => 0", path, path_str, mode);
            0
        }
        Err(err) => {
            log!(
                "Warning: mkdir({:?} {:?}, {:#x}) failed with {:?}, returning -1",
                path,
                path_str,
                mode,
                err
            );
            match err {
                FsError::AlreadyExist => {
                    set_errno(env, EEXIST);
                }
                _ => unimplemented!(),
            }
            -1
        }
    }
}

/// Helper for [stat()] and [fstat()] that fills the data in the stat struct
fn fstat_inner(env: &mut Environment, fd: FileDescriptor, buf: MutPtr<stat>) -> i32 {
    // TODO: error handling for unknown fd?
    let file = env.libc_state.posix_io.file_for_fd(fd).unwrap();

    // FIXME: This implementation is highly incomplete. fstat() returns a huge
    // struct with many kinds of data in it. This code is assuming the caller
    // only wants a small part of it.

    let mut stat = stat::default();

    match file.file {
        GuestFile::File(_) | GuestFile::IpaBundleFile(_) | GuestFile::ResourceFile(_) => {
            stat.st_mode |= S_IFREG;

            // Obtain file size
            // TODO: Use the stream_len() method if that ever gets stabilized.
            let old_pos = file.file.stream_position().unwrap();
            stat.st_size = file
                .file
                .seek(SeekFrom::End(0))
                .unwrap()
                .try_into()
                .unwrap();
            file.file.seek(SeekFrom::Start(old_pos)).unwrap();
        }
        GuestFile::Directory => {
            stat.st_mode |= S_IFDIR;

            // TODO: st_size
        }
    }

    env.mem.write(buf, stat);

    0 // success
}

fn fstat(env: &mut Environment, fd: FileDescriptor, buf: MutPtr<stat>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    log!("Warning: fstat() call, this function is mostly unimplemented");
    let result = fstat_inner(env, fd, buf);
    log_dbg!("fstat({:?}, {:?}) -> {}", fd, buf, result);
    result
}

fn stat(env: &mut Environment, path: ConstPtr<u8>, buf: MutPtr<stat>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    log!("Warning: stat() call, this function is mostly unimplemented");

    fn do_stat(env: &mut Environment, path: ConstPtr<u8>, buf: MutPtr<stat>) -> i32 {
        if path.is_null() {
            return -1; // TODO: Set errno
        }

        // Open and reuse fstat implementation
        let fd = open_direct(env, path, 0);
        if fd == -1 {
            return -1; // TODO: Set errno
        }

        let result = fstat_inner(env, fd, buf);
        assert!(close(env, fd) == 0);
        result
    }
    let result = do_stat(env, path, buf);

    log_dbg!(
        "stat({:?} {:?}, {:?}) -> {}",
        path,
        env.mem.cstr_at_utf8(path),
        buf,
        result
    );
    result
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(mkdir(_, _)),
    export_c_func!(fstat(_, _)),
    export_c_func!(stat(_, _)),
];
