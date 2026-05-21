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
use crate::libc::errno::{set_errno, EINVAL};
use crate::libc::string::strlen;
use crate::libc::wchar::wchar_t;
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr, SafeRead};
use crate::{impl_GuestRet_for_large_struct, Environment};
use std::str::FromStr;
use std::time::SystemTime;

pub mod qsort;

#[derive(Default)]
pub struct State {
    rand: u32,
    random: u32,
    arc4random: u32,
}

// Sizes of zero are implementation-defined. macOS will happily give you back
// an allocation for any of these, so presumably iPhone OS does too.
// (touchHLE's allocator will round up allocations to at least 16 bytes.)

fn malloc(env: &mut Environment, size: GuestUSize) -> MutVoidPtr {
    // TODO: handle errno properly
    set_errno(env, 0);

    env.mem.alloc(size)
}

fn malloc_size(env: &mut Environment, ptr: ConstVoidPtr) -> GuestUSize {
    env.mem.malloc_size(ptr)
}

fn calloc(env: &mut Environment, count: GuestUSize, size: GuestUSize) -> MutVoidPtr {
    // TODO: handle errno properly
    set_errno(env, 0);

    let total = size.checked_mul(count).unwrap();
    env.mem.calloc(total)
}

fn valloc(env: &mut Environment, size: GuestUSize) -> MutVoidPtr {
    // TODO: handle errno properly
    set_errno(env, 0);

    env.mem.valloc(size)
}

fn realloc(env: &mut Environment, ptr: MutVoidPtr, size: GuestUSize) -> MutVoidPtr {
    // TODO: handle errno properly
    set_errno(env, 0);

    if ptr.is_null() {
        return malloc(env, size);
    }
    env.mem.realloc(ptr, size)
}

fn free(env: &mut Environment, ptr: MutVoidPtr) {
    // We need to catch situations of freeing NSObjects early!
    if env.objc.get_host_object(ptr.cast()).is_some() {
        log!(
            "App attempted to call free({:?}) on an object, calling dealloc_object() instead!",
            ptr
        );
        env.objc.dealloc_object(ptr.cast(), &mut env.mem);
        return;
    }

    // TODO: handle errno properly
    set_errno(env, 0);

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

#[allow(rustdoc::broken_intra_doc_links)] // https://github.com/rust-lang/rust/issues/83049
/// Counts whitespaces in `subject` starting from `offset`.
///
/// `getc_fn` is a callback to get next character from `subject`.
/// 3rd parameter in this callback is a index which is safe to ignore
/// (for example, in case of a file stream).
/// Error signifies an abnormal stop of input,
/// such as [crate::libc::stdio::EOF] in the file stream.
/// Note: `'\0'` does not necessary expect to produce an error!
///
/// `ungetc_fn` is a callback to un-get character from `subject`.
/// Could be ignored entirely (for example, in case of a string).
///
/// `subject` is either C string or file stream (for now).
///
/// `offset` defines an offset in `subject` from which conversion starts.
/// Could be ignored entirely (for example, in case of a file stream).
///
/// Returns count of whitespaces. Error returned from `getc_fn` is propagated
/// but count is retuned too.
fn count_whitespace_generic<
    T,
    U,
    F1: Fn(&mut Environment, MutPtr<U>, GuestUSize) -> Result<T, ()>,
    F2: Fn(&mut Environment, MutPtr<U>, u8), // TODO: make last param generic too?
>(
    env: &mut Environment,
    getc_fn: F1,
    ungetc_fn: F2,
    subject: MutPtr<U>,
    offset: GuestUSize,
) -> Result<GuestUSize, GuestUSize>
where
    u8: From<T>,
{
    let mut count: GuestUSize = offset;
    loop {
        let Ok(c) = getc_fn(env, subject, count) else {
            return Err(count - offset);
        };
        let c: u8 = c.into();
        // Rust's definition of whitespace excludes vertical tab, unlike C's
        if c.is_ascii_whitespace() || c == b'\x0b' {
            count += 1;
        } else {
            ungetc_fn(env, subject, c);
            break;
        }
    }
    Ok(count - offset)
}

fn atoi(env: &mut Environment, s: ConstPtr<u8>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

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
    // TODO: handle errno properly
    set_errno(env, 0);

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

fn srand(env: &mut Environment, seed: u32) {
    env.libc_state.stdlib.rand = seed;
}

// BSD's rand() seed function — we just use host system time,
// good enough for games that want fresh "fake" randomness each run.
fn sranddev(env: &mut Environment) {
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let seed = (time ^ (time >> 32)) as u32;
    env.libc_state.stdlib.rand = seed;
}

fn rand(env: &mut Environment) -> i32 {
    env.libc_state.stdlib.rand = prng(env.libc_state.stdlib.rand);
    (env.libc_state.stdlib.rand as i32) & RAND_MAX
}
fn rand_r(env: &mut Environment, seed_ptr: MutPtr<u32>) -> i32 {
    let mut seed = env.mem.read(seed_ptr);
    seed = prng(seed);
    env.mem.write(seed_ptr, seed);
    (seed as i32) & RAND_MAX
}

// BSD's "better" random number generator, with an implementation that is not
// actually better.
fn srandom(env: &mut Environment, seed: u32) {
    // TODO: handle errno properly
    set_errno(env, 0);

    env.libc_state.stdlib.random = seed;
}
fn random(env: &mut Environment) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    env.libc_state.stdlib.random = prng(env.libc_state.stdlib.random);
    (env.libc_state.stdlib.random as i32) & RAND_MAX
}

fn arc4random(env: &mut Environment) -> u32 {
    env.libc_state.stdlib.arc4random = prng(env.libc_state.stdlib.arc4random);
    env.libc_state.stdlib.arc4random
}

#[allow(non_camel_case_types)]
#[derive(Debug)]
#[repr(C, packed)]
struct div_t {
    quot: i32,
    rem: i32,
}
unsafe impl SafeRead for div_t {}
impl_GuestRet_for_large_struct!(div_t);

fn div(_env: &mut Environment, numer: i32, denom: i32) -> div_t {
    div_t {
        quot: numer.wrapping_div(denom),
        rem: numer.wrapping_rem(denom),
    }
}

fn getenv(env: &mut Environment, name: ConstPtr<u8>) -> MutPtr<u8> {
    let name_cstr = env.mem.cstr_at(name);
    let Some(&value) = env.env_vars.get(name_cstr) else {
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
    // TODO: handle errno properly
    set_errno(env, 0);

    let name_cstr = env.mem.cstr_at(name);
    if let Some(&existing) = env.env_vars.get(name_cstr) {
        if overwrite == 0 {
            return 0; // success
        }
        env.mem.free(existing.cast());
    };
    let value = super::string::strdup(env, value);
    let name_cstr = env.mem.cstr_at(name); // reborrow
    env.env_vars.insert(name_cstr.to_vec(), value);
    log_dbg!(
        "Stored new value {:?} ({:?}) for environment variable {:?}",
        value,
        env.mem.cstr_at_utf8(value),
        std::str::from_utf8(name_cstr),
    );
    0 // success
}
fn unsetenv(env: &mut Environment, name: ConstPtr<u8>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let name_cstr = env.mem.cstr_at(name);
    if !env.env_vars.contains_key(name_cstr) {
        set_errno(env, EINVAL);
        -1
    } else {
        todo!()
    }
}

fn exit(env: &mut Environment, exit_code: i32) {
    // TODO: handle errno properly
    set_errno(env, 0);

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
    // TODO: handle errno properly
    set_errno(env, 0);

    let (number, length) = atof_inner(env, nptr).unwrap_or((0.0, 0));
    if !endptr.is_null() {
        env.mem.write(endptr, nptr + length);
    }
    number as f32
}

pub fn strtoul(
    env: &mut Environment,
    str: ConstPtr<u8>,
    endptr: MutPtr<MutPtr<u8>>,
    base: i32,
) -> u32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let parse_res = str_to_int_inner_generic(
        env,
        |env, s, idx| Ok(env.mem.read(s + idx)),
        |_, _, _| (), // could be ignored
        str.cast_mut(),
        0, // starting offset
        base.try_into().unwrap(),
        u32::MAX, // max_length
        |s, base| u32::from_str_radix(s, base).unwrap_or(u32::MAX),
        |num| num.wrapping_neg(),
    );
    match parse_res {
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
fn wcstoul(
    env: &mut Environment,
    nptr: ConstPtr<wchar_t>,
    endptr: MutPtr<MutPtr<wchar_t>>,
    base: i32,
) -> u32 {
    // TODO: support other locales
    let ctype_locale = setlocale(env, LC_CTYPE, Ptr::null());
    assert_eq!(env.mem.read(ctype_locale), b'C');

    let w_string = env.mem.wcstr_at(nptr);
    assert!(w_string.is_ascii()); // TODO

    assert!(endptr.is_null()); // TODO

    let c_string = env.mem.alloc_and_write_cstr(w_string.as_bytes());
    // TODO: use str_to_int_inner_generic() instead
    let res = strtoul(env, c_string.cast_const(), Ptr::null(), base);
    env.mem.free(c_string.cast());
    log_dbg!(
        "wcstoul({:?} ({:?}), {:?}, {}) => {}",
        nptr,
        w_string,
        endptr,
        base,
        res
    );
    res
}

fn strtoull(
    env: &mut Environment,
    str: ConstPtr<u8>,
    endptr: MutPtr<MutPtr<u8>>,
    base: i32,
) -> u64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let parse_res = str_to_int_inner_generic(
        env,
        |env, s, idx| Ok(env.mem.read(s + idx)),
        |_, _, _| (), // could be ignored
        str.cast_mut(),
        0, // starting offset
        base.try_into().unwrap(),
        u32::MAX, // max_length
        |s, base| u64::from_str_radix(s, base).unwrap_or(u64::MAX),
        |num| num.wrapping_neg(),
    );
    match parse_res {
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

fn strtol(env: &mut Environment, str: ConstPtr<u8>, endptr: MutPtr<MutPtr<u8>>, base: i32) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

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
    // TODO: handle errno properly
    set_errno(env, 0);

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
    let len: GuestUSize = wcstr.len() as GuestUSize;
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

fn system(env: &mut Environment, cmd: ConstPtr<u8>) -> i32 {
    if cmd.is_null() {
        log!("TODO: App checked for sh availability with system(NULL), returning 0");
        return 0; // sh is not available!
    }
    log!("system({:?})", env.mem.cstr_at_utf8(cmd));
    todo!()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(malloc(_)),
    export_c_func!(malloc_size(_)),
    export_c_func!(calloc(_, _)),
    export_c_func!(valloc(_)),
    export_c_func!(realloc(_, _)),
    export_c_func!(free(_)),
    export_c_func!(atexit(_)),
    export_c_func!(atoi(_)),
    export_c_func!(atol(_)),
    export_c_func!(atof(_)),
    export_c_func!(strtod(_, _)),
    export_c_func!(srand(_)),
    export_c_func!(sranddev()),
    export_c_func!(rand()),
    export_c_func!(rand_r(_)),
    export_c_func!(srandom(_)),
    export_c_func!(random()),
    export_c_func!(arc4random()),
    export_c_func!(div(_, _)),
    export_c_func!(getenv(_)),
    export_c_func!(setenv(_, _, _)),
    export_c_func!(unsetenv(_)),
    export_c_func!(exit(_)),
    export_c_func!(bsearch(_, _, _, _, _)),
    export_c_func!(strtof(_, _)),
    export_c_func!(strtoul(_, _, _)),
    export_c_func!(wcstoul(_, _, _)),
    export_c_func!(strtoull(_, _, _)),
    export_c_func!(strtol(_, _, _)),
    export_c_func!(realpath(_, _)),
    export_c_func_aliased!("realpath$DARWIN_EXTSN", realpath(_, _)),
    export_c_func!(mbstowcs(_, _, _)),
    export_c_func!(wcstombs(_, _, _)),
    export_c_func!(system(_)),
];

/// A simple wrapper around [atof_inner_generic] for the case of C string.
pub fn atof_inner(
    env: &mut Environment,
    s: ConstPtr<u8>,
) -> Result<(f64, u32), <f64 as FromStr>::Err> {
    atof_inner_generic(
        env,
        |env, s, idx| Ok(env.mem.read(s + idx)),
        |_, _, _| (),
        s.cast_mut(),
        0,
    )
}

#[allow(rustdoc::broken_intra_doc_links)] // https://github.com/rust-lang/rust/issues/83049
/// Generic implementation of a conversion helper to `double`.
///
/// `getc_fn` is a callback to get next character from `subject`.
/// 3rd parameter in this callback is a index which is safe to ignore
/// (for example, in case of a file stream).
/// Error signifies an abnormal stop of input,
/// such as [crate::libc::stdio::EOF] in the file stream.
/// Note: `'\0'` does not necessary expect to produce an error!
///
/// `ungetc_fn` is a callback to un-get character from `subject`.
/// Could be ignored entirely (for example, in case of a string).
///
/// `subject` is either C string or file stream (for now).
///
/// `offset` defines an offset in `subject` from which conversion starts.
/// Could be ignored entirely (for example, in case of a file stream).
///
/// Returns a tuple containing the parsed number and the length of the number in
/// the string.
///
/// See also a TODO comment in [str_to_int_inner_generic].
pub fn atof_inner_generic<
    T,
    U,
    F1: Fn(&mut Environment, MutPtr<U>, GuestUSize) -> Result<T, ()>,
    F2: Fn(&mut Environment, MutPtr<U>, u8), // TODO: make last param generic too?
>(
    env: &mut Environment,
    getc_fn: F1,
    ungetc_fn: F2,
    subject: MutPtr<U>,
    offset: GuestUSize,
) -> Result<(f64, u32), <f64 as FromStr>::Err>
where
    u8: From<T>,
{
    let mut whitespace_len = 0;
    let mut len = 0;
    let mut chars = Vec::new();

    // Helper is needed to support early returns on `getc_fn` errors
    // (e.g. EOF in the input stream)
    // We don't care about return of helper because modified vars are
    // captured indirectly.
    let _ = || -> Result<(), ()> {
        // atof() is similar to atoi().
        // FIXME: no C99 hexfloat, INF, NAN support
        match count_whitespace_generic(env, &getc_fn, &ungetc_fn, subject, offset) {
            Ok(count) => {
                whitespace_len = count;
            }
            Err(count) => {
                whitespace_len = count;
                return Err(());
            }
        }

        let maybe_sign: u8 = getc_fn(env, subject, offset + whitespace_len + len)?.into();
        if maybe_sign == b'+' || maybe_sign == b'-' || maybe_sign.is_ascii_digit() {
            chars.push(maybe_sign);
            len += 1;
        } else {
            ungetc_fn(env, subject, maybe_sign);
        }

        let mut curr: u8 = getc_fn(env, subject, offset + whitespace_len + len)?.into();
        while (curr as char).is_ascii_digit() {
            chars.push(curr);
            len += 1;
            curr = getc_fn(env, subject, offset + whitespace_len + len)?.into();
        }

        // TODO: assert C locale
        if curr == b'.' {
            chars.push(curr);
            len += 1;
            curr = getc_fn(env, subject, offset + whitespace_len + len)?.into();
            while (curr as char).is_ascii_digit() {
                chars.push(curr);
                len += 1;
                curr = getc_fn(env, subject, offset + whitespace_len + len)?.into();
            }
        }

        if curr.eq_ignore_ascii_case(&b'e') {
            chars.push(curr);
            len += 1;

            let maybe_sign: u8 = getc_fn(env, subject, offset + whitespace_len + len)?.into();
            if maybe_sign == b'+' || maybe_sign == b'-' || maybe_sign.is_ascii_digit() {
                chars.push(maybe_sign);
                len += 1;
            } else {
                ungetc_fn(env, subject, maybe_sign);
            }

            curr = getc_fn(env, subject, offset + whitespace_len + len)?.into();
            while (curr as char).is_ascii_digit() {
                chars.push(curr);
                len += 1;
                curr = getc_fn(env, subject, offset + whitespace_len + len)?.into();
            }
        }
        ungetc_fn(env, subject, curr);

        assert_eq!(chars.len() as u32, len);
        Ok(())
    }();

    let s = std::str::from_utf8(&chars).unwrap();
    log_dbg!("atof_inner_generic('{}')", s);
    s.parse().map(|result| (result, whitespace_len + len))
}

/// A simple wrapper around [str_to_int_inner_generic]
/// for the case of C string and i32.
fn strtol_inner(env: &mut Environment, str: ConstPtr<u8>, base: u32) -> Result<(i32, u32), ()> {
    str_to_int_inner_generic(
        env,
        |env, s, idx| Ok(env.mem.read(s + idx)),
        |_, _, _| (), // could be ignored
        str.cast_mut(),
        0, // starting offset
        base,
        u32::MAX, // max_length
        |s, base| i32::from_str_radix(s, base).unwrap_or(i32::MAX),
        |num| num.checked_mul(-1).unwrap_or(i32::MIN),
    )
}

#[allow(rustdoc::broken_intra_doc_links)] // https://github.com/rust-lang/rust/issues/83049
/// Generic implementation of a conversion helper from string to an integer.
///
/// `getc_fn` is a callback to get next character from `subject`.
/// 3rd parameter in this callback is a index which is safe to ignore
/// (for example, in case of a file stream).
/// Error signifies an abnormal stop of input,
/// such as [crate::libc::stdio::EOF] in the file stream.
/// Note: `'\0'` does not necessary expect to produce an error!
///
/// `ungetc_fn` is a callback to un-get character from `subject`.
/// Could be ignored entirely (for example, in case of a string).
///
/// `subject` is either C string or file stream (for now).
///
/// `offset` defines an offset in `subject` from which conversion starts.
/// Could be ignored entirely (for example, in case of a file stream).
///
/// `base` of conversion.
/// Is mutable because in case of base 0 we need to auto-detect it.
///
/// `from_str_radix_fn` is a callback to actually convert accumulated string
/// to the number.
///
/// `negation_fn` is a callback which specifies how '-' is treated.
///
/// Returns a tuple containing the parsed number in the given base and
/// the length of the number in the string.
///
/// Right now this function is a bit of the mess... We bridge together the
/// worlds of string indexing and file stream processing with questionable
/// results. We have fair amount of integration tests for `strtoul`
/// and `sscanf`/`fscanf`, but some of corner cases are definitely not covered.
/// One idea for cleaning that would be to fully embrace `getc`/`ungetc`
/// approach and get rid of indexing.
/// (Like, let caller to deal with indexing and override `offset` somehow?)
/// TODO: find a more powerful abstraction for generalization
#[allow(clippy::too_many_arguments)]
pub fn str_to_int_inner_generic<
    T,
    U,
    Q,
    F1: Fn(&mut Environment, MutPtr<U>, GuestUSize) -> Result<T, ()>,
    F2: Fn(&mut Environment, MutPtr<U>, u8), // TODO: make last param generic too?
    F3: Fn(&str, u32) -> Q,
    F4: Fn(Q) -> Q,
>(
    env: &mut Environment,
    getc_fn: F1,
    ungetc_fn: F2,
    subject: MutPtr<U>,
    offset: GuestUSize,
    mut base: u32,
    max_length: GuestUSize,
    from_str_radix_fn: F3,
    negation_fn: F4,
) -> Result<(Q, u32), ()>
where
    u8: From<T>,
    Q: Default,
{
    let mut whitespace_len = 0;
    let mut len = 0;
    let mut sign = None;
    let mut prefix_length = 0;
    let mut chars = Vec::new();

    // Helper is needed to support early returns on `getc_fn` errors
    // (e.g. EOF in the input stream)
    // We don't care about return of helper because modified vars are
    // captured indirectly.
    let _ = || -> Result<(), ()> {
        // strtol() doesn't work with a null-terminated string,
        // instead it stops once it hits something that's not a digit,
        // so we have to do some parsing ourselves.
        match count_whitespace_generic(env, &getc_fn, &ungetc_fn, subject, offset) {
            Ok(count) => {
                whitespace_len = count;
            }
            Err(count) => {
                whitespace_len = count;
                return Err(());
            }
        }

        let maybe_sign: u8 = getc_fn(env, subject, offset + whitespace_len + len)?.into();
        if maybe_sign == b'+' || maybe_sign == b'-' {
            sign = Some(maybe_sign);
            prefix_length += 1;
            len += 1;
            if len == max_length {
                return Ok(());
            }
        } else {
            ungetc_fn(env, subject, maybe_sign);
        }
        // We need to do base detection before we can start counting
        // the number length, but after we maybe skipped the sign
        // TODO: detect base and skip prefix in one pass
        if base == 0 {
            let curr: u8 = getc_fn(env, subject, offset + whitespace_len + len)?.into();
            base = if curr == b'0' {
                let next: u8 = getc_fn(env, subject, offset + whitespace_len + len + 1)?.into();
                ungetc_fn(env, subject, next);
                ungetc_fn(env, subject, curr);
                if next == b'x' || next == b'X' {
                    16
                } else {
                    8
                }
            } else {
                ungetc_fn(env, subject, curr);
                10
            }
        }
        // Skipping prefix if needed
        if base == 8 || base == 16 {
            let curr: u8 = getc_fn(env, subject, offset + whitespace_len + len)?.into();
            if curr == b'0' {
                len += 1;
                if len == max_length {
                    return Ok(());
                }
                prefix_length += 1;
                if base == 16 {
                    let next: u8 = getc_fn(env, subject, offset + whitespace_len + len)?.into();
                    if next == b'x' || next == b'X' {
                        len += 1;
                        if len == max_length {
                            return Ok(());
                        }
                        prefix_length += 1;
                    } else {
                        ungetc_fn(env, subject, next);
                    }
                } else {
                    ungetc_fn(env, subject, curr);
                }
            } else {
                ungetc_fn(env, subject, curr);
            }
        }
        let mut curr: u8 = getc_fn(env, subject, offset + whitespace_len + len)?.into();
        while (curr as char).is_digit(base) {
            chars.push(curr);
            len += 1;
            if len == max_length {
                return Ok(());
            }
            curr = getc_fn(env, subject, offset + whitespace_len + len)?.into();
        }
        ungetc_fn(env, subject, curr);
        assert_eq!(chars.len() as u32, len - prefix_length);
        Ok(())
    }();

    let s = std::str::from_utf8(&chars).unwrap();
    log_dbg!("strtol_inner_generic('{}', {})", s, base);

    assert!((2..=36).contains(&base));
    let magnitude_len = len - prefix_length;
    let res = if magnitude_len > 0 {
        // TODO: set errno on range errors
        let mut res = from_str_radix_fn(s, base);
        if sign == Some(b'-') {
            res = negation_fn(res);
        }
        res
    } else {
        // Special case - prefix of invalid octal number is a valid number 0
        if base == 8 && prefix_length > 0 {
            return Ok((Q::default(), whitespace_len + prefix_length));
        }
        return Err(());
    };
    Ok((res, whitespace_len + len))
}
