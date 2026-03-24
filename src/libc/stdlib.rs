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
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr};
use crate::Environment;
use std::str::FromStr;

pub mod qsort;

#[derive(Default)]
pub struct State {
    rand: u32,
    random: u32,
    arc4random: u32,
}

fn malloc(env: &mut Environment, size: GuestUSize) -> MutVoidPtr {
    set_errno(env, 0);
    env.mem.alloc(size)
}

fn malloc_size(env: &mut Environment, ptr: ConstVoidPtr) -> GuestUSize {
    env.mem.malloc_size(ptr)
}

fn calloc(env: &mut Environment, count: GuestUSize, size: GuestUSize) -> MutVoidPtr {
    set_errno(env, 0);
    let total = size.checked_mul(count).unwrap();
    env.mem.calloc(total)
}

fn realloc(env: &mut Environment, ptr: MutVoidPtr, size: GuestUSize) -> MutVoidPtr {
    set_errno(env, 0);
    if ptr.is_null() {
        return malloc(env, size);
    }
    env.mem.realloc(ptr, size)
}

fn free(env: &mut Environment, ptr: MutVoidPtr) {
    if env.objc.get_host_object(ptr.cast()).is_some() {
        log!(
            "App attempted to call free({:?}) on an object, calling dealloc_object() instead!",
            ptr
        );
        env.objc.dealloc_object(ptr.cast(), &mut env.mem);
        return;
    }
    set_errno(env, 0);
    if ptr.is_null() {
        return;
    }
    env.mem.free(ptr);
}

fn atexit(_env: &mut Environment, func: GuestFunction) -> i32 {
    log!("TODO: atexit({:?}) (unimplemented)", func);
    0 
}

fn count_whitespace_generic<
    T,
    U,
    F1: Fn(&mut Environment, MutPtr<U>, GuestUSize) -> Result<T, ()>,
    F2: Fn(&mut Environment, MutPtr<U>, u8),
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
    set_errno(env, 0);
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
    set_errno(env, 0);
    log_dbg!("strtod nptr {}", env.mem.cstr_at_utf8(nptr).unwrap());
    let (res, len) = atof_inner(env, nptr).unwrap_or((0.0, 0));
    if !endptr.is_null() {
        env.mem.write(endptr, (nptr + len).cast_mut());
    }
    res
}

fn prng(state: u32) -> u32 {
    let mut state: u32 = state.max(1);
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

fn srandom(env: &mut Environment, seed: u32) {
    set_errno(env, 0);
    env.libc_state.stdlib.random = seed;
}
fn random(env: &mut Environment) -> i32 {
    set_errno(env, 0);
    env.libc_state.stdlib.random = prng(env.libc_state.stdlib.random);
    (env.libc_state.stdlib.random as i32) & RAND_MAX
}

fn arc4random(env: &mut Environment) -> u32 {
    env.libc_state.stdlib.arc4random = prng(env.libc_state.stdlib.arc4random);
    env.libc_state.stdlib.arc4random
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
    log_dbg!("getenv({:?}) => {:?}", name, value);
    value
}

fn setenv(env: &mut Environment, name: ConstPtr<u8>, value: ConstPtr<u8>, overwrite: i32) -> i32 {
    set_errno(env, 0);
    let name_cstr = env.mem.cstr_at(name);
    if let Some(&existing) = env.env_vars.get(name_cstr) {
        if overwrite == 0 {
            return 0;
        }
        env.mem.free(existing.cast());
    };
    let value = super::string::strdup(env, value);
    let name_cstr = env.mem.cstr_at(name); 
    env.env_vars.insert(name_cstr.to_vec(), value);
    0
}

fn unsetenv(env: &mut Environment, name: ConstPtr<u8>) -> i32 {
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
    compare_callback: GuestFunction,
) -> ConstVoidPtr {
    let mut low = 0;
    let mut len = item_count;
    while len > 0 {
        let half_len = len / 2;
        let item: ConstVoidPtr = (items.cast::<u8>() + item_size * (low + half_len)).cast();
        let cmp_result: i32 = compare_callback.call_from_host(env, (key, item));
        (low, len) = match cmp_result.signum() {
            0 => return item,
            1 => (low + half_len + 1, len - half_len - 1),
            -1 => (low, half_len),
            _ => unreachable!(),
        }
    }
    Ptr::null()
}

fn strtof(env: &mut Environment, nptr: ConstPtr<u8>, endptr: MutPtr<ConstPtr<u8>>) -> f32 {
    set_errno(env, 0);
    let (number, length) = atof_inner(env, nptr).unwrap_or((0.0, 0));
    if !endptr.is_null() {
        env.mem.write(endptr, nptr + length);
    }
    number as f32
}

pub fn strtoul(env: &mut Environment, str: ConstPtr<u8>, endptr: MutPtr<MutPtr<u8>>, base: i32) -> u32 {
    set_errno(env, 0);
    let parse_res = str_to_int_inner_generic(
        env,
        |env, s, idx| Ok(env.mem.read(s + idx)),
        |_, _, _| (),
        str.cast_mut(),
        0,
        base.try_into().unwrap(),
        u32::MAX,
        |s, base| u32::from_str_radix(s, base).unwrap_or(u32::MAX),
        |num| num.wrapping_neg(),
    );
    match parse_res {
        Ok((res, len)) => {
            if !endptr.is_null() { env.mem.write(endptr, (str + len).cast_mut()); }
            res
        }
        Err(_) => {
            if !endptr.is_null() { env.mem.write(endptr, str.cast_mut()); }
            0
        }
    }
}

fn strtoull(env: &mut Environment, str: ConstPtr<u8>, endptr: MutPtr<MutPtr<u8>>, base: i32) -> u64 {
    set_errno(env, 0);
    let parse_res = str_to_int_inner_generic(
        env,
        |env, s, idx| Ok(env.mem.read(s + idx)),
        |_, _, _| (),
        str.cast_mut(),
        0,
        base.try_into().unwrap(),
        u32::MAX,
        |s, base| u64::from_str_radix(s, base).unwrap_or(u64::MAX),
        |num| num.wrapping_neg(),
    );
    match parse_res {
        Ok((res, len)) => {
            if !endptr.is_null() { env.mem.write(endptr, (str + len).cast_mut()); }
            res
        }
        Err(_) => {
            if !endptr.is_null() { env.mem.write(endptr, str.cast_mut()); }
            0
        }
    }
}

fn strtol(env: &mut Environment, str: ConstPtr<u8>, endptr: MutPtr<MutPtr<u8>>, base: i32) -> i32 {
    set_errno(env, 0);
    match strtol_inner(env, str, base as u32) {
        Ok((res, len)) => {
            if !endptr.is_null() { env.mem.write(endptr, (str + len).cast_mut()); }
            res
        }
        Err(_) => {
            if !endptr.is_null() { env.mem.write(endptr, str.cast_mut()); }
            0
        }
    }
}

fn realpath(env: &mut Environment, file_name: ConstPtr<u8>, resolve_name: MutPtr<u8>) -> MutPtr<u8> {
    assert!(!resolve_name.is_null());
    let file_name_str = env.mem.cstr_at_utf8(file_name).unwrap();
    let resolved = resolve_path(GuestPath::new(file_name_str), Some(env.fs.working_directory()));
    let result = format!("/{}", resolved.join("/"));
    env.mem.bytes_at_mut(resolve_name, result.len() as GuestUSize).copy_from_slice(result.as_bytes());
    env.mem.write(resolve_name + result.len() as GuestUSize, b'\0');
    resolve_name
}

fn mbstowcs(env: &mut Environment, pwcs: MutPtr<wchar_t>, s: ConstPtr<u8>, n: GuestUSize) -> GuestUSize {
    set_errno(env, 0);
    let ctype_locale = setlocale(env, LC_CTYPE, Ptr::null());
    assert_eq!(env.mem.read(ctype_locale), b'C');
    let size = strlen(env, s);
    let to_write = size.min(n);
    for i in 0..to_write {
        let c = env.mem.read(s + i);
        env.mem.write(pwcs + i, c as wchar_t);
    }
    if to_write < n { env.mem.write(pwcs + to_write, wchar_t::default()); }
    to_write
}

fn wcstombs(env: &mut Environment, s: ConstPtr<u8>, pwcs: MutPtr<wchar_t>, n: GuestUSize) -> GuestUSize {
    let ctype_locale = setlocale(env, LC_CTYPE, Ptr::null());
    assert_eq!(env.mem.read(ctype_locale), b'C');
    if n == 0 { return 0; }
    let wcstr = env.mem.wcstr_at(pwcs);
    let len = (wcstr.len() as GuestUSize).min(n);
    env.mem.bytes_at_mut(s.cast_mut(), len).copy_from_slice(wcstr.as_bytes());
    if len < n { env.mem.write((s + len).cast_mut(), b'\0'); }
    len
}

fn system(env: &mut Environment, cmd: ConstPtr<u8>) -> i32 {
    if cmd.is_null() { return 0; }
    log!("system({:?})", env.mem.cstr_at_utf8(cmd));
    todo!()
}

// Исправлено: Удалено избыточное -> ()
fn ___assert_rtn(
    env: &mut Environment,
    func: ConstPtr<u8>,
    file: ConstPtr<u8>,
    line: i32,
    msg: ConstPtr<u8>,
) {
    let func_str = env.mem.cstr_at_utf8(func).unwrap_or("unknown_func");
    let file_str = env.mem.cstr_at_utf8(file).unwrap_or("unknown_file");
    let msg_str = env.mem.cstr_at_utf8(msg).unwrap_or("no message");

    panic!(
        "\n[GUEST ASSERTION FAILED]\nMessage: \"{}\"\nFunction: {}\nFile: {}\nLine: {}\n",
        msg_str, func_str, file_str, line
    );
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(malloc(_)),
    export_c_func!(malloc_size(_)),
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
    export_c_func!(unsetenv(_)),
    export_c_func!(exit(_)),
    export_c_func!(bsearch(_, _, _, _, _)),
    export_c_func!(strtof(_, _)),
    export_c_func!(strtoul(_, _, _)),
    export_c_func!(strtoull(_, _, _)),
    export_c_func!(strtol(_, _, _)),
    export_c_func!(realpath(_, _)),
    export_c_func_aliased!("realpath$DARWIN_EXTSN", realpath(_, _)),
    export_c_func!(mbstowcs(_, _, _)),
    export_c_func!(wcstombs(_, _, _)),
    export_c_func!(system(_)),
    export_c_func_aliased!("___assert_rtn", ___assert_rtn(_, _, _, _)),
];

pub fn atof_inner(env: &mut Environment, s: ConstPtr<u8>) -> Result<(f64, u32), <f64 as FromStr>::Err> {
    atof_inner_generic(env, |env, s, idx| Ok(env.mem.read(s + idx)), |_, _, _| (), s.cast_mut(), 0)
}

pub fn atof_inner_generic<T, U, F1, F2>(
    env: &mut Environment,
    getc_fn: F1,
    ungetc_fn: F2,
    subject: MutPtr<U>,
    offset: GuestUSize,
) -> Result<(f64, u32), <f64 as FromStr>::Err>
where
    u8: From<T>,
    F1: Fn(&mut Environment, MutPtr<U>, GuestUSize) -> Result<T, ()>,
    F2: Fn(&mut Environment, MutPtr<U>, u8),
{
    let mut whitespace_len = 0;
    let mut len = 0;
    let mut chars = Vec::new();
    let _ = || -> Result<(), ()> {
        match count_whitespace_generic(env, &getc_fn, &ungetc_fn, subject, offset) {
            Ok(count) => whitespace_len = count,
            Err(count) => { whitespace_len = count; return Err(()); }
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
            
            // Исправлено: заменено is_digit(10) на is_ascii_digit()
            while (curr as char).is_ascii_digit() {
                chars.push(curr);
                len += 1;
                curr = getc_fn(env, subject, offset + whitespace_len + len)?.into();
            }
        }
        ungetc_fn(env, subject, curr);
        Ok(())
    }();
    let s = std::str::from_utf8(&chars).unwrap();
    s.parse().map(|result| (result, whitespace_len + len))
}

fn strtol_inner(env: &mut Environment, str: ConstPtr<u8>, base: u32) -> Result<(i32, u32), ()> {
    str_to_int_inner_generic(
        env,
        |env, s, idx| Ok(env.mem.read(s + idx)),
        |_, _, _| (),
        str.cast_mut(),
        0,
        base,
        u32::MAX,
        |s, base| i32::from_str_radix(s, base).unwrap_or(i32::MAX),
        |num| num.checked_mul(-1).unwrap_or(i32::MIN),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn str_to_int_inner_generic<T, U, Q, F1, F2, F3, F4>(
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
    F1: Fn(&mut Environment, MutPtr<U>, GuestUSize) -> Result<T, ()>,
    F2: Fn(&mut Environment, MutPtr<U>, u8),
    F3: Fn(&str, u32) -> Q,
    F4: Fn(Q) -> Q,
{
    let mut whitespace_len = 0;
    let mut len = 0;
    let mut sign = None;
    let mut prefix_length = 0;
    let mut chars = Vec::new();
    let _ = || -> Result<(), ()> {
        match count_whitespace_generic(env, &getc_fn, &ungetc_fn, subject, offset) {
            Ok(count) => whitespace_len = count,
            Err(count) => { whitespace_len = count; return Err(()); }
        }
        let maybe_sign: u8 = getc_fn(env, subject, offset + whitespace_len + len)?.into();
        if maybe_sign == b'+' || maybe_sign == b'-' {
            sign = Some(maybe_sign);
            prefix_length += 1;
            len += 1;
            if len == max_length { return Ok(()); }
        } else {
            ungetc_fn(env, subject, maybe_sign);
        }
        if base == 0 {
            let curr: u8 = getc_fn(env, subject, offset + whitespace_len + len)?.into();
            base = if curr == b'0' {
                let next: u8 = getc_fn(env, subject, offset + whitespace_len + len + 1)?.into();
                ungetc_fn(env, subject, next);
                ungetc_fn(env, subject, curr);
                if next == b'x' || next == b'X' { 16 } else { 8 }
            } else {
                ungetc_fn(env, subject, curr);
                10
            }
        }
        if base == 8 || base == 16 {
            let curr: u8 = getc_fn(env, subject, offset + whitespace_len + len)?.into();
            if curr == b'0' {
                len += 1;
                if len == max_length { return Ok(()); }
                prefix_length += 1;
                if base == 16 {
                    let next: u8 = getc_fn(env, subject, offset + whitespace_len + len)?.into();
                    if next == b'x' || next == b'X' {
                        len += 1;
                        if len == max_length { return Ok(()); }
                        prefix_length += 1;
                    } else { ungetc_fn(env, subject, next); }
                } else { ungetc_fn(env, subject, curr); }
            } else { ungetc_fn(env, subject, curr); }
        }
        let mut curr: u8 = getc_fn(env, subject, offset + whitespace_len + len)?.into();
        while (curr as char).is_digit(base) {
            chars.push(curr);
            len += 1;
            if len == max_length { return Ok(()); }
            curr = getc_fn(env, subject, offset + whitespace_len + len)?.into();
        }
        ungetc_fn(env, subject, curr);
        Ok(())
    }();
    let s = std::str::from_utf8(&chars).unwrap();
    let magnitude_len = len - prefix_length;
    let res = if magnitude_len > 0 {
        let mut res = from_str_radix_fn(s, base);
        if sign == Some(b'-') { res = negation_fn(res); }
        res
    } else {
        if base == 8 && prefix_length > 0 { return Ok((Q::default(), whitespace_len + prefix_length)); }
        return Err(());
    };
    Ok((res, whitespace_len + len))
}

