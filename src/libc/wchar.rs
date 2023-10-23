/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `wchar.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, GuestUSize, MutPtr};
use crate::Environment;

use super::generic_char::GenericChar;

#[allow(non_camel_case_types)]
pub type wchar_t = i32; // not sure if this signedness is correct

#[allow(non_camel_case_types)]
type wint_t = i32;

const WEOF: wint_t = -1;

fn btowc(_env: &mut Environment, c: i32) -> wint_t {
    let c = c as u8;
    // Assuming ASCII locale, like in ctype.rs.
    if c.is_ascii() {
        c as wint_t
    } else {
        WEOF
    }
}

fn wctob(_env: &mut Environment, c: wint_t) -> i32 {
    // Assuming ASCII locale, like in ctype.rs.
    if u32::try_from(c)
        .ok()
        .and_then(char::from_u32)
        .map_or(false, |c| c.is_ascii())
    {
        c
    } else {
        WEOF
    }
}

// Functions shared with string.rs

fn wmemset(
    env: &mut Environment,
    dest: MutPtr<wchar_t>,
    ch: wchar_t,
    count: GuestUSize,
) -> MutPtr<wchar_t> {
    GenericChar::<wchar_t>::memset(env, dest, ch, count)
}
fn wmemcpy(
    env: &mut Environment,
    dest: MutPtr<wchar_t>,
    src: ConstPtr<wchar_t>,
    size: GuestUSize,
) -> MutPtr<wchar_t> {
    GenericChar::<wchar_t>::memcpy(env, dest, src, size)
}
fn wmemmove(
    env: &mut Environment,
    dest: MutPtr<wchar_t>,
    src: ConstPtr<wchar_t>,
    size: GuestUSize,
) -> MutPtr<wchar_t> {
    GenericChar::<wchar_t>::memmove(env, dest, src, size)
}
fn wmemchr(
    env: &mut Environment,
    string: ConstPtr<wchar_t>,
    c: wchar_t,
    size: GuestUSize,
) -> ConstPtr<wchar_t> {
    GenericChar::<wchar_t>::memchr(env, string, c, size)
}
fn wmemcmp(
    env: &mut Environment,
    a: ConstPtr<wchar_t>,
    b: ConstPtr<wchar_t>,
    size: GuestUSize,
) -> i32 {
    GenericChar::<wchar_t>::memcmp(env, a, b, size)
}
fn wcslen(env: &mut Environment, s: ConstPtr<wchar_t>) -> GuestUSize {
    GenericChar::<wchar_t>::strlen(env, s)
}
fn wcscpy(env: &mut Environment, dest: MutPtr<wchar_t>, src: ConstPtr<wchar_t>) -> MutPtr<wchar_t> {
    GenericChar::<wchar_t>::strcpy(env, dest, src, GuestUSize::MAX)
}
fn wcscat(env: &mut Environment, dest: MutPtr<wchar_t>, src: ConstPtr<wchar_t>) -> MutPtr<wchar_t> {
    GenericChar::<wchar_t>::strcat(env, dest, src, GuestUSize::MAX)
}
fn wcsncpy(
    env: &mut Environment,
    dest: MutPtr<wchar_t>,
    src: ConstPtr<wchar_t>,
    size: GuestUSize,
) -> MutPtr<wchar_t> {
    GenericChar::<wchar_t>::strncpy(env, dest, src, size)
}
fn wcsdup(env: &mut Environment, src: ConstPtr<wchar_t>) -> MutPtr<wchar_t> {
    GenericChar::<wchar_t>::strdup(env, src)
}
fn wcscmp(env: &mut Environment, a: ConstPtr<wchar_t>, b: ConstPtr<wchar_t>) -> i32 {
    GenericChar::<wchar_t>::strcmp(env, a, b)
}
fn wcsncmp(
    env: &mut Environment,
    a: ConstPtr<wchar_t>,
    b: ConstPtr<wchar_t>,
    n: GuestUSize,
) -> i32 {
    GenericChar::<wchar_t>::strncmp(env, a, b, n)
}
fn wcsncat(
    env: &mut Environment,
    s1: MutPtr<wchar_t>,
    s2: ConstPtr<wchar_t>,
    n: GuestUSize,
) -> MutPtr<wchar_t> {
    GenericChar::<wchar_t>::strncat(env, s1, s2, n)
}
fn wcsstr(
    env: &mut Environment,
    wcsing: ConstPtr<wchar_t>,
    subwcsing: ConstPtr<wchar_t>,
) -> ConstPtr<wchar_t> {
    GenericChar::<wchar_t>::strstr(env, wcsing, subwcsing)
}
fn wcschr(env: &mut Environment, wcsing: ConstPtr<wchar_t>, wchar: wchar_t) -> ConstPtr<wchar_t> {
    GenericChar::<wchar_t>::strchr(env, wcsing, wchar)
}
fn wcsrchr(env: &mut Environment, wcsing: ConstPtr<wchar_t>, wchar: wchar_t) -> ConstPtr<wchar_t> {
    GenericChar::<wchar_t>::strrchr(env, wcsing, wchar)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(btowc(_)),
    export_c_func!(wctob(_)),
    // Functions shared with string.rs
    export_c_func!(wmemset(_, _, _)),
    export_c_func!(wmemcpy(_, _, _)),
    export_c_func!(wmemmove(_, _, _)),
    export_c_func!(wmemchr(_, _, _)),
    export_c_func!(wmemcmp(_, _, _)),
    export_c_func!(wcslen(_)),
    export_c_func!(wcscpy(_, _)),
    export_c_func!(wcscat(_, _)),
    export_c_func!(wcsncpy(_, _, _)),
    export_c_func!(wcsdup(_)),
    export_c_func!(wcscmp(_, _)),
    export_c_func!(wcsncmp(_, _, _)),
    export_c_func!(wcsncat(_, _, _)),
    export_c_func!(wcsstr(_, _)),
    export_c_func!(wcschr(_, _)),
    export_c_func!(wcsrchr(_, _)),
];
