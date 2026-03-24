/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 */

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, MutPtr, SafeRead};
use crate::Environment;

pub type Sqlite3Ptr = MutPtr<u32>;
pub type Sqlite3StmtPtr = MutPtr<u32>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Sqlite3;
unsafe impl SafeRead for Sqlite3 {}

pub fn sqlite3_open(
    env: &mut Environment,
    filename: ConstPtr<u8>,
    _pp_db: MutPtr<Sqlite3Ptr>,
) -> i32 {
    let name = env.mem.cstr_at_utf8(filename).unwrap_or("unknown");
    log!("sqlite3_open({:?})", name);
    0
}

pub fn sqlite3_close(_env: &mut Environment, _db: Sqlite3Ptr) -> i32 {
    log_dbg!("sqlite3_close");
    0
}

pub fn sqlite3_exec(
    env: &mut Environment,
    _db: Sqlite3Ptr,
    sql: ConstPtr<u8>,
    _callback: u32,
    _arg: u32,
    _errmsg: MutPtr<MutPtr<u8>>,
) -> i32 {
    let query = env.mem.cstr_at_utf8(sql).unwrap_or("invalid sql");
    log!("sqlite3_exec: {}", query);
    0
}

pub fn sqlite3_finalize(_env: &mut Environment, _stmt: Sqlite3StmtPtr) -> i32 {
    log_dbg!("sqlite3_finalize");
    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sqlite3_open(_, _)),
    export_c_func!(sqlite3_close(_)),
    export_c_func!(sqlite3_exec(_, _, _, _, _)),
    export_c_func!(sqlite3_finalize(_)),
];
