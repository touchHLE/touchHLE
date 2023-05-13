/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Functions under `Objective-C Runtime Utilities`, just `NSStringFromClass` right now.

use crate::{
    dyld::FunctionExports,
    environment::Environment,
    export_c_func,
    frameworks::foundation::ns_string,
    objc::{class_getName_inner, id, nil},
};

pub(super) fn NSStringFromClass(env: &mut Environment, class: id) -> id {
    if class == nil {
        return nil;
    }
    let class_string = class_getName_inner(&mut env.objc, class).to_string();
    ns_string::from_rust_string(env, class_string)
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(NSStringFromClass(_))];
