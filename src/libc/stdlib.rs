//! `stdlib.h`

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, GuestUSize, MutVoidPtr};
use crate::Environment;

fn malloc(env: &mut Environment, size: GuestUSize) -> MutVoidPtr {
    assert!(size != 0);
    env.mem.alloc(size)
}

fn calloc(env: &mut Environment, count: GuestUSize, size: GuestUSize) -> MutVoidPtr {
    assert!(size != 0 && count != 0);
    let total = size.checked_mul(count).unwrap();
    env.mem.alloc(total)
}

fn free(env: &mut Environment, ptr: MutVoidPtr) {
    env.mem.free(ptr);
}

fn atexit(
    _env: &mut Environment,
    func: GuestFunction, // void (*func)(void)
) -> i32 {
    // TODO: when this is implemented, make sure it's properly compatible with
    // __cxa_atexit.
    log!("TODO: atexit({:?}) (unimplemented)", func);
    0 // success
}

fn skip_whitespace(env: &mut Environment, s: ConstPtr<u8>) -> ConstPtr<u8> {
    let mut start = s;
    loop {
        let c = env.mem.read(start);
        // Rust's definition of whitespace excludes vertical tab, unlike C's
        if c.is_ascii_whitespace() || c == b'\x0b' {
            start += 1;
        } else {
            break;
        }
    }
    start
}

fn atoi(env: &mut Environment, s: ConstPtr<u8>) -> i32 {
    // atoi() doesn't work with a null-terminated string, instead it stops
    // once it hits something that's not a digit, so we have to do some parsing
    // ourselves.
    let start = skip_whitespace(env, s);
    let mut len = 0;
    let maybe_sign = env.mem.read(start + len);
    if maybe_sign == b'+' || maybe_sign == b'-' || maybe_sign.is_ascii_digit() {
        len += 1;
    }
    while env.mem.read(start + len).is_ascii_digit() {
        len += 1;
    }

    let s = std::str::from_utf8(env.mem.bytes_at(start, len)).unwrap();
    // conveniently, overflow is undefined, so 0 is as valid a result as any
    s.parse().unwrap_or(0)
}

fn atof(env: &mut Environment, s: ConstPtr<u8>) -> f64 {
    // atof() is similar to atoi().
    // FIXME: no C99 hexfloat, INF, NAN support
    let start = skip_whitespace(env, s);
    let mut len = 0;
    let maybe_sign = env.mem.read(start + len);
    if maybe_sign == b'+' || maybe_sign == b'-' || maybe_sign.is_ascii_digit() {
        len += 1;
    }
    while env.mem.read(start + len).is_ascii_digit() {
        len += 1;
    }
    if env.mem.read(start + len) == b'.' {
        len += 1;
        while env.mem.read(start + len).is_ascii_digit() {
            len += 1;
        }
    }
    if env.mem.read(start + len).to_ascii_lowercase() == b'e' {
        len += 1;
        let maybe_sign = env.mem.read(start + len);
        if maybe_sign == b'+' || maybe_sign == b'-' || maybe_sign.is_ascii_digit() {
            len += 1;
        }
        while env.mem.read(start + len).is_ascii_digit() {
            len += 1;
        }
    }

    let s = std::str::from_utf8(env.mem.bytes_at(start, len)).unwrap();
    s.parse().unwrap_or(0.0)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(malloc(_)),
    export_c_func!(calloc(_, _)),
    export_c_func!(free(_)),
    export_c_func!(atexit(_)),
    export_c_func!(atoi(_)),
    export_c_func!(atof(_)),
];
