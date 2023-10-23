/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! POSIX `sys/stat.h`

use super::{off_t, FileDescriptor};
use crate::dyld::{export_c_func, FunctionExports};
use crate::fs::GuestPath;
use crate::mem::{ConstPtr, MutPtr, MutVoidPtr, SafeRead};
use crate::Environment;
use std::io::{Seek, SeekFrom};

#[allow(non_camel_case_types)]
pub type mode_t = u16;
#[allow(non_camel_case_types)]
pub type dev_t = u32;
#[allow(non_camel_case_types)]
pub type nlink_t = u16;
#[allow(non_camel_case_types)]
pub type uid_t = u32;
#[allow(non_camel_case_types)]
pub type gid_t = u32;

pub const S_IFREG: mode_t = 0o100000;
pub const S_IFDIR: mode_t = 0o040000;

#[repr(packed, C)]
#[derive(Default)]
struct Timespec {
    tv_sec: i32,
    tv_nsec: i32,
}

unsafe impl SafeRead for Timespec {}

#[repr(packed, C)]
#[derive(Default)]
struct StatStruct {
    st_dev: dev_t,
    st_mode: mode_t,
    st_nlink: nlink_t,
    st_ino: u64,
    st_uid: uid_t,
    st_gid: gid_t,
    st_rdev: dev_t,
    st_atimespec: Timespec,
    st_mtimespec: Timespec,
    st_ctimespec: Timespec,
    st_birthtimespec: Timespec,
    st_size: off_t,
    //TODO: More stuff here
}

unsafe impl SafeRead for StatStruct {}

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

fn fstat(env: &mut Environment, fd: FileDescriptor, buf: MutPtr<StatStruct>) -> i32 {
    // TODO: error handling for unknown fd?
    let mut file = env.libc_state.posix_io.file_for_fd(fd).unwrap();

    log!("Warning: fstat() call, this function is mostly unimplemented");

    // TODO: Use the stream_len() method if that ever gets stabilized.
    let old_pos = file.file.stream_position().unwrap();
    let full_size = file.file.seek(SeekFrom::End(0)).unwrap();
    file.file.seek(SeekFrom::Start(old_pos)).unwrap();

    // FIXME: This implementation is highly incomplete. fstat() returns a huge
    // struct with many kinds of data in it. This code is assuming the caller
    // only wants the file size.
    let stat = StatStruct {
        st_size: full_size.try_into().unwrap(),
        ..Default::default()
    };

    env.mem.write(buf, stat);

    0 // success
}

fn statfs(_: &mut Environment, _: MutVoidPtr, _: MutVoidPtr) -> i32 {
    log!(
        "Warning: statfs() call, this is completely unimplemented, but should be enough for sqlite"
    );
    -1
}

fn stat(env: &mut Environment, path: ConstPtr<u8>, buf: MutPtr<StatStruct>) -> i32 {
    let path = GuestPath::new(env.mem.cstr_at_utf8(path).unwrap());
    if !env.fs.exists(path) {
        return -1;
    }
    log!("Warning: stat() call, this function is mostly unimplemented");
    let file = env.fs.is_file(path);
    let dir = env.fs.is_dir(path);
    let mut stats = StatStruct::default();
    if file {
        stats.st_mode = S_IFREG;
    }
    if dir {
        stats.st_mode = S_IFDIR;
    }
    env.mem.write(buf, stats);

    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(mkdir(_, _)),
    export_c_func!(fstat(_, _)),
    export_c_func!(stat(_, _)),
    export_c_func!(statfs(_, _)),
];
