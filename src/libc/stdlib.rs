/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `stdlib.h`

use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr};
use crate::Environment;
use std::collections::HashMap;
use std::str::FromStr;

pub mod qsort;

#[derive(Default)]
pub struct State {
    rand: u32,
    random: u32,
    arc4random: u32,
    env: HashMap<Vec<u8>, MutPtr<u8>>,
}

// Sizes of zero are implementation-defined. macOS will happily give you back
// an allocation for any of these, so presumably iPhone OS does too.
// (touchHLE's allocator will round up allocations to at least 16 bytes.)

fn malloc(env: &mut Environment, size: GuestUSize) -> MutVoidPtr {
    env.mem.alloc(size)
}

fn calloc(env: &mut Environment, count: GuestUSize, size: GuestUSize) -> MutVoidPtr {
    let total = size.checked_mul(count).unwrap();
    env.mem.alloc(total)
}

fn realloc(env: &mut Environment, ptr: MutVoidPtr, size: GuestUSize) -> MutVoidPtr {
    if ptr.is_null() {
        return malloc(env, size);
    }
    env.mem.realloc(ptr, size)
}

fn free(env: &mut Environment, ptr: MutVoidPtr) {
    if ptr.is_null() {
        // "If ptr is a NULL pointer, no operation is performed."
        return;
    }
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

fn atol(env: &mut Environment, s: ConstPtr<u8>) -> i32 {
    atoi(env, s)
}

fn atof(env: &mut Environment, s: ConstPtr<u8>) -> f64 {
    atof_inner(env, s).map_or(0.0, |tuple| tuple.0)
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

fn arc4random(env: &mut Environment) -> u32 {
    env.libc_state.stdlib.arc4random = prng(env.libc_state.stdlib.arc4random);
    env.libc_state.stdlib.arc4random
}

fn getenv(env: &mut Environment, name: ConstPtr<u8>) -> MutPtr<u8> {
    let name_cstr = env.mem.cstr_at(name);
    // TODO: Provide all the system environment variables an app might expect to
    // find. Currently the only environment variables that can be found are
    // those put there by the app (Crash Bandicoot Nitro Kart 3D uses this).
    let Some(&value) = env.libc_state.stdlib.env.get(name_cstr) else {
        log!(
            "Warning: getenv() for {:?} ({:?}) unhandled",
            name,
            std::str::from_utf8(name_cstr)
        );
        return Ptr::null();
    };
    log_dbg!(
        "getenv({:?} ({:?})) => {:?} ({:?})",
        name,
        name_cstr,
        value,
        env.mem.cstr_at_utf8(value),
    );
    // Caller should not modify the result
    value
}
fn setenv(env: &mut Environment, name: ConstPtr<u8>, value: ConstPtr<u8>, overwrite: i32) -> i32 {
    let name_cstr = env.mem.cstr_at(name);
    if let Some(&existing) = env.libc_state.stdlib.env.get(name_cstr) {
        if overwrite == 0 {
            return 0; // success
        }
        env.mem.free(existing.cast());
    };
    let value = super::string::strdup(env, value);
    let name_cstr = env.mem.cstr_at(name); // reborrow
    env.libc_state.stdlib.env.insert(name_cstr.to_vec(), value);
    log_dbg!(
        "Stored new value {:?} ({:?}) for environment variable {:?}",
        value,
        env.mem.cstr_at_utf8(value),
        std::str::from_utf8(name_cstr),
    );
    0 // success
}

fn exit(_env: &mut Environment, exit_code: i32) {
    echo!("App called exit(), exiting.");
    std::process::exit(exit_code);
}

fn bsearch(
    env: &mut Environment,
    key: ConstVoidPtr,
    items: ConstVoidPtr,
    item_count: GuestUSize,
    item_size: GuestUSize,
    compare_callback: GuestFunction, // (*int)(const void*, const void*)
) -> ConstVoidPtr {
    log_dbg!(
        "binary search for {:?} in {} items of size {:#x} starting at {:?}",
        key,
        item_count,
        item_size,
        items
    );
    let mut low = 0;
    let mut len = item_count;
    while len > 0 {
        let half_len = len / 2;
        let item: ConstVoidPtr = (items.cast::<u8>() + item_size * (low + half_len)).cast();
        // key must be first argument
        let cmp_result: i32 = compare_callback.call_from_host(env, (key, item));
        (low, len) = match cmp_result.signum() {
            0 => {
                log_dbg!("=> {:?}", item);
                return item;
            }
            1 => (low + half_len + 1, len - half_len - 1),
            -1 => (low, half_len),
            _ => unreachable!(),
        }
    }
    log_dbg!("=> NULL (not found)");
    Ptr::null()
}

fn strtof(env: &mut Environment, nptr: ConstPtr<u8>, endptr: MutPtr<ConstPtr<u8>>) -> f32 {
    let (number, length) = atof_inner(env, nptr).unwrap_or((0.0, 0));
    if !endptr.is_null() {
        env.mem.write(endptr, nptr + length);
    }
    number as f32
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(malloc(_)),
    export_c_func!(calloc(_, _)),
    export_c_func!(realloc(_, _)),
    export_c_func!(free(_)),
    export_c_func!(atexit(_)),
    export_c_func!(atoi(_)),
    export_c_func!(atol(_)),
    export_c_func!(atof(_)),
    export_c_func!(srand(_)),
    export_c_func!(rand()),
    export_c_func!(srandom(_)),
    export_c_func!(random()),
    export_c_func!(arc4random()),
    export_c_func!(getenv(_)),
    export_c_func!(setenv(_, _, _)),
    export_c_func!(exit(_)),
    export_c_func!(bsearch(_, _, _, _, _)),
    export_c_func!(strtof(_, _)),
];

/// Returns a tuple containing the parsed number and the length of the number in
/// the string
fn atof_inner(env: &mut Environment, s: ConstPtr<u8>) -> Result<(f64, u32), <f64 as FromStr>::Err> {
    // atof() is similar to atoi().
    // FIXME: no C99 hexfloat, INF, NAN support
    let start = skip_whitespace(env, s);
    let whitespace_len = Ptr::to_bits(start) - Ptr::to_bits(s);
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
    s.parse().map(|result| (result, whitespace_len + len))
}
