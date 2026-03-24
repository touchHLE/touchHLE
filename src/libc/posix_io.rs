/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 */
//! POSIX I/O functions (`fcntl.h`, parts of `unistd.h`, etc)

pub mod stat;
pub mod statvfs;

use crate::abi::DotDotDot;
use crate::dyld::{export_c_func, FunctionExports};
use crate::fs::{GuestFile, GuestOpenOptions, GuestPath};
use crate::libc::errno::{set_errno, EBADF, EINTR, EINVAL, EIO, EISDIR, EOVERFLOW, ESPIPE};
use crate::libc::sys::socket::close_socket;
use crate::libc::unistd::pid_t;
use crate::mem::{
    ConstPtr, ConstVoidPtr, GuestISize, GuestUSize, MutPtr, MutVoidPtr, Ptr, SafeRead,
};
use crate::Environment;
use std::io::{Read, Seek, SeekFrom, Write};

/// ======================= STATE =======================

#[derive(Default)]
pub struct State {
    pub files: Vec<Option<PosixFileHostObject>>,
}

impl State {
    fn file_for_fd(&mut self, fd: FileDescriptor) -> Option<&mut PosixFileHostObject> {
        self.files
            .get_mut(fd_to_file_idx(fd))
            .and_then(|f| f.as_mut())
    }
}

pub struct PosixFileHostObject {
    file: GuestFile,
    needs_flush: bool,
    reached_eof: bool,
    flags: i32,
}

/// ======================= FD =======================

pub type FileDescriptor = i32;

pub const STDIN_FILENO: FileDescriptor = 0;
pub const STDOUT_FILENO: FileDescriptor = 1;
pub const STDERR_FILENO: FileDescriptor = 2;

const NORMAL_FILENO_BASE: FileDescriptor = 3;

fn file_idx_to_fd(idx: usize) -> FileDescriptor {
    idx as i32 + NORMAL_FILENO_BASE
}

fn fd_to_file_idx(fd: FileDescriptor) -> usize {
    (fd - NORMAL_FILENO_BASE) as usize
}

/// ======================= FLAGS =======================

pub type OpenFlag = i32;

pub const O_RDONLY: OpenFlag = 0;
pub const O_WRONLY: OpenFlag = 1;
pub const O_RDWR: OpenFlag = 2;

pub const O_CREAT: OpenFlag = 0x200;
pub const O_TRUNC: OpenFlag = 0x400;
pub const O_APPEND: OpenFlag = 0x8;

/// ======================= OPEN =======================

fn open(env: &mut Environment, path: ConstPtr<u8>, flags: i32, _args: DotDotDot) -> i32 {
    open_direct(env, path, flags)
}

pub fn open_direct(env: &mut Environment, path: ConstPtr<u8>, flags: i32) -> i32 {
    if path.is_null() {
        return -1;
    }

    let path = match env.mem.cstr_at_utf8(path) {
        Ok(p) => p.to_string(),
        Err(_) => return -1,
    };

    let mut options = GuestOpenOptions::new();

    match flags & 3 {
        O_RDONLY => { options.read(); }
        O_WRONLY => { options.write(); }
        O_RDWR => { options.read().write(); }
        _ => {}
    }

    if flags & O_CREAT != 0 {
        options.create();
    }

    if flags & O_TRUNC != 0 {
        options.truncate();
    }

    if flags & O_APPEND != 0 {
        options.append();
    }

    match env.fs.open_with_options(GuestPath::new(&path), options) {
        Ok(file) => {
            let obj = PosixFileHostObject {
                file,
                needs_flush: true,
                reached_eof: false,
                flags: 0,
            };
            find_or_create_fd(env, obj)
        }
        Err(_) => -1,
    }
}

/// ======================= READ/WRITE =======================

pub fn read(
    env: &mut Environment,
    fd: FileDescriptor,
    buffer: MutVoidPtr,
    size: GuestUSize,
) -> GuestISize {
    let file = match env.libc_state.posix_io.file_for_fd(fd) {
        Some(f) => f,
        None => return -1,
    };

    let buf = env.mem.bytes_at_mut(buffer.cast(), size);

    match file.file.read(buf) {
        Ok(n) => n as GuestISize,
        Err(_) => -1,
    }
}

pub fn write(
    env: &mut Environment,
    fd: FileDescriptor,
    buffer: ConstVoidPtr,
    size: GuestUSize,
) -> GuestISize {
    let file = env.libc_state.posix_io.file_for_fd(fd).unwrap();

    let buf = env.mem.bytes_at(buffer.cast(), size);

    match file.file.write(buf) {
        Ok(n) => n as GuestISize,
        Err(_) => -1,
    }
}

/// ======================= SEEK =======================

pub type off_t = i64;

pub const SEEK_SET: i32 = 0;
pub const SEEK_CUR: i32 = 1;
pub const SEEK_END: i32 = 2;

pub fn lseek(
    env: &mut Environment,
    fd: FileDescriptor,
    offset: off_t,
    whence: i32,
) -> off_t {
    let file = match env.libc_state.posix_io.file_for_fd(fd) {
        Some(f) => f,
        None => return -1,
    };

    let res = match whence {
        SEEK_SET => file.file.seek(SeekFrom::Start(offset as u64)),
        SEEK_CUR => file.file.seek(SeekFrom::Current(offset)),
        SEEK_END => file.file.seek(SeekFrom::End(offset)),
        _ => return -1,
    };

    match res {
        Ok(pos) => pos as off_t,
        Err(_) => -1,
    }
}

/// ======================= CLOSE =======================

pub fn close(env: &mut Environment, fd: FileDescriptor) -> i32 {
    if fd < NORMAL_FILENO_BASE {
        return 0;
    }

    if fd_to_file_idx(fd) >= env.libc_state.posix_io.files.len() {
        return -1;
    }

    env.libc_state.posix_io.files[fd_to_file_idx(fd)] = None;
    0
}

/// ======================= UTILS =======================

fn find_or_create_fd(
    env: &mut Environment,
    obj: PosixFileHostObject,
) -> FileDescriptor {
    let files = &mut env.libc_state.posix_io.files;

    if let Some(i) = files.iter().position(|f| f.is_none()) {
        files[i] = Some(obj);
        file_idx_to_fd(i)
    } else {
        files.push(Some(obj));
        file_idx_to_fd(files.len() - 1)
    }
}

/// ======================= EXPORTS =======================

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(open(_, _, _)),
    export_c_func!(read(_, _, _)),
    export_c_func!(write(_, _, _)),
    export_c_func!(lseek(_, _, _)),
    export_c_func!(close(_)),
];
