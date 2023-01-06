//! `string.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, GuestUSize, MutPtr};
use crate::Environment;
use std::cmp::Ordering;

fn strlen(env: &mut Environment, s: ConstPtr<u8>) -> GuestUSize {
    let mut size = 0;
    while env.mem.read(s + size) != b'\0' {
        size += 1;
    }
    size
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

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(strlen(_)),
    export_c_func!(strcpy(_, _)),
    export_c_func!(strcat(_, _)),
    export_c_func!(strcmp(_, _)),
];
