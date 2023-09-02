/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `clocale.h`

use std::collections::hash_map::Entry;

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::mem::{ConstPtr, MutPtr};

pub type LocaleCategory = i32;
#[allow(dead_code)]
pub const LC_ALL: LocaleCategory = 0;
#[allow(dead_code)]
pub const LC_COLLATE: LocaleCategory = 1;
#[allow(dead_code)]
pub const LC_CTYPE: LocaleCategory = 2;
#[allow(dead_code)]
pub const LC_MONETARY: LocaleCategory = 3;
#[allow(dead_code)]
pub const LC_NUMERIC: LocaleCategory = 4;
#[allow(dead_code)]
pub const LC_TIME: LocaleCategory = 5;
#[allow(dead_code)]
pub const LC_MESSAGES: LocaleCategory = 6;

#[derive(Default)]
pub struct State {
    locale: std::collections::HashMap<LocaleCategory, MutPtr<u8>>,
}

fn setlocale(env: &mut Environment, category: LocaleCategory, locale: ConstPtr<u8>) -> MutPtr<u8> {
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
