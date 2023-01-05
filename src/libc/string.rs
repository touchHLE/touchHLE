//! `string.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, GuestUSize, MutPtr};
use crate::Environment;

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

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(strlen(_)),
    export_c_func!(strcpy(_, _)),
    export_c_func!(strcat(_, _)),
];
