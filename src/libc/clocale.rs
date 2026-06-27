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
use crate::mem::{ConstPtr, MutPtr, MutVoidPtr};

pub type LocaleCategory = i32;
pub const LC_ALL: LocaleCategory = 0;
pub const LC_COLLATE: LocaleCategory = 1;
pub const LC_CTYPE: LocaleCategory = 2;
pub const LC_MONETARY: LocaleCategory = 3;
pub const LC_NUMERIC: LocaleCategory = 4;
pub const LC_TIME: LocaleCategory = 5;
pub const LC_MESSAGES: LocaleCategory = 6;

// locale_t is an opaque pointer — we use MutVoidPtr as a handle.
pub type locale_t = MutVoidPtr;

#[derive(Default)]
pub struct State {
    locale: std::collections::HashMap<LocaleCategory, MutPtr<u8>>,
}

// MARK: - setlocale

pub fn setlocale(
    env: &mut Environment,
    category: LocaleCategory,
    locale: ConstPtr<u8>,
) -> MutPtr<u8> {
    assert!(matches!(
        category,
        LC_ALL | LC_COLLATE | LC_CTYPE | LC_MONETARY | LC_NUMERIC | LC_TIME | LC_MESSAGES
    ));

    if !locale.is_null() {
        let locale_cstr = env.mem.cstr_at(locale).to_owned();
        let locale_to_use = if locale_cstr.is_empty() {
            b"C".as_slice()
        } else {
            locale_cstr.as_slice()
        };
        let new_locale = env.mem.alloc_and_write_cstr(locale_to_use);
        if let Some(old_locale) = env.libc_state.clocale.locale.insert(category, new_locale) {
            env.mem.free(old_locale.cast());
        }
    } else if let Entry::Vacant(entry) = env.libc_state.clocale.locale.entry(category) {
        let default_locale = env.mem.alloc_and_write_cstr(b"C");
        entry.insert(default_locale);
    }

    env.libc_state.clocale.locale.get(&category).unwrap().cast()
}

// MARK: - POSIX locale_t extended API

/// newlocale — create a locale object. We always return a non-null sentinel
/// (pointer value 1) so callers can distinguish success from failure without
/// us allocating real state.
fn newlocale(
    _env: &mut Environment,
    _category_mask: i32,
    _locale: ConstPtr<u8>,
    _base: locale_t,
) -> locale_t {
    // Return a non-null sentinel — we don't implement real locale objects.
    log!("newlocale: stubbed, returning sentinel");
    MutVoidPtr::from_bits(1)
}

/// duplocale — duplicate a locale object. Return the same sentinel.
fn duplocale(_env: &mut Environment, _locale: locale_t) -> locale_t {
    MutVoidPtr::from_bits(1)
}

/// freelocale — free a locale object. No-op since we don't allocate real state.
fn freelocale(_env: &mut Environment, _locale: locale_t) {
    // No-op.
}

/// uselocale — set/get the calling thread's locale. Return the sentinel.
fn uselocale(_env: &mut Environment, _locale: locale_t) -> locale_t {
    MutVoidPtr::from_bits(1)
}

/// localeconv — return a pointer to the current locale's lconv structure.
/// We write a minimal lconv into guest memory with "C" locale values.
fn localeconv(env: &mut Environment) -> MutVoidPtr {
    // lconv is a struct of char* fields. We write a minimal version with
    // the most commonly read fields: decimal_point and thousands_sep.
    // Layout (subset we need, all char*):
    //   decimal_point, thousands_sep, grouping, ... (many more, all null/empty)
    // Rather than model the full struct, we allocate a zeroed block and write
    // the two fields apps most often read.

    // Allocate 128 bytes of zeroed guest memory (more than enough for the
    // pointer-sized fields apps typically access).
    let block_size: u32 = 128;
    let block: MutVoidPtr = env.mem.alloc(block_size).cast();

    // Zero the block.
    for i in 0..block_size {
        env.mem.write(block.cast::<u8>() + i, 0u8);
    }

    // Write decimal_point = "." at offset 0 (first field in lconv).
    let dot = env.mem.alloc_and_write_cstr(b".");
    let comma = env.mem.alloc_and_write_cstr(b"");
    // Write pointer to "." as first field (decimal_point).
    env.mem.write(block.cast::<MutPtr<u8>>(), dot);
    // Write pointer to "" as second field (thousands_sep).
    env.mem.write(block.cast::<MutPtr<u8>>() + 1u32, comma);

    log_dbg!("localeconv: returning minimal C-locale lconv");
    block
}

/// localeconv_l — locale-aware variant of localeconv. Per Apple's
/// `localeconv_l(3)` man page (iPhoneOS man pages), this behaves like
/// `localeconv()` except it uses the passed `locale_t` instead of the
/// global locale. touchHLE only models the "C" locale, so the extended
/// locale argument is ignored and we return the same minimal lconv.
fn localeconv_l(env: &mut Environment, _locale: locale_t) -> MutVoidPtr {
    localeconv(env)
}

/// nl_langinfo — return locale information item. Stub returning ASCII/UTF-8
/// strings for the items apps most commonly request.
fn nl_langinfo(env: &mut Environment, item: i32) -> ConstPtr<u8> {
    // nl_item values (subset):
    // CODESET = 0, D_T_FMT = 1, D_FMT = 2, T_FMT = 3,
    // RADIXCHAR = 65536, THOUSEP = 65537
    let s: &[u8] = match item {
        0 => b"UTF-8\0",                // CODESET
        1 => b"%a %b %e %H:%M:%S %Y\0", // D_T_FMT
        2 => b"%m/%d/%y\0",             // D_FMT
        3 => b"%H:%M:%S\0",             // T_FMT
        65536 => b".\0",                // RADIXCHAR / decimal_point
        65537 => b"\0",                 // THOUSEP / thousands_sep
        _ => b"\0",
    };
    let ptr = env.mem.alloc_and_write_cstr(&s[..s.len() - 1]); // strip the \0 we added
    ptr.cast_const()
}

/// nl_langinfo_l — locale-aware version of nl_langinfo (locale ignored).
fn nl_langinfo_l(env: &mut Environment, item: i32, _locale: locale_t) -> ConstPtr<u8> {
    nl_langinfo(env, item)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(setlocale(_, _)),
    export_c_func!(newlocale(_, _, _)),
    export_c_func!(duplocale(_)),
    export_c_func!(freelocale(_)),
    export_c_func!(uselocale(_)),
    export_c_func!(localeconv()),
    export_c_func!(localeconv_l(_)),
    export_c_func!(nl_langinfo(_)),
    export_c_func!(nl_langinfo_l(_, _)),
];
