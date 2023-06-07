//! `NSLog()`, `NSLogv()`

use super::ns_string;
use crate::abi::DotDotDot;
use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::stdio::printf::printf_inner;
use crate::objc::id;
use crate::Environment;

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
    // TODO: Should we include a timestamp, like the real NSLog?
    echo!(
        "{}[{}] {}",
        env.bundle.executable_path().file_name().unwrap(),
        env.current_thread,
        String::from_utf8_lossy(&res)
    );
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(NSLog(_, _))];
