/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `string.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr};
use crate::Environment;

use super::generic_char::GenericChar;

#[derive(Default)]
pub struct State {
    strtok: Option<MutPtr<u8>>,
}

fn strtok(env: &mut Environment, s: MutPtr<u8>, sep: ConstPtr<u8>) -> MutPtr<u8> {
    let s = if s.is_null() {
        let state = env.libc_state.string.strtok.unwrap();
        if state.is_null() {
            env.libc_state.string.strtok = None;
            return Ptr::null();
        }
        state
    } else {
        s
    };

    let sep = env.mem.cstr_at(sep);

    let mut token_start = s;
    loop {
        let c = env.mem.read(token_start);
        if c == b'\0' {
            env.libc_state.string.strtok = None;
            return Ptr::null();
        } else if sep.contains(&c) {
            token_start += 1;
        } else {
            break;
        }
    }

    let mut token_end = token_start;
    let next_token = loop {
        let c = env.mem.read(token_end);
        if sep.contains(&c) {
            env.mem.write(token_end, b'\0');
            break token_end + 1;
        } else if c == b'\0' {
            break Ptr::null();
        } else {
            token_end += 1;
        }
    };

    env.libc_state.string.strtok = Some(next_token);

    token_start
}

// Functions shared with wchar.rs

fn memset(env: &mut Environment, dest: MutVoidPtr, ch: i32, count: GuestUSize) -> MutVoidPtr {
    GenericChar::<u8>::memset(env, dest.cast(), ch as u8, count).cast()
}
fn memcpy(
    env: &mut Environment,
    dest: MutVoidPtr,
    src: ConstVoidPtr,
    size: GuestUSize,
) -> MutVoidPtr {
    GenericChar::<u8>::memcpy(env, dest.cast(), src.cast(), size).cast()
}
fn memmove(
    env: &mut Environment,
    dest: MutVoidPtr,
    src: ConstVoidPtr,
    size: GuestUSize,
) -> MutVoidPtr {
    GenericChar::<u8>::memmove(env, dest.cast(), src.cast(), size).cast()
}
fn memchr(env: &mut Environment, string: ConstVoidPtr, c: i32, size: GuestUSize) -> ConstVoidPtr {
    GenericChar::<u8>::memchr(env, string.cast(), c as u8, size).cast()
}
fn memcmp(env: &mut Environment, a: ConstVoidPtr, b: ConstVoidPtr, size: GuestUSize) -> i32 {
    GenericChar::<u8>::memcmp(env, a.cast(), b.cast(), size)
}
pub(super) fn strlen(env: &mut Environment, s: ConstPtr<u8>) -> GuestUSize {
    GenericChar::<u8>::strlen(env, s)
}
fn strcpy(env: &mut Environment, dest: MutPtr<u8>, src: ConstPtr<u8>) -> MutPtr<u8> {
    GenericChar::<u8>::strcpy(env, dest, src)
}
fn __strcpy_chk(
    env: &mut Environment,
    dest: MutPtr<u8>,
    src: ConstPtr<u8>,
    _size: GuestUSize,
) -> MutPtr<u8> {
    log!("Warning: ignore a buffer overflow check in __strcpy_chk");
    strcpy(env, dest, src)
}
fn strcat(env: &mut Environment, dest: MutPtr<u8>, src: ConstPtr<u8>) -> MutPtr<u8> {
    GenericChar::<u8>::strcat(env, dest, src)
}
fn strncpy(
    env: &mut Environment,
    dest: MutPtr<u8>,
    src: ConstPtr<u8>,
    size: GuestUSize,
) -> MutPtr<u8> {
    GenericChar::<u8>::strncpy(env, dest, src, size)
}
pub(super) fn strdup(env: &mut Environment, src: ConstPtr<u8>) -> MutPtr<u8> {
    GenericChar::<u8>::strdup(env, src)
}
fn strcmp(env: &mut Environment, a: ConstPtr<u8>, b: ConstPtr<u8>) -> i32 {
    GenericChar::<u8>::strcmp(env, a, b)
}
fn strncmp(env: &mut Environment, a: ConstPtr<u8>, b: ConstPtr<u8>, n: GuestUSize) -> i32 {
    GenericChar::<u8>::strncmp(env, a, b, n)
}
fn strncat(
    env: &mut Environment,
    s1: ConstPtr<u8>,
    s2: ConstPtr<u8>,
    n: GuestUSize,
) -> ConstPtr<u8> {
    GenericChar::<u8>::strncat(env, s1, s2, n)
}
fn strstr(env: &mut Environment, string: ConstPtr<u8>, substring: ConstPtr<u8>) -> ConstPtr<u8> {
    GenericChar::<u8>::strstr(env, string, substring)
}
fn strchr(env: &mut Environment, path: ConstPtr<u8>, c: u8) -> ConstPtr<u8> {
    GenericChar::<u8>::strchr(env, path, c)
}
fn strrchr(env: &mut Environment, path: ConstPtr<u8>, c: u8) -> ConstPtr<u8> {
    GenericChar::<u8>::strrchr(env, path, c)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(strtok(_, _)),
    // Functions shared with wchar.rs
    export_c_func!(memset(_, _, _)),
    export_c_func!(memcpy(_, _, _)),
    export_c_func!(memmove(_, _, _)),
    export_c_func!(memchr(_, _, _)),
    export_c_func!(memcmp(_, _, _)),
    export_c_func!(strlen(_)),
    export_c_func!(strcpy(_, _)),
    export_c_func!(__strcpy_chk(_, _, _)),
    export_c_func!(strcat(_, _)),
    export_c_func!(strncpy(_, _, _)),
    export_c_func!(strdup(_)),
    export_c_func!(strcmp(_, _)),
    export_c_func!(strncmp(_, _, _)),
    export_c_func!(strncat(_, _, _)),
    export_c_func!(strstr(_, _)),
    export_c_func!(strchr(_, _)),
    export_c_func!(strrchr(_, _)),
];
