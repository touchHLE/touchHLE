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
use crate::libc::errno::{set_errno, EBUSY};
use crate::libc::string::strlen;
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, Mem, MutPtr, MutVoidPtr, Ptr, SafeRead};
use crate::Environment;

use crate::environment::{ThreadBlock, ThreadId};
use std::collections::HashMap;
use std::io::Write;

// Standard C functions

pub mod printf;

const EOF: i32 = -1;

struct FILEHostObject {
    /// `ungetc()` implementation
    pushbacks: Vec<u8>,
    lock_count: u32,
    owning_thread: Option<ThreadId>,
}

#[allow(clippy::upper_case_acronyms)]
/// C `FILE` struct. This is an opaque type in C, so the definition here is our
/// own.
pub(crate) struct FILE {
    fd: posix_io::FileDescriptor,
}
unsafe impl SafeRead for FILE {}

#[derive(Default)]
pub struct State {
    file_streams: HashMap<MutPtr<FILE>, FILEHostObject>,
}
impl State {
    fn get_mut(env: &mut Environment) -> &mut Self {
        &mut env.libc_state.stdio
    }
    fn get_file_host_obj_mut(
        &mut self,
        mem: &mut Mem,
        file_ptr: MutPtr<FILE>,
    ) -> &mut FILEHostObject {
        let FILE { fd } = mem.read(file_ptr);
        if matches!(fd, STDIN_FILENO | STDOUT_FILENO | STDERR_FILENO)
            && !self.file_streams.contains_key(&file_ptr)
        {
            // Special case, need to do a lazy creation of host object
            self.file_streams.insert(
                file_ptr,
                FILEHostObject {
                    pushbacks: Vec::new(),
                    lock_count: 0,
                    owning_thread: None,
                },
            );
        }
        self.file_streams.get_mut(&file_ptr).unwrap()
    }
    pub(crate) fn try_acquire_file_object_lock(
        &mut self,
        mem: &mut Mem,
        file_ptr: MutPtr<FILE>,
        thread_id: ThreadId,
    ) -> bool {
        let FILEHostObject {
            lock_count,
            owning_thread,
            ..
        } = self.get_file_host_obj_mut(mem, file_ptr);
        if *lock_count == 0 {
            assert!(owning_thread.is_none());
            *lock_count = 1;
            *owning_thread = Some(thread_id);
            true
        } else {
            false
        }
    }
}

fn _touchHLE_check_file_object_lock(env: &mut Environment, file_ptr: MutPtr<FILE>) {
    let FILEHostObject {
        lock_count,
        owning_thread,
        ..
    } = env
        .libc_state
        .stdio
        .get_file_host_obj_mut(&mut env.mem, file_ptr);
    // Technically, stdio should use same locking mechanism as `flockfile`.
    // Practically, we don't need that as we are single host threaded.
    //
    // Still, an app can call a stdio function with currently
    // `flockfile`-locked FILE object from another thread.
    // TODO: handle indirect locking
    assert!(
        (owning_thread.is_none() && *lock_count == 0) || *owning_thread == Some(env.current_thread)
    );
}

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
        (b'r', true) => O_RDWR,
        (b'w', false) => O_WRONLY | O_CREAT | O_TRUNC,
        (b'w', true) => O_RDWR | O_CREAT | O_TRUNC,
        (b'a', false) => O_WRONLY | O_APPEND | O_CREAT,
        (b'a', true) => O_RDWR | O_APPEND | O_CREAT,
        _ => unreachable!(),
    };

    match posix_io::open_direct(env, filename, flags) {
        -1 => Ptr::null(),
        fd => {
            let res = env.mem.alloc_and_write(FILE { fd });
            assert!(!State::get_mut(env).file_streams.contains_key(&res));
            State::get_mut(env).file_streams.insert(
                res,
                FILEHostObject {
                    pushbacks: Vec::new(),
                    lock_count: 0,
                    owning_thread: None,
                },
            );
            res
        }
    }
}

fn fread(
    env: &mut Environment,
    mut buffer: MutVoidPtr,
    item_size: GuestUSize,
    n_items: GuestUSize,
    file_ptr: MutPtr<FILE>,
) -> GuestUSize {
    // TODO: handle errno properly
    set_errno(env, 0);

    _touchHLE_check_file_object_lock(env, file_ptr);

    if item_size == 0 {
        return 0;
    }

    // Yes, the item_size/n_items split doesn't mean anything. The C standard
    // really does expect you to just multiply and divide like this, with no
    // attempt being made to ensure a whole number are read or written!
    let mut total_size = item_size.checked_mul(n_items).unwrap();
    let FILEHostObject {
        ref mut pushbacks, ..
    } = env
        .libc_state
        .stdio
        .get_file_host_obj_mut(&mut env.mem, file_ptr);
    let already_read = if !pushbacks.is_empty() {
        let to_copy = pushbacks.len().min(total_size as usize);
        let offset = pushbacks.len() - to_copy;

        _ = &pushbacks[offset..].reverse();
        let to_copy: GuestUSize = to_copy.try_into().unwrap();
        env.mem
            .bytes_at_mut(buffer.cast(), to_copy)
            .copy_from_slice(&pushbacks[offset..]);
        pushbacks.truncate(offset);

        if total_size == to_copy {
            return total_size;
        }
        total_size -= to_copy;
        let ptr: MutPtr<u8> = buffer.cast();
        buffer = (ptr + to_copy).cast();
        to_copy
    } else {
        0
    };
    let FILE { fd } = env.mem.read(file_ptr);
    match posix_io::read(env, fd, buffer, total_size) {
        // TODO: ferror() support.
        -1 => already_read / item_size,
        bytes_read => {
            let bytes_read: GuestUSize = bytes_read.try_into().unwrap();
            (bytes_read + already_read) / item_size
        }
    }
}

fn fgetc(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    _touchHLE_check_file_object_lock(env, file_ptr);
    let FILE { fd } = env.mem.read(file_ptr);
    let FILEHostObject {
        ref mut pushbacks, ..
    } = env
        .libc_state
        .stdio
        .get_file_host_obj_mut(&mut env.mem, file_ptr);
    if let Some(pushback) = pushbacks.pop() {
        let new_offset = posix_io::lseek(env, fd, 1, SEEK_CUR);
        assert!(new_offset > 0); // TODO: handle error
        return pushback.into();
    }

    let buffer = env.mem.alloc(1);

    match posix_io::read(env, fd, buffer, 1) {
        -1 => EOF,
        bytes_read => {
            let bytes_read: GuestUSize = bytes_read.try_into().unwrap();
            if bytes_read < 1 {
                EOF
            } else {
                let buf: MutPtr<u8> = buffer.cast();
                env.mem.read(buf) as i32
            }
        }
    }
}

fn getc(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    // `getc` is essentially identical to the `fgetc`
    fgetc(env, file_ptr)
}

fn ungetc(env: &mut Environment, c: i32, file_ptr: MutPtr<FILE>) -> i32 {
    assert!(c != EOF); // TODO
    _touchHLE_check_file_object_lock(env, file_ptr);
    let FILE { fd } = env.mem.read(file_ptr);
    let curr_offset = posix_io::lseek(env, fd, 0, SEEK_CUR);
    assert!(curr_offset > 0);
    // Note: successful seeking clears EOF indicator
    let new_offset = posix_io::lseek(env, fd, -1, SEEK_CUR);
    assert!(new_offset >= 0); // TODO: handle error
    let FILEHostObject {
        ref mut pushbacks, ..
    } = env
        .libc_state
        .stdio
        .get_file_host_obj_mut(&mut env.mem, file_ptr);
    pushbacks.push(c.try_into().unwrap());
    log_dbg!("ungetc pushbacks: {:?}", pushbacks);
    c
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
    // TODO: handle errno properly
    set_errno(env, 0);

    // TODO: this function doesn't set errno or return EOF yet
    let str_len = strlen(env, str);
    fwrite(env, str.cast(), str_len, 1, stream)
        .try_into()
        .unwrap()
}

fn fputc(env: &mut Environment, c: i32, stream: MutPtr<FILE>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let ptr: MutPtr<u8> = env.mem.alloc_and_write(c.try_into().unwrap());
    let res = fwrite(env, ptr.cast_const().cast(), 1, 1, stream)
        .try_into()
        .unwrap();
    env.mem.free(ptr.cast());
    res
}

// From man page,
// `The putc() macro acts essentially identically to fputc(),
// but is a macro that expands in-line.`
fn putc(env: &mut Environment, c: i32, stream: MutPtr<FILE>) -> i32 {
    fputc(env, c, stream)
}

fn fwrite(
    env: &mut Environment,
    buffer: ConstVoidPtr,
    item_size: GuestUSize,
    n_items: GuestUSize,
    file_ptr: MutPtr<FILE>,
) -> GuestUSize {
    // TODO: handle errno properly
    set_errno(env, 0);

    if item_size == 0 || buffer.is_null() {
        return 0;
    }

    _touchHLE_check_file_object_lock(env, file_ptr);
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
    fseeko(env, file_ptr, offset.into(), whence)
}
fn fseeko(env: &mut Environment, file_ptr: MutPtr<FILE>, offset: off_t, whence: i32) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    _touchHLE_check_file_object_lock(env, file_ptr);
    let FILE { fd } = env.mem.read(file_ptr);

    assert!([SEEK_SET, SEEK_CUR, SEEK_END].contains(&whence));
    match posix_io::lseek(env, fd, offset, whence) {
        -1 => -1,
        _cur_pos => {
            let FILEHostObject {
                ref mut pushbacks, ..
            } = env
                .libc_state
                .stdio
                .get_file_host_obj_mut(&mut env.mem, file_ptr);
            pushbacks.clear();
            0
        }
    }
}

fn ftell(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    // TODO: What's the correct behaviour if the position is beyond 2GiB?
    ftello(env, file_ptr).try_into().unwrap()
}
fn ftello(env: &mut Environment, file_ptr: MutPtr<FILE>) -> off_t {
    // TODO: handle errno properly
    set_errno(env, 0);

    _touchHLE_check_file_object_lock(env, file_ptr);
    let FILE { fd } = env.mem.read(file_ptr);
    posix_io::lseek(env, fd, 0, posix_io::SEEK_CUR)
}

fn rewind(env: &mut Environment, file_ptr: MutPtr<FILE>) {
    // TODO: handle errno properly
    set_errno(env, 0);

    // Note: this call will clean pushbacks as well
    fseek(env, file_ptr, 0, SEEK_SET);
}

fn fclose(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    if file_ptr.is_null() {
        // According to the docs, this should segfault.
        // But as tested on iPhone Simulator, it doesn't
        log!("fclose(NULL) => EOF");
        return EOF;
    }

    _touchHLE_check_file_object_lock(env, file_ptr);

    // This is needed in order to force lazy instantiation
    // of stdin-like host object.
    // Why the app may need to close stdin?
    // The answer is left as an exercise for the reader.
    _ = env
        .libc_state
        .stdio
        .get_file_host_obj_mut(&mut env.mem, file_ptr);

    let FILE { fd } = env.mem.read(file_ptr);
    if matches!(fd, STDIN_FILENO | STDOUT_FILENO | STDERR_FILENO) {
        log!(
            "Warning! fclose({:?}) is called for standard descriptor {}.",
            file_ptr,
            fd
        );
    }
    assert!(State::get_mut(env).file_streams.remove(&file_ptr).is_some());

    env.mem.free(file_ptr.cast());

    match posix_io::close(env, fd) {
        0 => 0,
        -1 => EOF,
        _ => unreachable!(),
    }
}

fn ferror(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    _touchHLE_check_file_object_lock(env, file_ptr);

    log!("TODO: ferror() support.");
    0
}

fn fsetpos(env: &mut Environment, file_ptr: MutPtr<FILE>, pos: ConstPtr<fpos_t>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    _touchHLE_check_file_object_lock(env, file_ptr);
    let FILE { fd } = env.mem.read(file_ptr);

    let res = posix_io::lseek(env, fd, env.mem.read(pos), SEEK_SET);
    if res == -1 {
        -1
    } else {
        let FILEHostObject {
            ref mut pushbacks, ..
        } = env
            .libc_state
            .stdio
            .get_file_host_obj_mut(&mut env.mem, file_ptr);
        pushbacks.clear();
        0
    }
}

fn fgetpos(env: &mut Environment, file_ptr: MutPtr<FILE>, pos: MutPtr<fpos_t>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    _touchHLE_check_file_object_lock(env, file_ptr);
    let FILE { fd } = env.mem.read(file_ptr);

    let res = posix_io::lseek(env, fd, 0, posix_io::SEEK_CUR);
    if res == -1 {
        return -1;
    }
    env.mem.write(pos, res);
    0
}

fn feof(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    _touchHLE_check_file_object_lock(env, file_ptr);
    let FILE { fd } = env.mem.read(file_ptr);
    posix_io::eof(env, fd)
}

fn clearerr(env: &mut Environment, file_ptr: MutPtr<FILE>) {
    // TODO: handle errno properly
    set_errno(env, 0);

    _touchHLE_check_file_object_lock(env, file_ptr);
    let FILE { fd } = env.mem.read(file_ptr);
    posix_io::clearerr(env, fd)
}

fn fflush(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    _touchHLE_check_file_object_lock(env, file_ptr);
    let FILE { fd } = env.mem.read(file_ptr);
    posix_io::fflush(env, fd)
}

fn puts(env: &mut Environment, s: ConstPtr<u8>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let _ = std::io::stdout().write_all(env.mem.cstr_at(s));
    let _ = std::io::stdout().write_all(b"\n");
    // TODO: I/O error handling
    // TODO: is this the return value iPhone OS uses?
    0
}

fn putchar(env: &mut Environment, c: u8) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let _ = std::io::stdout().write(std::slice::from_ref(&c));
    0
}

fn remove(env: &mut Environment, path: ConstPtr<u8>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

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

fn setbuf(env: &mut Environment, file_ptr: MutPtr<FILE>, buf: ConstPtr<u8>) {
    // TODO: handle errno properly
    set_errno(env, 0);

    _touchHLE_check_file_object_lock(env, file_ptr);

    assert!(buf.is_null());
    log!(
        "Warning: ignoring a setbuf() for {:?} with NULL (unbuffered)",
        file_ptr
    );
}

// POSIX-specific functions

fn fileno(env: &mut Environment, file_ptr: MutPtr<FILE>) -> posix_io::FileDescriptor {
    _touchHLE_check_file_object_lock(env, file_ptr);
    let FILE { fd } = env.mem.read(file_ptr);
    fd
}

fn flockfile(env: &mut Environment, file_ptr: MutPtr<FILE>) {
    let FILEHostObject {
        ref mut lock_count,
        ref mut owning_thread,
        ..
    } = env
        .libc_state
        .stdio
        .get_file_host_obj_mut(&mut env.mem, file_ptr);
    match owning_thread {
        Some(thread_id) if *thread_id != env.current_thread => {
            env.yield_thread(ThreadBlock::FileObjectLock(file_ptr));
        }
        _ => {
            *lock_count = lock_count.checked_add(1).unwrap();
            *owning_thread = Some(env.current_thread);
        }
    }
}

fn ftrylockfile(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    let FILEHostObject {
        ref mut lock_count,
        ref mut owning_thread,
        ..
    } = env
        .libc_state
        .stdio
        .get_file_host_obj_mut(&mut env.mem, file_ptr);
    match owning_thread {
        Some(thread_id) if *thread_id != env.current_thread => EBUSY,
        _ => {
            *lock_count = lock_count.checked_add(1).unwrap();
            *owning_thread = Some(env.current_thread);
            0
        }
    }
}

fn funlockfile(env: &mut Environment, file_ptr: MutPtr<FILE>) {
    let FILEHostObject {
        ref mut lock_count,
        ref mut owning_thread,
        ..
    } = env
        .libc_state
        .stdio
        .get_file_host_obj_mut(&mut env.mem, file_ptr);
    assert_eq!(*owning_thread, Some(env.current_thread));
    *lock_count = lock_count.checked_sub(1).unwrap();
    if *lock_count == 0 {
        *owning_thread = None;
    }
}

pub const CONSTANTS: ConstantExports = &[
    (
        "___stdinp",
        HostConstant::Custom(|env| -> ConstVoidPtr {
            let ptr = env.mem.alloc_and_write(FILE { fd: STDIN_FILENO });
            // Note: Host object would be created lazily
            env.mem.alloc_and_write(ptr).cast().cast_const()
        }),
    ),
    (
        "___stdoutp",
        HostConstant::Custom(|env| -> ConstVoidPtr {
            let ptr = env.mem.alloc_and_write(FILE { fd: STDOUT_FILENO });
            // Note: Host object would be created lazily
            env.mem.alloc_and_write(ptr).cast().cast_const()
        }),
    ),
    (
        "___stderrp",
        HostConstant::Custom(|env| -> ConstVoidPtr {
            let ptr = env.mem.alloc_and_write(FILE { fd: STDERR_FILENO });
            // Note: Host object would be created lazily
            env.mem.alloc_and_write(ptr).cast().cast_const()
        }),
    ),
];

pub const FUNCTIONS: FunctionExports = &[
    // Standard C functions
    export_c_func!(fopen(_, _)),
    export_c_func!(fread(_, _, _, _)),
    export_c_func!(fgetc(_)),
    export_c_func!(getc(_)),
    export_c_func!(ungetc(_, _)),
    export_c_func!(fgets(_, _, _)),
    export_c_func!(fputs(_, _)),
    export_c_func!(fputc(_, _)),
    export_c_func!(putc(_, _)),
    export_c_func!(fwrite(_, _, _, _)),
    export_c_func!(fseek(_, _, _)),
    export_c_func!(fseeko(_, _, _)),
    export_c_func!(ftell(_)),
    export_c_func!(ftello(_)),
    export_c_func!(rewind(_)),
    export_c_func!(fsetpos(_, _)),
    export_c_func!(fgetpos(_, _)),
    export_c_func!(feof(_)),
    export_c_func!(clearerr(_)),
    export_c_func!(fflush(_)),
    export_c_func!(fclose(_)),
    export_c_func!(ferror(_)),
    export_c_func!(puts(_)),
    export_c_func!(putchar(_)),
    export_c_func!(remove(_)),
    export_c_func!(setbuf(_, _)),
    // POSIX-specific functions
    export_c_func!(fileno(_)),
    export_c_func!(flockfile(_)),
    export_c_func!(ftrylockfile(_)),
    export_c_func!(funlockfile(_)),
];
