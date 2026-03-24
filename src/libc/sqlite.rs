use crate::Environment;
use crate::mem::{ConstPtr, MutPtr, MutVoidPtr, Ptr};
use crate::dyld::FunctionExports;

// SQLITE_OK = 0
fn sqlite3_open(env: &mut Environment, _filename: ConstPtr<u8>, pp_db: MutPtr<MutVoidPtr>) -> i32 {
    log!("STUB: sqlite3_open called");
    // Записываем "фейковый" указатель на структуру БД, чтобы игра не видела NULL
    env.mem.write(pp_db, Ptr::from_raw(0xBAADF00D as _)); 
    0 
}

fn sqlite3_close(_env: &mut Environment, _db: MutVoidPtr) -> i32 {
    log!("STUB: sqlite3_close called");
    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sqlite3_open(_, _)),
    export_c_func!(sqlite3_close(_)),
];

