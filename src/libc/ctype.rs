/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `ctype.h`

use super::wchar::wchar_t;
use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::mem::{ConstVoidPtr, Mem, MutVoidPtr, Ptr, SafeRead};
use crate::Environment;

/// Called by inlined `tolower()` on Darwin
fn __tolower(_env: &mut Environment, c: i32) -> i32 {
    if (c as u8) as i32 == c {
        (c as u8).to_ascii_lowercase().into()
    } else {
        c
    }
}
/// Called by inlined `toupper()` on Darwin
fn __toupper(_env: &mut Environment, c: i32) -> i32 {
    if (c as u8) as i32 == c {
        (c as u8).to_ascii_uppercase().into()
    } else {
        c
    }
}

#[allow(non_camel_case_types)]
type darwin_rune_t = wchar_t;

const LOOKUP_TABLE_SIZE: usize = 1 << 8;

/// Darwin inlines its implementation of the ctype functions and so this struct
/// is part of its ABI. The names have had their leading underscores removed.
#[repr(C, packed)]
struct RuneLocale {
    magic: [u8; 8],
    /// Fixed-width string naming the encoding
    encoding: [u8; 32],

    getrune: GuestFunction, // TODO
    putrune: GuestFunction, // TODO
    invalid_rune: darwin_rune_t,

    /// Bits represent type of character
    runetype: [u32; LOOKUP_TABLE_SIZE],
    map_lower: [darwin_rune_t; LOOKUP_TABLE_SIZE],
    map_upper: [darwin_rune_t; LOOKUP_TABLE_SIZE],

    variable: MutVoidPtr, // extra data, not used
    variable_len: i32,

    ncharclasses: i32,     // extra data, not used
    charclass: MutVoidPtr, // type should be pointer to RuneCharClass
}
unsafe impl SafeRead for RuneLocale {}

fn get_default_rune_locale(mem: &mut Mem) -> ConstVoidPtr {
    let mut runetype = [0u32; LOOKUP_TABLE_SIZE];
    let mut map_lower = [0 as darwin_rune_t; LOOKUP_TABLE_SIZE];
    let mut map_upper = [0 as darwin_rune_t; LOOKUP_TABLE_SIZE];

    for idx in 0..LOOKUP_TABLE_SIZE {
        let c: u8 = idx.try_into().unwrap();

        let as_lower = c.to_ascii_lowercase();
        let as_upper = c.to_ascii_lowercase();

        let mut as_runetype = 0u32;
        if c.is_ascii_alphabetic() {
            as_runetype |= 0x100;
        }
        if c.is_ascii_control() {
            as_runetype |= 0x200;
        }
        if c.is_ascii_digit() {
            as_runetype |= 0x400;
        }
        if c.is_ascii_graphic() {
            as_runetype |= 0x800;
        }
        if c.is_ascii_lowercase() {
            as_runetype |= 0x1000;
        }
        if c.is_ascii_punctuation() {
            as_runetype |= 0x2000;
        }
        // Rust's definition excludes vertical tab
        if c.is_ascii_whitespace() || c == b'\x0b' {
            as_runetype |= 0x4000;
        }
        if c.is_ascii_uppercase() {
            as_runetype |= 0x8000;
        }
        if c.is_ascii_hexdigit() {
            as_runetype |= 0x10000;
        }
        // isblank()
        if c == b' ' || c == b'\t' {
            as_runetype |= 0x20000;
        }
        // isprint()
        if c.is_ascii_graphic() || c == b' ' {
            as_runetype |= 0x40000;
        }
        // TODO: There are some other flags: "ideogram", "special", "phonogram",
        // and a character "width" between 0 and 4. These aren't standard C and
        // aren't implemented here.

        runetype[idx] = as_runetype;
        map_lower[idx] = as_lower.into();
        map_upper[idx] = as_upper.into();
    }

    let mut encoding = [0u8; 32];
    encoding[0..4].copy_from_slice(b"NONE"); // this is the real value!

    mem.alloc_and_write(RuneLocale {
        magic: *b"RuneMagA",
        encoding,

        getrune: GuestFunction::from_addr_with_thumb_bit(0), // TODO
        putrune: GuestFunction::from_addr_with_thumb_bit(0), // TODO
        invalid_rune: -1,                                    // probably not correct

        runetype,
        map_lower,
        map_upper,

        variable: Ptr::null(),
        variable_len: 0,

        ncharclasses: 0,
        charclass: Ptr::null(),
    })
    .cast()
    .cast_const()
}

pub const CONSTANTS: ConstantExports = &[(
    "__DefaultRuneLocale",
    HostConstant::Custom(get_default_rune_locale),
)];

pub const FUNCTIONS: FunctionExports =
    &[export_c_func!(__tolower(_)), export_c_func!(__toupper(_))];
