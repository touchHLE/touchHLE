/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `stdlib.h`

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, GuestUSize, MutVoidPtr};
use crate::Environment;

#[derive(Default)]
pub struct State {
    rand: u32,
    random: u32,
}

fn malloc(env: &mut Environment, size: GuestUSize) -> MutVoidPtr {
    // size == 0 is an implementation-defined case. macOS will give you an
    // allocation so presumably iPhone OS does too.
    env.mem.alloc(size.max(1))
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

fn prng(state: u32) -> u32 {
    // The state must not be zero for this algorithm to work. This also makes
    // the default seed be 1, which matches the C standard.
    let mut state: u32 = state.max(1);
    // https://en.wikipedia.org/wiki/Xorshift#Example_implementation
    // xorshift32 is not a good random number generator, but it is cute one!
    // It's not like anyone expects the C stdlib `rand()` to be good.
    state ^= state << 13;
    state ^= state >> 17;
    state ^= state << 5;
    state
}

const RAND_MAX: i32 = i32::MAX;

fn srand(env: &mut Environment, seed: u32) {
    env.libc_state.stdlib.rand = seed;
}
fn rand(env: &mut Environment) -> i32 {
    env.libc_state.stdlib.rand = prng(env.libc_state.stdlib.rand);
    (env.libc_state.stdlib.rand as i32) & RAND_MAX
}

// BSD's "better" random number generator, with an implementation that is not
// actually better.
fn srandom(env: &mut Environment, seed: u32) {
    env.libc_state.stdlib.random = seed;
}
fn random(env: &mut Environment) -> i32 {
    env.libc_state.stdlib.random = prng(env.libc_state.stdlib.random);
    (env.libc_state.stdlib.random as i32) & RAND_MAX
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(malloc(_)),
    export_c_func!(calloc(_, _)),
    export_c_func!(free(_)),
    export_c_func!(atexit(_)),
    export_c_func!(atoi(_)),
    export_c_func!(atof(_)),
    export_c_func!(srand(_)),
    export_c_func!(rand()),
    export_c_func!(srandom(_)),
    export_c_func!(random()),
];
