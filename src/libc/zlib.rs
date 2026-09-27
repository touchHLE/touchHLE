/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! zlib's "gz" file API (`zlib.h`)

use std::collections::HashMap;

use flate2::{Compression, read::MultiGzDecoder, write::GzEncoder};

use super::posix_io::{
    self, FileDescriptor, O_CREAT, O_RDONLY, O_TRUNC, O_WRONLY,
};
use crate::dyld::{export_c_func, FunctionExports};
use std::io::{Read, Write};

use crate::mem::{
    ConstPtr, ConstVoidPtr, GuestISize, GuestUSize, Mem, MutPtr, MutVoidPtr, Ptr, SafeRead,
};
use crate::Environment;

const Z_OK: i32 = 0;
const Z_ERRNO: i32 = -1;

#[repr(C)]
struct gzFileS {
    fd: FileDescriptor,
}
unsafe impl SafeRead for gzFileS {}

pub type gzFile = MutPtr<gzFileS>;

enum GzFileHostObject {
    /// A fully-buffered, decompressed gzip file for reading.
    Read {
        fd: FileDescriptor,
        data: Vec<u8>,
        pos: usize,
    },
    /// An in-progress gzip encoder for writing.
    Write {
        fd: FileDescriptor,
        encoder: Option<GzEncoder<Vec<u8>>>,
        pos: usize,
    },
}

impl GzFileHostObject {
    fn fd(&self) -> FileDescriptor {
        match self {
            GzFileHostObject::Read { fd, .. } => *fd,
            GzFileHostObject::Write { fd, .. } => *fd,
        }
    }
}

#[derive(Default)]
pub struct State {
    objects: HashMap<gzFile, GzFileHostObject>,
    /// Lazily-allocated, empty error string returned by `gzerror()` when
    /// there is no error.
    error_str: Option<MutPtr<u8>>,
}

fn is_write_mode(mode: &[u8]) -> bool {
    mode.contains(&b'w')
}

fn gzopen(env: &mut Environment, path: ConstPtr<u8>, mode: ConstPtr<u8>) -> gzFile {
    let path_str = env.mem.cstr_at(path).to_vec();
    let mode_str = env.mem.cstr_at(mode).to_vec();

    let write_mode = is_write_mode(&mode_str);
    let flags = if write_mode {
        O_WRONLY | O_CREAT | O_TRUNC
    } else {
        O_RDONLY
    };

    let fd = posix_io::open_direct(env, path, flags);
    if fd == -1 {
        log!("Warning: gzopen({:?}, {:?}) failed to open file", path_str, mode_str);
        return Ptr::null();
    }

    let object = if write_mode {
        GzFileHostObject::Write {
            fd,
            encoder: Some(GzEncoder::new(Vec::new(), Compression::default())),
            pos: 0,
        }
    } else {
        let data = env.libc_state.posix_io.read_fd_all(fd);
        posix_io::close(env, fd);
        GzFileHostObject::Read { fd: -1, data, pos: 0 }
    };

    let file: gzFile = env.mem.alloc_and_write(gzFileS { fd: object.fd() });
    env.libc_state.zlib.objects.insert(file, object);
    file
}

fn gzclose(env: &mut Environment, file: gzFile) -> i32 {
    let Some(object) = env.libc_state.zlib.objects.remove(&file) else {
        return Z_ERRNO;
    };
    if let GzFileHostObject::Write { fd, encoder, .. } = object {
        if let Err(e) = finish_write(env, fd, encoder) {
            log!("Warning: gzclose() failed to write compressed output: {e}");
        }
        posix_io::close(env, fd);
    }
    env.mem.free(file.cast());
    Z_OK
}

/// Writes the fully-compressed output of a write-mode file to its file
/// descriptor, leaving the encoder ready for reuse.
fn finish_write(
    env: &mut Environment,
    fd: FileDescriptor,
    encoder: Option<GzEncoder<Vec<u8>>>,
) -> Result<(), std::io::Error> {
    let Some(encoder) = encoder else { return Ok(()) };
    let output = encoder.finish()?;
    env.libc_state.posix_io.write_fd_all(fd, &output);
    Ok(())
}

/// Returns the decompressed contents of a read-mode file. Panics if the file
/// is not a read-mode file.
fn read_data(env: &mut Environment, file: gzFile) -> Vec<u8> {
    match env.libc_state.zlib.objects.get(&file) {
        Some(GzFileHostObject::Read { data, .. }) => data.clone(),
        _ => panic!("gzip file {:?} is not open for reading", file),
    }
}

fn gzread(env: &mut Environment, file: gzFile, buf: MutVoidPtr, len: GuestUSize) -> GuestISize {
    let data = read_data(env, file);
    let pos = match env.libc_state.zlib.objects.get(&file) {
        Some(GzFileHostObject::Read { pos, .. }) => *pos,
        _ => return -1,
    };
    let remaining = &data[pos.min(data.len())..];
    let n = (len as usize).min(remaining.len());
    if n > 0 {
        let chunk = remaining[..n].to_vec();
        env.mem
            .bytes_at_mut(buf.cast(), n as GuestUSize)
            .copy_from_slice(&chunk);
    }
    if let Some(GzFileHostObject::Read { pos, .. }) = env.libc_state.zlib.objects.get_mut(&file) {
        *pos += n;
    }
    n as GuestISize
}

fn gzwrite(env: &mut Environment, file: gzFile, buf: ConstVoidPtr, len: GuestUSize) -> GuestISize {
    let Some(GzFileHostObject::Write { encoder, pos, .. }) = env
        .libc_state
        .zlib
        .objects
        .get_mut(&file)
    else {
        return -1;
    };
    let Some(enc) = encoder.as_mut() else { return -1 };
    let data = env.mem.bytes_at(buf.cast(), len).to_vec();
    match enc.write_all(&data) {
        Ok(()) => {
            *pos += len as usize;
            len as GuestISize
        }
        Err(e) => {
            log!("Warning: gzwrite() failed: {e}");
            -1
        }
    }
}

fn gzflush(env: &mut Environment, file: gzFile, flush: i32) -> i32 {
    // TODO: proper support for flush levels. For now, only flush the pending
    // compressed output to the file descriptor.
    assert!(flush >= 0 && flush <= 5, "gzflush() with invalid flush level {flush}");
    let Some(GzFileHostObject::Write { fd, encoder, .. }) =
        env.libc_state.zlib.objects.get_mut(&file)
    else {
        return Z_ERRNO;
    };
    let Some(enc) = encoder.take() else { return Z_OK };
    let fd = *fd;
    encoder.replace(GzEncoder::new(Vec::new(), Compression::default()));
    if let Err(e) = finish_write(env, fd, Some(enc)) {
        log!("Warning: gzflush() failed: {e}");
        return Z_ERRNO;
    }
    Z_OK
}

fn gzseek(env: &mut Environment, file: gzFile, offset: i64, whence: i32) -> i64 {
    match env.libc_state.zlib.objects.get_mut(&file) {
        Some(GzFileHostObject::Read { data, pos, .. }) => {
            let data_len = data.len() as i64;
            let new_pos = match whence {
                0 => offset,
                1 => *pos as i64 + offset,
                2 => data_len + offset,
                _ => return -1,
            };
            if !(0..=data_len).contains(&new_pos) {
                return -1;
            }
            *pos = new_pos as usize;
            new_pos
        }
        Some(GzFileHostObject::Write { pos, .. }) => {
            if offset == 0 && (whence == 1 || (*pos as i64) == offset && whence == 0) {
                *pos as i64
            } else {
                log!("Warning: gzseek() on a write-mode gzip file is not supported");
                -1
            }
        }
        None => -1,
    }
}

fn gztell(env: &mut Environment, file: gzFile) -> i64 {
    match env.libc_state.zlib.objects.get(&file) {
        Some(GzFileHostObject::Read { pos, .. }) => *pos as i64,
        Some(GzFileHostObject::Write { pos, .. }) => *pos as i64,
        None => -1,
    }
}

fn gzrewind(env: &mut Environment, file: gzFile) -> i32 {
    match env.libc_state.zlib.objects.get_mut(&file) {
        Some(GzFileHostObject::Read { pos, .. }) => {
            *pos = 0;
            0
        }
        _ => -1,
    }
}

fn gzeof(env: &mut Environment, file: gzFile) -> i32 {
    match env.libc_state.zlib.objects.get(&file) {
        Some(GzFileHostObject::Read { data, pos, .. }) => (*pos >= data.len()) as i32,
        Some(GzFileHostObject::Write { .. }) => 0,
        None => 1,
    }
}

fn gzerror(env: &mut Environment, file: gzFile, errnum: MutPtr<i32>) -> ConstPtr<u8> {
    let _active = env.libc_state.zlib.objects.contains_key(&file);
    if !errnum.is_null() {
        env.mem.write(errnum, Z_OK);
    }
    let err_str = match env.libc_state.zlib.error_str {
        Some(ptr) => ptr,
        None => {
            let ptr = env.mem.alloc_and_write(0u8);
            env.libc_state.zlib.error_str = Some(ptr);
            ptr
        }
    };
    err_str.cast_const()
}

fn gzgetc(env: &mut Environment, file: gzFile) -> i32 {
    let data = read_data(env, file);
    let pos = match env.libc_state.zlib.objects.get(&file) {
        Some(GzFileHostObject::Read { pos, .. }) => *pos,
        _ => return -1,
    };
    if pos >= data.len() {
        return -1;
    }
    if let Some(GzFileHostObject::Read { pos, .. }) = env.libc_state.zlib.objects.get_mut(&file) {
        *pos += 1;
    }
    data[pos] as i32
}

fn gzungetc(env: &mut Environment, c: i32, file: gzFile) -> i32 {
    match env.libc_state.zlib.objects.get_mut(&file) {
        Some(GzFileHostObject::Read { pos, .. }) => {
            if *pos == 0 {
                return -1;
            }
            *pos -= 1;
            c
        }
        _ => -1,
    }
}

fn gzputc(env: &mut Environment, file: gzFile, c: i32) -> i32 {
    let Some(GzFileHostObject::Write { encoder, pos, .. }) =
        env.libc_state.zlib.objects.get_mut(&file)
    else {
        return -1;
    };
    let Some(enc) = encoder.as_mut() else { return -1 };
    match enc.write_all(&[c as u8]) {
        Ok(()) => {
            *pos += 1;
            c & 0xff
        }
        Err(_) => -1,
    }
}

fn gzputs(env: &mut Environment, file: gzFile, s: ConstPtr<u8>) -> i32 {
    let bytes = env.mem.cstr_at(s);
    let len = bytes.len() as GuestUSize;
    gzwrite(env, file, s.cast(), len)
}

fn gzgets(env: &mut Environment, file: gzFile, buf: MutPtr<u8>, len: GuestUSize) -> MutPtr<u8> {
    if len == 0 {
        return Ptr::null();
    }
    let data = read_data(env, file);
    let pos = match env.libc_state.zlib.objects.get(&file) {
        Some(GzFileHostObject::Read { pos, .. }) => *pos,
        _ => return Ptr::null(),
    };
    if pos >= data.len() {
        return Ptr::null();
    }
    let line: Vec<u8> = {
        let (data, pos) = match env.libc_state.zlib.objects.get_mut(&file) {
            Some(GzFileHostObject::Read { data, pos, .. }) => (data, pos),
            _ => return Ptr::null(),
        };
        let remaining = &data[*pos..];
        let max = (len as usize - 1).min(remaining.len());
        let line_end = remaining[..max]
            .iter()
            .position(|&b| b == b'\n')
            .map(|i| i + 1)
            .unwrap_or(max);
        *pos += line_end;
        remaining[..line_end].to_vec()
    };
    let line_len = line.len() as GuestUSize;
    env.mem
        .bytes_at_mut(buf, line_len + 1)[..line.len()]
        .copy_from_slice(&line);
    env.mem.write(buf + line_len, 0u8);
    buf
}

pub(crate) const ZLIB_FUNCTIONS: FunctionExports = &[
    export_c_func!(gzopen(_, _)),
    export_c_func!(gzclose(_)),
    export_c_func!(gzread(_, _, _)),
    export_c_func!(gzwrite(_, _, _)),
    export_c_func!(gzflush(_, _)),
    export_c_func!(gzseek(_, _, _)),
    export_c_func!(gztell(_)),
    export_c_func!(gzrewind(_)),
    export_c_func!(gzeof(_)),
    export_c_func!(gzerror(_, _)),
    export_c_func!(gzgetc(_)),
    export_c_func!(gzungetc(_, _)),
    export_c_func!(gzdirect(_)),
    export_c_func!(gzputc(_, _)),
    export_c_func!(gzputs(_, _)),
    export_c_func!(gzgets(_, _, _)),
];

fn gzdirect(env: &mut Environment, file: gzFile) -> i32 {
    let _ = env;
    let _ = file;
    // Report that the stream is compressed (not direct).
    0
}
