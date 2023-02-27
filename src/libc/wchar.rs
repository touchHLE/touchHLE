/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `wchar.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::Environment;

// See also ctype.rs and its definition of wchar_t.
#[allow(non_camel_case_types)]
type wint_t = i32;

const WEOF: wint_t = -1;

fn btowc(_env: &mut Environment, c: i32) -> wint_t {
    let c = c as u8;
    // Assuming ASCII locale, like in ctype.rs.
    if c.is_ascii() {
        c as wint_t
    } else {
        WEOF
    }
}

fn wctob(_env: &mut Environment, c: wint_t) -> i32 {
    // Assuming ASCII locale, like in ctype.rs.
    if u32::try_from(c)
        .ok()
        .and_then(char::from_u32)
        .map_or(false, |c| c.is_ascii())
    {
        c
    } else {
        WEOF
    }
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(btowc(_)), export_c_func!(wctob(_))];
