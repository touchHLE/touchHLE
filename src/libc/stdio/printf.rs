//! `printf` function family.

use crate::abi::VAList;
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, MutPtr};
use crate::Environment;

fn printf_inner(env: &mut Environment, format: ConstPtr<u8>, mut args: VAList) -> Vec<u8> {
    log_dbg!(
        "Processing format string {:?}",
        env.mem.cstr_at_utf8(format)
    );

    let mut res = Vec::<u8>::new();

    let mut current_format = format;

    loop {
        let c = env.mem.read(current_format);
        current_format += 1;

        if c == b'\0' {
            break;
        }
        if c != b'%' {
            res.push(c);
            continue;
        }

        let specifier = env.mem.read(current_format);
        current_format += 1;

        assert!(specifier != b'\0');
        if specifier == b'%' {
            res.push(b'%');
            continue;
        }

        match specifier {
            b's' => {
                let c_string: ConstPtr<u8> = args.next(env);
                res.extend_from_slice(env.mem.cstr_at(c_string));
            }
            b'd' | b'i' => {
                let int: i32 = args.next(env);
                // TODO: avoid copy?
                res.extend_from_slice(format!("{}", int).as_bytes());
            }
            // TODO: more specifiers
            _ => unimplemented!("Format character '{}'", specifier as char),
        }
    }

    log_dbg!("=> {:?}", std::str::from_utf8(&res));

    res
}

fn sprintf(env: &mut Environment, dest: MutPtr<u8>, format: ConstPtr<u8>, args: VAList) -> i32 {
    let res = printf_inner(env, format, args);

    log_dbg!("sprintf({:?}, {:?}, ...)", dest, format);

    let dest_slice = env
        .mem
        .bytes_at_mut(dest, (res.len() + 1).try_into().unwrap());
    for (i, &byte) in res.iter().chain(b"\0".iter()).enumerate() {
        dest_slice[i] = byte;
    }

    res.len().try_into().unwrap()
}

// TODO: more printf variants

pub const FUNCTIONS: FunctionExports = &[export_c_func!(sprintf(_, _, _))];
