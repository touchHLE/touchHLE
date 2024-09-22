/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! POSIX I/O functions (`fcntl.h`, parts of `unistd.h`, etc)

pub mod stat;

use crate::abi::DotDotDot;
use crate::dyld::{export_c_func, FunctionExports};
use crate::fs::{GuestFile, GuestOpenOptions, GuestPath};
use crate::libc::errno::{set_errno, EBADF};
use crate::mem::{ConstPtr, ConstVoidPtr, GuestISize, GuestUSize, MutPtr, MutVoidPtr, Ptr};
use crate::Environment;
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Default)]
pub struct State {
    /// File descriptors _other than stdin, stdout, and stderr_
    files: Vec<Option<PosixFileHostObject>>,
}
impl State {
    fn file_for_fd(&mut self, fd: FileDescriptor) -> Option<&mut PosixFileHostObject> {
        self.files
            .get_mut(fd_to_file_idx(fd))
            .and_then(|file_or_none| file_or_none.as_mut())
    }
}

struct PosixFileHostObject {
    file: GuestFile,
    needs_flush: bool,
    reached_eof: bool,
}

// TODO: stdin/stdout/stderr handling somehow
fn file_idx_to_fd(idx: usize) -> FileDescriptor {
    FileDescriptor::try_from(idx)
        .unwrap()
        .checked_add(NORMAL_FILENO_BASE)
        .unwrap()
}
fn fd_to_file_idx(fd: FileDescriptor) -> usize {
    fd.checked_sub(NORMAL_FILENO_BASE).unwrap() as usize
}

/// File descriptor type. This alias is for readability, POSIX just uses `int`.
pub type FileDescriptor = i32;
pub const STDIN_FILENO: FileDescriptor = 0;
pub const STDOUT_FILENO: FileDescriptor = 1;
pub const STDERR_FILENO: FileDescriptor = 2;
const NORMAL_FILENO_BASE: FileDescriptor = STDERR_FILENO + 1;

/// Flags bitfield for `open`. This alias is for readability, POSIX just uses
/// `int`.
pub type OpenFlag = i32;
pub const O_RDONLY: OpenFlag = 0x0;
pub const O_WRONLY: OpenFlag = 0x1;
pub const O_RDWR: OpenFlag = 0x2;
pub const O_ACCMODE: OpenFlag = O_RDWR | O_WRONLY | O_RDONLY;

pub const O_NONBLOCK: OpenFlag = 0x4;
pub const O_APPEND: OpenFlag = 0x8;
pub const O_SHLOCK: OpenFlag = 0x10;
pub const O_NOFOLLOW: OpenFlag = 0x100;
pub const O_CREAT: OpenFlag = 0x200;
pub const O_TRUNC: OpenFlag = 0x400;
pub const O_EXCL: OpenFlag = 0x800;

/// File control command flags.
/// This alias is for readability, POSIX just uses `int`.
pub type FileControlCommand = i32;
const F_RDADVISE: FileControlCommand = 44;
const F_NOCACHE: FileControlCommand = 48;

pub type FLockFlag = i32;
pub const LOCK_SH: FLockFlag = 1;
#[allow(dead_code)]
pub const LOCK_EX: FLockFlag = 2;
#[allow(dead_code)]
pub const LOCK_NB: FLockFlag = 4;
#[allow(dead_code)]
pub const LOCK_UN: FLockFlag = 8;

fn open(env: &mut Environment, path: ConstPtr<u8>, flags: i32, _args: DotDotDot) -> FileDescriptor {
    // TODO: handle errno properly
    set_errno(env, 0);

    // TODO: parse variadic arguments and pass them on (file creation mode)
    self::open_direct(env, path, flags)
}

/// Special extension for host code: [open] without the [DotDotDot].
pub fn open_direct(env: &mut Environment, path: ConstPtr<u8>, flags: i32) -> FileDescriptor {
    // TODO: support more flags, this list is not complete
    assert!(
        flags
            & !(O_ACCMODE
                | O_NONBLOCK
                | O_APPEND
                | O_SHLOCK
                | O_NOFOLLOW
                | O_CREAT
                | O_TRUNC
                | O_EXCL)
            == 0
    );
    // TODO: exclusive mode not implemented yet
    assert!(flags & O_EXCL == 0);

    if path.is_null() {
        log_dbg!("open({:?}, {:#x}) => -1", path, flags);
        return -1; // TODO: set errno to EFAULT
    }

    // TODO: respect the mode (in the variadic arguments) when creating a file
    // Note: NONBLOCK flag is ignored, assumption is all file I/O is fast
    let mut needs_flush = false;
    let mut options = GuestOpenOptions::new();
    match flags & O_ACCMODE {
        O_RDONLY => {
            options.read();
        }
        O_WRONLY => {
            options.write();
            needs_flush = true;
        }
        O_RDWR => {
            options.read().write();
            needs_flush = true;
        }
        _ => panic!(),
    };
    if (flags & O_APPEND) != 0 {
        options.append();
    }
    if (flags & O_CREAT) != 0 {
        options.create();
    }
    if (flags & O_TRUNC) != 0 {
        options.truncate();
    }

    let path_string = match env.mem.cstr_at_utf8(path) {
        Ok(path_str) => path_str.to_owned(),
        Err(err) => {
            log!(
                "open() error, unable to treat {:?} as utf8 str: {:?}",
                path,
                err
            );
            // TODO: set errno
            return -1;
        }
    };
    // TODO: symlinks don't exist in the FS yet, so we can't "not follow" them.
    if flags & O_NOFOLLOW != 0 {
        log!("Ignoring O_NOFOLLOW when opening {:?}", path_string);
    }
    let res = match env
        .fs
        .open_with_options(GuestPath::new(&path_string), options)
    {
        Ok(file) => {
            let host_object = PosixFileHostObject {
                file,
                needs_flush,
                reached_eof: false,
            };

            let idx = if let Some(free_idx) = env
                .libc_state
                .posix_io
                .files
                .iter()
                .position(|f| f.is_none())
            {
                env.libc_state.posix_io.files[free_idx] = Some(host_object);
                free_idx
            } else {
                let idx = env.libc_state.posix_io.files.len();
                env.libc_state.posix_io.files.push(Some(host_object));
                idx
            };
            file_idx_to_fd(idx)
        }
        Err(()) => {
            // TODO: set errno
            -1
        }
    };
    if res != -1 && (flags & O_SHLOCK) != 0 {
        // TODO: Handle possible errors
        flock(env, res, LOCK_SH);
    }
    log_dbg!(
        "open({:?} {:?}, {:#x}) => {:?}",
        path,
        path_string,
        flags,
        res
    );
    res
}

pub fn read(
    env: &mut Environment,
    fd: FileDescriptor,
    buffer: MutVoidPtr,
    size: GuestUSize,
) -> GuestISize {
    // TODO: handle errno properly
    set_errno(env, 0);

    if buffer.is_null() {
        // TODO: set errno to EFAULT
        return -1;
    }

    // TODO: error handling for unknown fd?
    let file = env.libc_state.posix_io.file_for_fd(fd).unwrap();

    let buffer_slice = env.mem.bytes_at_mut(buffer.cast(), size);
    match file.file.read(buffer_slice) {
        Ok(bytes_read) => {
            if bytes_read == 0 && size != 0 {
                // need to set EOF
                file.reached_eof = true;
            }
            if bytes_read < buffer_slice.len() {
                log!(
                    "Warning: read({:?}, {:?}, {:#x}) read only {:#x} bytes",
                    fd,
                    buffer,
                    size,
                    bytes_read,
                );
            } else {
                log_dbg!(
                    "read({:?}, {:?}, {:#x}) => {:#x}",
                    fd,
                    buffer,
                    size,
                    bytes_read,
                );
            }
            bytes_read.try_into().unwrap()
        }
        Err(e) => {
            // TODO: set errno
            log!(
                "Warning: read({:?}, {:?}, {:#x}) encountered error {:?}, returning -1",
                fd,
                buffer,
                size,
                e,
            );
            -1
        }
    }
}

/// Helper for C `feof()`.
pub(super) fn eof(env: &mut Environment, fd: FileDescriptor) -> i32 {
    let file = env.libc_state.posix_io.file_for_fd(fd).unwrap();
    if file.reached_eof {
        1
    } else {
        0
    }
}

/// Helper for C `clearerr()`.
pub(super) fn clearerr(env: &mut Environment, fd: FileDescriptor) {
    // TODO: handle errno properly
    set_errno(env, 0);

    let file = env.libc_state.posix_io.file_for_fd(fd).unwrap();
    file.reached_eof = false;
}

/// Helper for C `fflush()`.
pub(super) fn fflush(env: &mut Environment, fd: FileDescriptor) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let Some(file) = env.libc_state.posix_io.file_for_fd(fd) else {
        // TODO: set errno to EBADF
        return -1;
    };
    match file.file.flush() {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

pub fn write(
    env: &mut Environment,
    fd: FileDescriptor,
    buffer: ConstVoidPtr,
    size: GuestUSize,
) -> GuestISize {
    // TODO: handle errno properly
    set_errno(env, 0);

    // TODO: error handling for unknown fd?
    let file = env.libc_state.posix_io.file_for_fd(fd).unwrap();

    let buffer_slice = env.mem.bytes_at(buffer.cast(), size);
    match file.file.write(buffer_slice) {
        Ok(bytes_written) => {
            if bytes_written < buffer_slice.len() {
                log!(
                    "Warning: write({:?}, {:?}, {:#x}) wrote only {:#x} bytes",
                    fd,
                    buffer,
                    size,
                    bytes_written,
                );
            } else {
                log_dbg!(
                    "write({:?}, {:?}, {:#x}) => {:#x}",
                    fd,
                    buffer,
                    size,
                    bytes_written,
                );
            }
            bytes_written.try_into().unwrap()
        }
        Err(e) => {
            // TODO: set errno
            log!(
                "Warning: write({:?}, {:?}, {:#x}) encountered error {:?}, returning -1",
                fd,
                buffer,
                size,
                e,
            );
            -1
        }
    }
}

#[allow(non_camel_case_types)]
pub type off_t = i64;
pub const SEEK_SET: i32 = 0;
pub const SEEK_CUR: i32 = 1;
pub const SEEK_END: i32 = 2;
pub fn lseek(env: &mut Environment, fd: FileDescriptor, offset: off_t, whence: i32) -> off_t {
    // TODO: handle errno properly
    set_errno(env, 0);

    // TODO: error handling for unknown fd?
    let file = env.libc_state.posix_io.file_for_fd(fd).unwrap();

    let from = match whence {
        // not sure whether offset is treated as signed or unsigned when using
        // SEEK_SET, so `.try_into()` seems safer.
        SEEK_SET => SeekFrom::Start(offset.try_into().unwrap()),
        SEEK_CUR => SeekFrom::Current(offset),
        SEEK_END => SeekFrom::End(offset),
        _ => panic!("Unsupported \"whence\" parameter to seek(): {}", whence),
    };

    let res = match file.file.seek(from) {
        Ok(new_offset) => {
            // "A successful call to the fseek() function clears
            // the end-of-file indicator for the stream..."
            file.reached_eof = false;

            new_offset.try_into().unwrap()
        }
        // TODO: set errno
        Err(_) => -1,
    };
    log_dbg!("lseek({:?}, {:#x}, {}) => {}", fd, offset, whence, res);
    res
}

pub fn close(env: &mut Environment, fd: FileDescriptor) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    // TODO: error handling for unknown fd?
    if fd < 0 || matches!(fd, STDOUT_FILENO | STDERR_FILENO) {
        return 0;
    }

    let result = match env.libc_state.posix_io.files[fd_to_file_idx(fd)].take() {
        Some(file) => {
            // The actual closing of the file happens implicitly when `file`
            // falls out of scope. The return value is about whether actions
            // performed before closing succeed or not.
            match file.file {
                // Closing directories requires no other actions
                GuestFile::Directory => 0,
                // Files must be synced if they require flushing
                _ => {
                    if !file.needs_flush {
                        0
                    } else {
                        match file.file.sync_all() {
                            Ok(()) => 0,
                            Err(_) => {
                                // TODO: set errno
                                -1
                            }
                        }
                    }
                }
            }
        }
        None => {
            // TODO: set errno
            -1
        }
    };

    if result == 0 {
        log_dbg!("close({:?}) => 0", fd);
    } else {
        log!("Warning: close({:?}) failed, returning -1", fd);
    }
    result
}

fn rename(env: &mut Environment, old: ConstPtr<u8>, new: ConstPtr<u8>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let old = env.mem.cstr_at_utf8(old).unwrap();
    let new = env.mem.cstr_at_utf8(new).unwrap();
    log_dbg!("rename('{}', '{}')", old, new);
    match env.fs.rename(GuestPath::new(&old), GuestPath::new(&new)) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

pub fn getcwd(env: &mut Environment, buf_ptr: MutPtr<u8>, buf_size: GuestUSize) -> MutPtr<u8> {
    let working_directory = env.fs.working_directory();
    if !env.fs.is_dir(working_directory) {
        // TODO: set errno to ENOENT
        log!(
            "Warning: getcwd({:?}, {:#x}) failed, returning NULL",
            buf_ptr,
            buf_size
        );
        return Ptr::null();
    }

    let working_directory = env.fs.working_directory().as_str().as_bytes();

    if buf_ptr.is_null() {
        // The buffer size argument is presumably ignored in this mode.
        // This mode is an extension, which might explain the strange API.
        let res = env.mem.alloc_and_write_cstr(working_directory);
        log_dbg!("getcwd(NULL, _) => {:?} ({:?})", res, working_directory);
        return res;
    }

    // Includes space for null terminator
    let res_size: GuestUSize = u32::try_from(working_directory.len()).unwrap() + 1;

    if buf_size < res_size {
        // TODO: set errno to EINVAL or ERANGE as appropriate
        log!(
            "Warning: getcwd({:?}, {:#x}) failed, returning NULL",
            buf_ptr,
            buf_size
        );
        return Ptr::null();
    }

    let buf = env.mem.bytes_at_mut(buf_ptr, res_size);
    buf[..(res_size - 1) as usize].copy_from_slice(working_directory);
    buf[(res_size - 1) as usize] = b'\0';

    log_dbg!(
        "getcwd({:?}, {:#x}) => {:?}, wrote {:?} ({:#x} bytes)",
        buf_ptr,
        buf_size,
        buf_ptr,
        working_directory,
        res_size
    );
    buf_ptr
}

fn chdir(env: &mut Environment, path_ptr: ConstPtr<u8>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let path = GuestPath::new(env.mem.cstr_at_utf8(path_ptr).unwrap());
    match env.fs.change_working_directory(path) {
        Ok(new) => {
            log_dbg!(
                "chdir({:?}) => 0, new working directory: {:?}",
                path_ptr,
                new,
            );
            0
        }
        Err(()) => {
            log!("Warning: chdir({:?}) failed, could not change working directory to {:?}, returning -1", path_ptr, path);
            // TODO: set errno
            -1
        }
    }
}
// TODO: fchdir(), once open() on a directory is supported.

fn fcntl(
    env: &mut Environment,
    fd: FileDescriptor,
    cmd: FileControlCommand,
    args: DotDotDot,
) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    if fd >= NORMAL_FILENO_BASE
        && env
            .libc_state
            .posix_io
            .files
            .get(fd_to_file_idx(fd))
            .is_none()
    {
        set_errno(env, EBADF);
        return -1;
    }

    match cmd {
        F_NOCACHE => {
            let mut args = args.start();
            let arg: i32 = args.next(env);
            assert_eq!(arg, 1);
            log!(
                "TODO: Ignoring enabling F_NOCACHE for file descriptor {}",
                fd
            );
        }
        F_RDADVISE => {
            log_dbg!("TODO: Ignoring F_RDADVISE for file descriptor {}", fd);
        }
        _ => unimplemented!(),
    }
    0 // success
}

fn flock(env: &mut Environment, fd: FileDescriptor, operation: FLockFlag) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    log!("TODO: flock({:?}, {:?})", fd, operation);
    0
}

fn ftruncate(env: &mut Environment, fd: FileDescriptor, len: off_t) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let file = env.libc_state.posix_io.file_for_fd(fd).unwrap();
    match file.file.set_len(len as u64) {
        Ok(()) => 0,
        Err(_) => -1, // TODO: set errno
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(open(_, _, _)),
    export_c_func!(read(_, _, _)),
    export_c_func!(write(_, _, _)),
    export_c_func!(lseek(_, _, _)),
    export_c_func!(close(_)),
    export_c_func!(rename(_, _)),
    export_c_func!(getcwd(_, _)),
    export_c_func!(chdir(_)),
    export_c_func!(fcntl(_, _, _)),
    export_c_func!(flock(_, _)),
    export_c_func!(ftruncate(_, _)),
];
