/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `stdio.h`

use super::posix_io::{self, O_APPEND, O_CREAT, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY};
use crate::dyld::{export_c_func, FunctionExports};
use crate::fs::GuestPath;
use crate::libc::string::strlen;
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr, SafeRead};
use crate::Environment;
use std::io::Write;

// Standard C functions

pub mod printf;

const EOF: i32 = -1;

#[allow(clippy::upper_case_acronyms)]
/// C `FILE` struct. This is an opaque type in C, so the definition here is our
/// own.
struct FILE {
    fd: posix_io::FileDescriptor,
}
unsafe impl SafeRead for FILE {}

fn fopen(env: &mut Environment, filename: ConstPtr<u8>, mode: ConstPtr<u8>) -> MutPtr<FILE> {
    // all valid modes are UTF-8
    let flags = match env.mem.cstr_at_utf8(mode).unwrap() {
        "r" | "rb" => O_RDONLY,
        "r+" | "rb+" | "r+b" => O_RDWR | O_APPEND,
        "w" | "wb" => O_WRONLY | O_CREAT | O_TRUNC,
        "w+" | "wb+" | "w+b" => O_RDWR | O_CREAT | O_TRUNC,
        "a" | "ab" => O_WRONLY | O_APPEND | O_CREAT,
        "a+" | "ab+" | "a+b" => O_RDWR | O_APPEND | O_CREAT,
        // Modern C adds 'x' but that's not in the documentation so presumably
        // iPhone OS did not have it.
        other => panic!("Unexpected fopen() mode {:?}", other),
    };

    match posix_io::open_direct(env, filename, flags) {
        -1 => Ptr::null(),
        fd => env.mem.alloc_and_write(FILE { fd }),
    }
}

fn fread(
    env: &mut Environment,
    buffer: MutVoidPtr,
    item_size: GuestUSize,
    n_items: GuestUSize,
    file_ptr: MutPtr<FILE>,
) -> GuestUSize {
    let FILE { fd } = env.mem.read(file_ptr);

    // Yes, the item_size/n_items split doesn't mean anything. The C standard
    // really does expect you to just multiply and divide like this, with no
    // attempt being made to ensure a whole number are read or written!
    let total_size = item_size.checked_mul(n_items).unwrap();
    match posix_io::read(env, fd, buffer, total_size) {
        // TODO: ferror() support.
        -1 => 0,
        bytes_read => {
            let bytes_read: GuestUSize = bytes_read.try_into().unwrap();
            bytes_read / item_size
        }
    }
}

fn fgetc(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    let FILE { fd } = env.mem.read(file_ptr);
    let buffer = env.mem.alloc(1);

    match posix_io::read(env, fd, buffer, 1) {
        -1 => EOF,
        bytes_read => {
            let bytes_read: GuestUSize = bytes_read.try_into().unwrap();
            if bytes_read < 1 {
                EOF
            } else {
                let buf: MutPtr<i32> = buffer.cast();
                env.mem.read(buf)
            }
        }
    }
}

fn fputs(env: &mut Environment, str: ConstPtr<u8>, stream: MutPtr<FILE>) -> i32 {
    // TODO: this function doesn't set errno or return EOF yet
    let str_len = strlen(env, str);
    fwrite(env, str.cast(), str_len, 1, stream)
        .try_into()
        .unwrap()
}

fn fwrite(
    env: &mut Environment,
    buffer: ConstVoidPtr,
    item_size: GuestUSize,
    n_items: GuestUSize,
    file_ptr: MutPtr<FILE>,
) -> GuestUSize {
    let FILE { fd } = env.mem.read(file_ptr);

    // The comment about the item_size/n_items split in fread() applies here too
    let total_size = item_size.checked_mul(n_items).unwrap();
    match posix_io::write(env, fd, buffer, total_size) {
        // TODO: ferror() support.
        -1 => 0,
        bytes_written => {
            let bytes_written: GuestUSize = bytes_written.try_into().unwrap();
            bytes_written / item_size
        }
    }
}

const SEEK_SET: i32 = posix_io::SEEK_SET;
const SEEK_CUR: i32 = posix_io::SEEK_CUR;
const SEEK_END: i32 = posix_io::SEEK_END;
fn fseek(env: &mut Environment, file_ptr: MutPtr<FILE>, offset: i32, whence: i32) -> i32 {
    let FILE { fd } = env.mem.read(file_ptr);

    assert!([SEEK_SET, SEEK_CUR, SEEK_END].contains(&whence));
    match posix_io::lseek(env, fd, offset.into(), whence) {
        -1 => -1,
        _cur_pos => 0,
    }
}

fn ftell(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    let FILE { fd } = env.mem.read(file_ptr);

    match posix_io::lseek(env, fd, 0, posix_io::SEEK_CUR) {
        -1 => -1,
        // TODO: What's the correct behaviour if the position is beyond 2GiB?
        cur_pos => cur_pos.try_into().unwrap(),
    }
}

fn fclose(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    let FILE { fd } = env.mem.read(file_ptr);

    env.mem.free(file_ptr.cast());

    match posix_io::close(env, fd) {
        0 => 0,
        -1 => EOF,
        _ => unreachable!(),
    }
}

fn feof(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    let FILE { fd } = env.mem.read(file_ptr);
    posix_io::eof(env, fd)
}

fn puts(env: &mut Environment, s: ConstPtr<u8>) -> i32 {
    let _ = std::io::stdout().write_all(env.mem.cstr_at(s));
    let _ = std::io::stdout().write_all(b"\n");
    // TODO: I/O error handling
    // TODO: is this the return value iPhone OS uses?
    0
}

fn putchar(_env: &mut Environment, c: u8) -> i32 {
    let _ = std::io::stdout().write(std::slice::from_ref(&c));
    0
}

fn remove(env: &mut Environment, path: ConstPtr<u8>) -> i32 {
    match env
        .fs
        .remove(GuestPath::new(&env.mem.cstr_at_utf8(path).unwrap()))
    {
        Ok(()) => {
            log_dbg!("remove({:?}) => 0", path);
            0
        }
        Err(_) => {
            // TODO: set errno
            log!("Warning: remove({:?}) failed, returning -1", path);
            -1
        }
    }
}

// POSIX-specific functions

fn fileno(env: &mut Environment, file_ptr: MutPtr<FILE>) -> posix_io::FileDescriptor {
    let FILE { fd } = env.mem.read(file_ptr);
    fd
}

pub const FUNCTIONS: FunctionExports = &[
    // Standard C functions
    export_c_func!(fopen(_, _)),
    export_c_func!(fread(_, _, _, _)),
    export_c_func!(fgetc(_)),
    export_c_func!(fputs(_, _)),
    export_c_func!(fwrite(_, _, _, _)),
    export_c_func!(fseek(_, _, _)),
    export_c_func!(ftell(_)),
    export_c_func!(feof(_)),
    export_c_func!(fclose(_)),
    export_c_func!(puts(_)),
    export_c_func!(putchar(_)),
    export_c_func!(remove(_)),
    // POSIX-specific functions
    export_c_func!(fileno(_)),
];
