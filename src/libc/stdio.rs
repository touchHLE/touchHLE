/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `stdio.h`

use super::posix_io::{
    self, off_t, O_APPEND, O_CREAT, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY, STDERR_FILENO,
    STDIN_FILENO, STDOUT_FILENO,
};
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::fs::GuestPath;
use crate::libc::string::strlen;
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, Mem, MutPtr, MutVoidPtr, Ptr, SafeRead};
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

#[allow(non_camel_case_types)]
type fpos_t = off_t;

fn fopen(env: &mut Environment, filename: ConstPtr<u8>, mode: ConstPtr<u8>) -> MutPtr<FILE> {
    // Some testing on macOS suggests Apple's implementation will just ignore
    // flags it doesn't know about, and unfortunately real-world apps seem to
    // rely on this, e.g. using "wt" to mean open for writing in text mode,
    // even though that's not a real flag. The one thing that is required is for
    // a known basic mode (r/w/a) to come first.

    let mode = env.mem.cstr_at(mode);
    let [basic_mode @ (b'r' | b'w' | b'a'), flags @ ..] = mode else {
        panic!(
            "Unexpected or missing fopen() mode first character: {:?}",
            mode.first()
        );
    };
    let mut plus = false;
    for &flag in flags {
        match flag {
            // binary flag does nothing on UNIX
            b'b' => (),
            b'+' => plus = true,
            other => {
                log!("Tolerating unrecognized fopen() mode flag: {:?}", other);
            }
        }
    }

    let flags = match (basic_mode, plus) {
        (b'r', false) => O_RDONLY,
        (b'r', true) => O_RDWR | O_APPEND,
        (b'w', false) => O_WRONLY | O_CREAT | O_TRUNC,
        (b'w', true) => O_RDWR | O_CREAT | O_TRUNC,
        (b'a', false) => O_WRONLY | O_APPEND | O_CREAT,
        (b'a', true) => O_RDWR | O_APPEND | O_CREAT,
        _ => unreachable!(),
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
    if item_size == 0 {
        return 0;
    }

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

fn fgets(
    env: &mut Environment,
    str: MutPtr<u8>,
    size: GuestUSize,
    stream: MutPtr<FILE>,
) -> MutPtr<u8> {
    let mut read = 0;
    let mut tmp = str;
    while read < size && fread(env, tmp.cast(), 1, 1, stream) != 0 {
        tmp += 1;
        read += 1;
        if env.mem.read(tmp - 1) == b'\n' {
            break;
        }
    }

    if read == 0 {
        return Ptr::null();
    } else {
        env.mem.write(tmp, b'\0');
    }
    str
}

fn fputs(env: &mut Environment, str: ConstPtr<u8>, stream: MutPtr<FILE>) -> i32 {
    // TODO: this function doesn't set errno or return EOF yet
    let str_len = strlen(env, str);
    fwrite(env, str.cast(), str_len, 1, stream)
        .try_into()
        .unwrap()
}

fn fputc(env: &mut Environment, c: i32, stream: MutPtr<FILE>) -> i32 {
    let ptr: MutPtr<u8> = env.mem.alloc_and_write(c.try_into().unwrap());
    let res = fwrite(env, ptr.cast_const().cast(), 1, 1, stream)
        .try_into()
        .unwrap();
    env.mem.free(ptr.cast());
    res
}

fn fwrite(
    env: &mut Environment,
    buffer: ConstVoidPtr,
    item_size: GuestUSize,
    n_items: GuestUSize,
    file_ptr: MutPtr<FILE>,
) -> GuestUSize {
    if item_size == 0 || buffer.is_null() {
        return 0;
    }

    let FILE { fd } = env.mem.read(file_ptr);

    let total_size = item_size.checked_mul(n_items).unwrap();

    // TODO: Refactor, use traits instead of this hack
    match fd {
        STDOUT_FILENO => {
            let buffer_slice = env.mem.bytes_at(buffer.cast(), total_size);
            match std::io::stdout().write(buffer_slice) {
                Ok(bytes_written) => (bytes_written / (item_size as usize)) as GuestUSize,
                Err(_err) => 0,
            }
        }
        STDERR_FILENO => {
            let buffer_slice = env.mem.bytes_at(buffer.cast(), total_size);
            match std::io::stderr().write(buffer_slice) {
                Ok(bytes_written) => (bytes_written / (item_size as usize)) as GuestUSize,
                Err(_err) => 0,
            }
        }
        _ => {
            // The comment about the item_size/n_items split in fread() applies
            // here too.
            match posix_io::write(env, fd, buffer, total_size) {
                // TODO: ferror() support.
                -1 => 0,
                bytes_written => {
                    let bytes_written: GuestUSize = bytes_written.try_into().unwrap();
                    bytes_written / item_size
                }
            }
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

fn rewind(env: &mut Environment, file_ptr: MutPtr<FILE>) {
    fseek(env, file_ptr, 0, SEEK_SET);
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

fn fsetpos(env: &mut Environment, file_ptr: MutPtr<FILE>, pos: ConstPtr<fpos_t>) -> i32 {
    let FILE { fd } = env.mem.read(file_ptr);

    let res = posix_io::lseek(env, fd, env.mem.read(pos), SEEK_SET);
    if res == -1 {
        -1
    } else {
        0
    }
}

fn fgetpos(env: &mut Environment, file_ptr: MutPtr<FILE>, pos: MutPtr<fpos_t>) -> i32 {
    let FILE { fd } = env.mem.read(file_ptr);

    let res = posix_io::lseek(env, fd, 0, posix_io::SEEK_CUR);
    if res == -1 {
        return -1;
    }
    env.mem.write(pos, res);
    0
}

fn feof(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    let FILE { fd } = env.mem.read(file_ptr);
    posix_io::eof(env, fd)
}

fn clearerr(env: &mut Environment, file_ptr: MutPtr<FILE>) {
    let FILE { fd } = env.mem.read(file_ptr);
    posix_io::clearerr(env, fd)
}

fn fflush(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    let FILE { fd } = env.mem.read(file_ptr);
    posix_io::fflush(env, fd)
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
    if Ptr::is_null(path) {
        // TODO: set errno
        log!("remove({:?}) => -1, attempted to remove null", path);
        return -1;
    }

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

fn setbuf(_env: &mut Environment, stream: MutPtr<FILE>, buf: ConstPtr<u8>) {
    assert!(buf.is_null());
    log!(
        "Warning: ignoring a setbuf() for {:?} with NULL (unbuffered)",
        stream
    );
}

// POSIX-specific functions

fn fileno(env: &mut Environment, file_ptr: MutPtr<FILE>) -> posix_io::FileDescriptor {
    let FILE { fd } = env.mem.read(file_ptr);
    fd
}

pub const CONSTANTS: ConstantExports = &[
    (
        "___stdinp",
        HostConstant::Custom(|mem: &mut Mem| -> ConstVoidPtr {
            let ptr = mem.alloc_and_write(FILE { fd: STDIN_FILENO });
            mem.alloc_and_write(ptr).cast().cast_const()
        }),
    ),
    (
        "___stdoutp",
        HostConstant::Custom(|mem: &mut Mem| -> ConstVoidPtr {
            let ptr = mem.alloc_and_write(FILE { fd: STDOUT_FILENO });
            mem.alloc_and_write(ptr).cast().cast_const()
        }),
    ),
    (
        "___stderrp",
        HostConstant::Custom(|mem: &mut Mem| -> ConstVoidPtr {
            let ptr = mem.alloc_and_write(FILE { fd: STDERR_FILENO });
            mem.alloc_and_write(ptr).cast().cast_const()
        }),
    ),
];

pub const FUNCTIONS: FunctionExports = &[
    // Standard C functions
    export_c_func!(fopen(_, _)),
    export_c_func!(fread(_, _, _, _)),
    export_c_func!(fgetc(_)),
    export_c_func!(fgets(_, _, _)),
    export_c_func!(fputs(_, _)),
    export_c_func!(fputc(_, _)),
    export_c_func!(fwrite(_, _, _, _)),
    export_c_func!(fseek(_, _, _)),
    export_c_func!(ftell(_)),
    export_c_func!(rewind(_)),
    export_c_func!(fsetpos(_, _)),
    export_c_func!(fgetpos(_, _)),
    export_c_func!(feof(_)),
    export_c_func!(clearerr(_)),
    export_c_func!(fflush(_)),
    export_c_func!(fclose(_)),
    export_c_func!(puts(_)),
    export_c_func!(putchar(_)),
    export_c_func!(remove(_)),
    export_c_func!(setbuf(_, _)),
    // POSIX-specific functions
    export_c_func!(fileno(_)),
];
