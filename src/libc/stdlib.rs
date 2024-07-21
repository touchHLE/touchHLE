/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `stdlib.h`

use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::{export_c_func, export_c_func_aliased, FunctionExports};
use crate::fs::{resolve_path, GuestPath};
use crate::libc::clocale::{setlocale, LC_CTYPE};
use crate::libc::string::strlen;
use crate::libc::wchar::wchar_t;
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
    // conveniently, overflow is undefined, so 0 is as valid a result as any
    let (res, _) = strtol_inner(env, s, 10).unwrap_or((0, 0));
    res
}

fn atol(env: &mut Environment, s: ConstPtr<u8>) -> i32 {
    atoi(env, s)
}

fn atof(env: &mut Environment, s: ConstPtr<u8>) -> f64 {
    strtod(env, s, Ptr::null())
}

fn strtod(env: &mut Environment, nptr: ConstPtr<u8>, endptr: MutPtr<MutPtr<u8>>) -> f64 {
    log_dbg!("strtod nptr {}", env.mem.cstr_at_utf8(nptr).unwrap());
    let (res, len) = atof_inner(env, nptr).unwrap_or((0.0, 0));
    if !endptr.is_null() {
        env.mem.write(endptr, (nptr + len).cast_mut());
    }
    res
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
const LONG_MIN: i32 = i32::MIN;
const LONG_MAX: i32 = i32::MAX;
const ULONG_MAX: u32 = u32::MAX;

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

// TODO: fix same issues as for strtol()
pub fn strtoul(
    env: &mut Environment,
    str: ConstPtr<u8>,
    endptr: MutPtr<MutPtr<u8>>,
    base: i32,
) -> u32 {
    let s = env.mem.cstr_at_utf8(str).unwrap();
    log_dbg!("strtoul({:?} ({}), {:?}, {})", str, s, endptr, base);
    assert_eq!(base, 16);
    let without_prefix = s.trim_start_matches("0x");
    let res = u32::from_str_radix(without_prefix, 16).unwrap_or(ULONG_MAX);
    if !endptr.is_null() {
        let len: GuestUSize = s.len().try_into().unwrap();
        env.mem.write(endptr, (str + len).cast_mut());
    }
    res
}

fn strtol(env: &mut Environment, str: ConstPtr<u8>, endptr: MutPtr<MutPtr<u8>>, base: i32) -> i32 {
    match strtol_inner(env, str, base as u32) {
        Ok((res, len)) => {
            if !endptr.is_null() {
                env.mem.write(endptr, (str + len).cast_mut());
            }
            res
        }
        Err(_) => {
            if !endptr.is_null() {
                env.mem.write(endptr, str.cast_mut());
            }
            0
        }
    }
}

fn realpath(
    env: &mut Environment,
    file_name: ConstPtr<u8>,
    resolve_name: MutPtr<u8>,
) -> MutPtr<u8> {
    assert!(!resolve_name.is_null());

    let file_name_str = env.mem.cstr_at_utf8(file_name).unwrap();
    // TOD0: resolve symbolic links
    let resolved = resolve_path(
        GuestPath::new(file_name_str),
        Some(env.fs.working_directory()),
    );
    let result = format!("/{}", resolved.join("/"));
    env.mem
        .bytes_at_mut(resolve_name, result.len() as GuestUSize)
        .copy_from_slice(result.as_bytes());
    env.mem
        .write(resolve_name + result.len() as GuestUSize, b'\0');

    log_dbg!(
        "realpath file_name '{}', resolve_name '{}'",
        env.mem.cstr_at_utf8(file_name).unwrap(),
        env.mem.cstr_at_utf8(resolve_name).unwrap()
    );

    resolve_name
}

fn mbstowcs(
    env: &mut Environment,
    pwcs: MutPtr<wchar_t>,
    s: ConstPtr<u8>,
    n: GuestUSize,
) -> GuestUSize {
    // TODO: support other locales
    let ctype_locale = setlocale(env, LC_CTYPE, Ptr::null());
    assert_eq!(env.mem.read(ctype_locale), b'C');

    let size = strlen(env, s);
    let to_write = size.min(n);
    for i in 0..to_write {
        let c = env.mem.read(s + i);
        env.mem.write(pwcs + i, c as wchar_t);
    }
    if to_write < n {
        env.mem.write(pwcs + to_write, wchar_t::default());
    }
    to_write
}

fn wcstombs(
    env: &mut Environment,
    s: ConstPtr<u8>,
    pwcs: MutPtr<wchar_t>,
    n: GuestUSize,
) -> GuestUSize {
    // TODO: support other locales
    let ctype_locale = setlocale(env, LC_CTYPE, Ptr::null());
    assert_eq!(env.mem.read(ctype_locale), b'C');

    if n == 0 {
        return 0;
    }
    let wcstr = env.mem.wcstr_at(pwcs);
    let len: GuestUSize = wcstr.bytes().len() as GuestUSize;
    let len = len.min(n);
    log_dbg!("wcstombs '{}', len {}, n {}", wcstr, len, n);
    env.mem
        .bytes_at_mut(s.cast_mut(), len)
        .copy_from_slice(wcstr.as_bytes());
    if len < n {
        env.mem.write((s + len).cast_mut(), b'\0');
    }
    len
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
    export_c_func!(strtod(_, _)),
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
    export_c_func!(strtoul(_, _, _)),
    export_c_func!(strtol(_, _, _)),
    export_c_func!(realpath(_, _)),
    export_c_func_aliased!("realpath$DARWIN_EXTSN", realpath(_, _)),
    export_c_func!(mbstowcs(_, _, _)),
    export_c_func!(wcstombs(_, _, _)),
];

/// Returns a tuple containing the parsed number and the length of the number in
/// the string
pub fn atof_inner(
    env: &mut Environment,
    s: ConstPtr<u8>,
) -> Result<(f64, u32), <f64 as FromStr>::Err> {
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

/// Returns a tuple containing the parsed number in the given base and
/// the length of the number in the string.
/// Base is mutable because in case if base 0 we need to auto-detect it.
pub fn strtol_inner(
    env: &mut Environment,
    str: ConstPtr<u8>,
    mut base: u32,
) -> Result<(i32, u32), ()> {
    // strtol() doesn't work with a null-terminated string, instead it stops
    // once it hits something that's not a digit, so we have to do some parsing
    // ourselves.
    let start = skip_whitespace(env, str);
    let whitespace_len = Ptr::to_bits(start) - Ptr::to_bits(str);
    let mut len = 0;
    let maybe_sign = env.mem.read(start + len);
    let mut sign = None;
    let mut prefix_length = 0;
    if maybe_sign == b'+' || maybe_sign == b'-' {
        sign = Some(maybe_sign);
        prefix_length += 1;
        len += 1;
    }
    // We need to do base detection before we can start counting
    // the number length, but after we maybe skipped the sign
    if base == 0 {
        base = if env.mem.read(start + len) == b'0' {
            let next = env.mem.read(start + len + 1);
            if next == b'x' || next == b'X' {
                16
            } else {
                8
            }
        } else {
            10
        }
    }
    // Skipping prefix if needed
    if (base == 8 || base == 16) && env.mem.read(start + len) == b'0' {
        len += 1;
        prefix_length += 1;
        if base == 16 {
            let next = env.mem.read(start + len);
            if next == b'x' || next == b'X' {
                len += 1;
                prefix_length += 1;
            }
        }
    }
    while (env.mem.read(start + len) as char).is_digit(base) {
        len += 1;
    }

    let s =
        std::str::from_utf8(env.mem.bytes_at(start + prefix_length, len - prefix_length)).unwrap();
    log_dbg!("strtol_inner({:?} ({}), {})", str, s, base);
    assert!((2..=36).contains(&base));
    let magnitude_len = len - prefix_length;
    let res = if magnitude_len > 0 {
        // TODO: set errno on range errors
        let mut res = i32::from_str_radix(s, base).unwrap_or(LONG_MAX);
        if sign == Some(b'-') {
            res = res.checked_mul(-1).unwrap_or(LONG_MIN);
        }
        res
    } else {
        // Special case - prefix of invalid octal number is a valid number 0
        if base == 8 && prefix_length > 0 {
            return Ok((0, whitespace_len + prefix_length));
        }
        return Err(());
    };
    Ok((res, whitespace_len + len))
}
