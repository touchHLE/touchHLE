/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `clocale.h`

use std::collections::hash_map::Entry;

use crate::{
    dyld::FunctionExports,
    environment::Environment,
    export_c_func,
    mem::{ConstPtr, MutPtr},
};

#[allow(dead_code)]
pub const LC_ALL: i32 = 0;
#[allow(dead_code)]
pub const LC_COLLATE: i32 = 1;
#[allow(dead_code)]
pub const LC_CTYPE: i32 = 2;
#[allow(dead_code)]
pub const LC_MONETARY: i32 = 3;
#[allow(dead_code)]
pub const LC_NUMERIC: i32 = 4;
#[allow(dead_code)]
pub const LC_TIME: i32 = 5;
#[allow(dead_code)]
pub const LC_MESSAGES: i32 = 6;

#[derive(Default)]
pub struct State {
    locale: std::collections::HashMap<i32, MutPtr<u8>>,
}

fn setlocale(env: &mut Environment, category: i32, locale: ConstPtr<u8>) -> MutPtr<u8> {
    if !locale.is_null() {
        let locale_cstr = env.mem.cstr_at(locale).to_owned();
        let new_locale = env.mem.alloc_and_write_cstr(locale_cstr.as_slice());
        if let Some(old_locale) = env.libc_state.clocale.locale.insert(category, new_locale) {
            env.mem.free(old_locale.cast())
        };
    } else if let Entry::Vacant(entry) = env.libc_state.clocale.locale.entry(category) {
        let default_locale = env.mem.alloc_and_write_cstr(b"C");
        entry.insert(default_locale);
    }
    env.libc_state.clocale.locale.get(&category).unwrap().cast()
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(setlocale(_, _))];
