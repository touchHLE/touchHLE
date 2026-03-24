/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! SQLite3 Stubs

use crate::dyld::export_c_func;
use crate::mem::{ConstPtr, MutPtr, MutVoidPtr, Ptr};
use crate::Environment;

const SQLITE_OK: i32 = 0;

fn sqlite3_open(
    env: &mut Environment,
    _filename: ConstPtr<u8>,
    pp_db: MutPtr<MutVoidPtr>,
) -> i32 {
    log!("sqlite3_open called (STUB)");

    // Write a dummy pointer to prevent the app from seeing NULL.
    env.mem.write(pp_db, Ptr::from_raw(0xBAADF00D as _));

    SQLITE_OK
}

fn sqlite3_close(_env: &mut Environment, db: MutVoidPtr) -> i32 {
    log!("sqlite3_close({:?}) called (STUB)", db);
    SQLITE_OK
}

pub const FUNCTIONS: crate::dyld::FunctionExports = &[
    export_c_func!(sqlite3_open(_, _, _)),
    export_c_func!(sqlite3_close(_)),
];
