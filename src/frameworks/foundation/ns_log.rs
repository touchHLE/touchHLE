//! `NSLog()`, `NSLogv()`

use super::ns_string;
use crate::abi::DotDotDot;
use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::stdio::printf::printf_inner;
use crate::objc::id;
use crate::Environment;
use std::io::Write;

fn NSLog(
    env: &mut Environment,
    format: id, // NSString
    args: DotDotDot,
) {
    // TODO: avoid copy
    let format_string = ns_string::to_rust_string(env, format);

    log_dbg!("NSLog({:?} ({:?}), ...)", format, format_string);

    let res = printf_inner::<true, _>(
        env,
        |_, idx| {
            if idx as usize == format_string.len() {
                b'\0'
            } else {
                format_string.as_bytes()[idx as usize]
            }
        },
        args.start(),
    );
    // TODO: The real NSLog also includes a process name, thread ID and
    // timestamp. Maybe we should add our own prefix.
    let _ = std::io::stdout().write_all(&res);
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(NSLog(_, _))];
