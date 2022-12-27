//! `stdlib.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{GuestUSize, MutVoidPtr};
use crate::Environment;

fn malloc(env: &mut Environment, size: GuestUSize) -> MutVoidPtr {
    assert!(size != 0);
    let res = env.mem.alloc(size);
    println!("libc: malloc({:#x}) => {:?}", size, res);
    res
}

fn calloc(env: &mut Environment, count: GuestUSize, size: GuestUSize) -> MutVoidPtr {
    assert!(size != 0 && count != 0);
    let total = size.checked_mul(count).unwrap();
    let res = env.mem.alloc(total);
    println!("libc: calloc({:#x}, {:#x}) => {:?}", count, size, res);
    res
}

fn free(env: &mut Environment, ptr: MutVoidPtr) {
    println!("libc: free({:?})", ptr);
    env.mem.free(ptr);
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(malloc(_)),
    export_c_func!(calloc(_, _)),
    export_c_func!(free(_)),
];
