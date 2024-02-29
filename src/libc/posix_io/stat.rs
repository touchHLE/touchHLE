/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! POSIX `sys/stat.h`

use super::{close, off_t, open_direct, FileDescriptor};
use crate::dyld::{export_c_func, FunctionExports};
use crate::fs::{GuestFile, GuestPath};
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

// Copied from ```man 2 stat```
#[allow(dead_code)]
pub const S_IFMT: mode_t = 0o0170000; /* type of file */
#[allow(dead_code)]
pub const S_IFIFO: mode_t = 0o0010000; /* named pipe (fifo) */
#[allow(dead_code)]
pub const S_IFCHR: mode_t = 0o0020000; /* character special */
pub const S_IFDIR: mode_t = 0o0040000; /* directory */
#[allow(dead_code)]
pub const S_IFBLK: mode_t = 0o0060000; /* block special */
pub const S_IFREG: mode_t = 0o0100000; /* regular */
#[allow(dead_code)]
pub const S_IFLNK: mode_t = 0o0120000; /* symbolic link */
#[allow(dead_code)]
pub const S_IFSOCK: mode_t = 0o0140000; /* socket */
#[allow(dead_code)]
pub const S_IFWHT: mode_t = 0o0160000; /* whiteout */
#[allow(dead_code)]
pub const S_ISUID: mode_t = 0o0004000; /* set user id on execution */
#[allow(dead_code)]
pub const S_ISGID: mode_t = 0o0002000; /* set group id on execution */
#[allow(dead_code)]
pub const S_ISVTX: mode_t = 0o0001000; /* save swapped text even after use */
#[allow(dead_code)]
pub const S_IRUSR: mode_t = 0o0000400; /* read permission, owner */
#[allow(dead_code)]
pub const S_IWUSR: mode_t = 0o0000200; /* write permission, owner */
#[allow(dead_code)]
pub const S_IXUSR: mode_t = 0o0000100; /* execute/search permission, owner */

#[allow(non_camel_case_types)]
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
    // TODO: respect the mode
    match env
        .fs
        .create_dir(GuestPath::new(&env.mem.cstr_at_utf8(path).unwrap()))
    {
        Ok(()) => {
            log_dbg!("mkdir({:?}, {:#x}) => 0", path, mode);
            0
        }
        Err(()) => {
            // TODO: set errno
            log!(
                "Warning: mkdir({:?}, {:#x}) failed, returning -1",
                path,
                mode,
            );
            -1
        }
    }
}

fn fstat(env: &mut Environment, fd: FileDescriptor, buf: MutPtr<stat>) -> i32 {
    // TODO: error handling for unknown fd?
    let file = env.libc_state.posix_io.file_for_fd(fd).unwrap();

    log!("Warning: fstat() call, this function is mostly unimplemented");
    // FIXME: This implementation is highly incomplete. fstat() returns a huge
    // struct with many kinds of data in it. This code is assuming the caller
    // only wants a small part of it.

    let mut stat = env.mem.read(buf);

    stat.st_mode |= match file.file {
        GuestFile::File(_) | GuestFile::IpaBundleFile(_) | GuestFile::ResourceFile(_) => S_IFREG,
        GuestFile::Directory => S_IFDIR,
    };

    // TODO: Implement stat for directories
    assert!(stat.st_mode & S_IFDIR == 0);

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

    env.mem.write(buf, stat);

    0 // success
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(mkdir(_, _)), export_c_func!(fstat(_, _))];
