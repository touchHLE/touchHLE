/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! POSIX `sys/stat.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::fs::GuestPath;
use crate::mem::ConstPtr;
use crate::Environment;

#[allow(non_camel_case_types)]
type mode_t = u16;

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

pub const FUNCTIONS: FunctionExports = &[export_c_func!(mkdir(_, _))];
