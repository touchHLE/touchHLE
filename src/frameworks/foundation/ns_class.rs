//! `NSClassFromString()`

use super::ns_string;
use crate::dyld::{export_c_func, FunctionExports};
use crate::objc::{id, nil};
use crate::{Environment, msg};

fn NSClassFromString(
    env: &mut Environment,
    class: id, // NSString
) {
    // TODO: avoid copy
    let class_name = ns_string::to_rust_string(env, class);

    log_dbg!("NSClassFromString({:?}, ...)", class_name);

    // TODO: We should use this, but i dont want to make a MPMediaPickerControllerDelegate yet
    // let class = env.objc.get_known_class(&class_name, &mut env.mem);
    msg![env; nil new]
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(NSClassFromString(_))];
