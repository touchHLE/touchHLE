/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `stdio.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::fs::{GuestOpenOptions, GuestPath};
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr, SafeRead};
use crate::Environment;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};

pub mod printf;

#[derive(Default)]
pub struct State {
    files: HashMap<MutPtr<FILE>, FileHostObject>,
}

const EOF: i32 = -1;

#[allow(clippy::upper_case_acronyms)]
struct FILE {
    _filler: u8,
}
unsafe impl SafeRead for FILE {}

// TODO: Rewrite this to be layered on top of the POSIX I/O implementation, so
// that we can implement things like fdopen() in future.

struct FileHostObject {
    file: crate::fs::GuestFile,
}

fn fopen(env: &mut Environment, filename: ConstPtr<u8>, mode: ConstPtr<u8>) -> MutPtr<FILE> {
    let mut options = GuestOpenOptions::new();
    // all valid modes are UTF-8
    match env.mem.cstr_at_utf8(mode).unwrap() {
        "r" | "rb" => options.read(),
        "r+" | "rb+" | "r+b" => options.read().append(),
        "w" | "wb" => options.write().create().truncate(),
        "w+" | "wb+" | "w+b" => options.write().create().truncate().read(),
        "a" | "ab" => options.append().create(),
        "a+" | "ab+" | "a+b" => options.append().create().read(),
        // Modern C adds 'x' but that's not in the documentation so presumably
        // iPhone OS did not have it.
        other => panic!("Unexpected fopen() mode {:?}", other),
    };

    let res = match env.fs.open_with_options(
        GuestPath::new(&env.mem.cstr_at_utf8(filename).unwrap()),
        options,
    ) {
        Ok(file) => {
            let host_object = FileHostObject { file };
            let file_ptr = env.mem.alloc_and_write(FILE { _filler: 0 });
            env.libc_state.stdio.files.insert(file_ptr, host_object);
            file_ptr
        }
        Err(()) => {
            // TODO: set errno
            Ptr::null()
        }
    };
    log_dbg!("fopen({:?}, {:?}) => {:?}", filename, mode, res);
    res
}

fn fread(
    env: &mut Environment,
    buffer: MutVoidPtr,
    item_size: GuestUSize,
    n_items: GuestUSize,
    file_ptr: MutPtr<FILE>,
) -> GuestUSize {
    let file = env.libc_state.stdio.files.get_mut(&file_ptr).unwrap();
    let total_size = item_size.checked_mul(n_items).unwrap();
    let buffer_slice = env.mem.bytes_at_mut(buffer.cast(), total_size);
    // This does actually have exactly the behaviour that the C standard allows
    // and most implementations provide. There's no requirement that partial
    // objects should not be written to the buffer, and perhaps some app will
    // rely on that. The file position also does not need to be rewound!
    let bytes_read = file.file.read(buffer_slice).unwrap_or(0);
    let items_read: GuestUSize = (bytes_read / usize::try_from(item_size).unwrap())
        .try_into()
        .unwrap();
    if bytes_read < buffer_slice.len() {
        // TODO: set errno
        log!(
            "Warning: fread({:?}, {:#x}, {:#x}, {:?}) read only {:#x} of requested {:#x} bytes",
            buffer,
            item_size,
            n_items,
            file_ptr,
            total_size,
            bytes_read
        );
    } else {
        log_dbg!(
            "fread({:?}, {:#x}, {:#x}, {:?}) => {:#x}",
            buffer,
            item_size,
            n_items,
            file_ptr,
            items_read
        );
    };
    items_read
}

fn fwrite(
    env: &mut Environment,
    buffer: ConstVoidPtr,
    item_size: GuestUSize,
    n_items: GuestUSize,
    file_ptr: MutPtr<FILE>,
) -> GuestUSize {
    let file = env.libc_state.stdio.files.get_mut(&file_ptr).unwrap();
    let total_size = item_size.checked_mul(n_items).unwrap();
    let buffer_slice = env.mem.bytes_at(buffer.cast(), total_size);
    // Remarks in fread() apply here too.
    let bytes_written = file.file.write(buffer_slice).unwrap_or(0);
    let items_written: GuestUSize = (bytes_written / usize::try_from(item_size).unwrap())
        .try_into()
        .unwrap();
    if bytes_written < buffer_slice.len() {
        // TODO: set errno
        log!(
            "Warning: fwrite({:?}, {:#x}, {:#x}, {:?}) wrote only {:#x} of requested {:#x} bytes",
            buffer,
            item_size,
            n_items,
            file_ptr,
            total_size,
            bytes_written
        );
    } else {
        log_dbg!(
            "fwrite({:?}, {:#x}, {:#x}, {:?}) => {:#x}",
            buffer,
            item_size,
            n_items,
            file_ptr,
            items_written
        );
    };
    items_written
}

const SEEK_SET: i32 = 0;
const SEEK_CUR: i32 = 1;
const SEEK_END: i32 = 2;
fn fseek(env: &mut Environment, file_ptr: MutPtr<FILE>, offset: i32, whence: i32) -> i32 {
    let file = env.libc_state.stdio.files.get_mut(&file_ptr).unwrap();

    let from = match whence {
        // not sure whether offset is treated as signed or unsigned when using
        // SEEK_SET, so `.try_into()` seems safer.
        SEEK_SET => SeekFrom::Start(offset.try_into().unwrap()),
        SEEK_CUR => SeekFrom::Current(offset.into()),
        SEEK_END => SeekFrom::End(offset.into()),
        _ => panic!("Unsupported \"whence\" parameter to fseek(): {}", whence),
    };

    let res = match file.file.seek(from) {
        Ok(_) => 0,
        // TODO: set errno
        Err(_) => -1,
    };
    log_dbg!(
        "fseek({:?}, {:#x}, {}) => {}",
        file_ptr,
        offset,
        whence,
        res
    );
    res
}

fn ftell(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    let file = env.libc_state.stdio.files.get_mut(&file_ptr).unwrap();

    let res = match file.file.stream_position() {
        // TODO: What's the correct behaviour if the position is beyond 2GiB?
        Ok(pos) => pos.try_into().unwrap(),
        // TODO: set errno
        Err(_) => -1,
    };
    log_dbg!("ftell({:?}) => {:?}", file_ptr, res);
    res
}

fn fclose(env: &mut Environment, file_ptr: MutPtr<FILE>) -> i32 {
    let file = env.libc_state.stdio.files.remove(&file_ptr).unwrap();

    // The actual closing of the file happens implicitly when `file` falls out
    // of scope. The return value is about whether flushing succeeds.
    match file.file.sync_all() {
        Ok(()) => {
            log_dbg!("fclose({:?}) => 0", file_ptr);
            0
        }
        Err(_) => {
            // TODO: set errno
            log!("Warning: fclose({:?}) failed, returning EOF", file_ptr);
            EOF
        }
    }
}

fn puts(env: &mut Environment, s: ConstPtr<u8>) -> i32 {
    let _ = std::io::stdout().write_all(env.mem.cstr_at(s));
    let _ = std::io::stdout().write_all(b"\n");
    // TODO: I/O error handling
    // TODO: is this the return value iPhone OS uses?
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

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(fopen(_, _)),
    export_c_func!(fread(_, _, _, _)),
    export_c_func!(fwrite(_, _, _, _)),
    export_c_func!(fseek(_, _, _)),
    export_c_func!(ftell(_)),
    export_c_func!(fclose(_)),
    export_c_func!(puts(_)),
    export_c_func!(remove(_)),
];
