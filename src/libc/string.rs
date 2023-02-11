/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `string.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr};
use crate::Environment;
use std::cmp::Ordering;

#[derive(Default)]
pub struct State {
    strtok: Option<MutPtr<u8>>,
}

fn memset(env: &mut Environment, dest: MutVoidPtr, ch: i32, count: GuestUSize) -> MutVoidPtr {
    env.mem.bytes_at_mut(dest.cast(), count).fill(ch as u8);
    dest
}

fn memcpy(
    env: &mut Environment,
    dest: MutVoidPtr,
    src: ConstVoidPtr,
    size: GuestUSize,
) -> MutVoidPtr {
    env.mem.memmove(dest, src, size);
    dest
}

fn memmove(
    env: &mut Environment,
    dest: MutVoidPtr,
    src: ConstVoidPtr,
    size: GuestUSize,
) -> MutVoidPtr {
    env.mem.memmove(dest, src, size);
    dest
}

fn strlen(env: &mut Environment, s: ConstPtr<u8>) -> GuestUSize {
    env.mem.cstr_at(s).len().try_into().unwrap()
}

fn strcpy(env: &mut Environment, dest: MutPtr<u8>, src: ConstPtr<u8>) -> MutPtr<u8> {
    {
        let (mut dest, mut src) = (dest, src);
        loop {
            let c = env.mem.read(src);
            env.mem.write(dest, c);
            if c == b'\0' {
                break;
            }
            dest += 1;
            src += 1;
        }
    }
    dest
}
fn strcat(env: &mut Environment, dest: MutPtr<u8>, src: ConstPtr<u8>) -> MutPtr<u8> {
    {
        let dest = dest + strlen(env, dest.cast_const());
        strcpy(env, dest, src);
    }
    dest
}

pub(super) fn strdup(env: &mut Environment, src: ConstPtr<u8>) -> MutPtr<u8> {
    let len = strlen(env, src);
    let new = env.mem.alloc(len + 1).cast();
    strcpy(env, new, src)
}

fn strcmp(env: &mut Environment, a: ConstPtr<u8>, b: ConstPtr<u8>) -> i32 {
    let mut offset = 0;
    loop {
        let char_a = env.mem.read(a + offset);
        let char_b = env.mem.read(b + offset);
        offset += 1;

        match char_a.cmp(&char_b) {
            Ordering::Less => return -1,
            Ordering::Greater => return 1,
            Ordering::Equal => {
                if char_a == b'\0' {
                    return 0;
                } else {
                    continue;
                }
            }
        }
    }
}

fn strncmp(env: &mut Environment, a: ConstPtr<u8>, b: ConstPtr<u8>, n: GuestUSize) -> i32 {
    if n == 0 {
        return 0;
    }

    let mut offset = 0;
    loop {
        let char_a = env.mem.read(a + offset);
        let char_b = env.mem.read(b + offset);
        offset += 1;

        match char_a.cmp(&char_b) {
            Ordering::Less => return -1,
            Ordering::Greater => return 1,
            Ordering::Equal => {
                if offset == n || char_a == b'\0' {
                    return 0;
                } else {
                    continue;
                }
            }
        }
    }
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

fn strstr(env: &mut Environment, string: ConstPtr<u8>, substring: ConstPtr<u8>) -> ConstPtr<u8> {
    let mut offset = 0;
    loop {
        let mut inner_offset = 0;
        loop {
            let char_string = env.mem.read(string + offset + inner_offset);
            let char_substring = env.mem.read(substring + inner_offset);
            if char_substring == b'\0' {
                return string + offset;
            } else if char_string == b'\0' {
                return Ptr::null();
            } else if char_string != char_substring {
                break;
            } else {
                inner_offset += 1;
            }
        }
        offset += 1;
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(memset(_, _, _)),
    export_c_func!(memcpy(_, _, _)),
    export_c_func!(memmove(_, _, _)),
    export_c_func!(strlen(_)),
    export_c_func!(strcpy(_, _)),
    export_c_func!(strcat(_, _)),
    export_c_func!(strdup(_)),
    export_c_func!(strcmp(_, _)),
    export_c_func!(strncmp(_, _, _)),
    export_c_func!(strtok(_, _)),
    export_c_func!(strstr(_, _)),
];
