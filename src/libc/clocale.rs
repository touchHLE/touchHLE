/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `clocale.h`

use crate::{
    dyld::FunctionExports,
    environment::Environment,
    export_c_func,
    mem::{ConstPtr, MutPtr},
};

fn setlocale(env: &mut Environment, _category: i32, _locale: ConstPtr<u8>) -> MutPtr<u8> {
    //log!("TODO: _setlocale({}, {:?})", category, locale);
    env.mem.alloc_and_write_cstr(b"")
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(setlocale(_, _))];
