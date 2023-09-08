/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! POSIX `sys/stat.h`

use super::{off_t, FileDescriptor};
use crate::dyld::{export_c_func, FunctionExports};
use crate::fs::GuestPath;
use crate::mem::{ConstPtr, MutVoidPtr};
use crate::Environment;
use std::io::{Seek, SeekFrom};

#[allow(non_camel_case_types)]
pub type mode_t = u16;

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

fn fstat(env: &mut Environment, fd: FileDescriptor, buf: MutVoidPtr) -> i32 {
    // TODO: error handling for unknown fd?
    let file = env.libc_state.posix_io.file_for_fd(fd).unwrap();

    log!("Warning: fstat() call, this function is mostly unimplemented");
    // FIXME: This implementation is highly incomplete. fstat() returns a huge
    // struct with many kinds of data in it. This code is assuming the caller
    // only wants the file size.
    let st_size_ptr = (buf + 0x3c).cast::<off_t>();

    // TODO: Use the stream_len() method if that ever gets stabilized.
    let old_pos = file.file.stream_position().unwrap();
    let full_size = file.file.seek(SeekFrom::End(0)).unwrap();
    file.file.seek(SeekFrom::Start(old_pos)).unwrap();

    env.mem.write(st_size_ptr, full_size.try_into().unwrap());

    0 // success
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(mkdir(_, _)), export_c_func!(fstat(_, _))];
