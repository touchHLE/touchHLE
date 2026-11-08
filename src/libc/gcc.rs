/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `libgcc` compiler helper routines (division and soft-float conversions).
//!
//! Binaries compiled for older ARM targets (e.g. armv6) call these instead of
//! using hardware instructions. They behave like the corresponding ARM `udiv`
//!/`sdiv`/VFP instructions: division by zero yields 0 rather than trapping.

use crate::dyld::{export_c_func, FunctionExports};
use crate::Environment;

// 32-bit division helpers.
fn __udivsi3(_env: &mut Environment, n: u32, d: u32) -> u32 {
    if d == 0 {
        0
    } else {
        n / d
    }
}

fn __umodsi3(_env: &mut Environment, n: u32, d: u32) -> u32 {
    if d == 0 {
        0
    } else {
        n % d
    }
}

fn __divsi3(_env: &mut Environment, a: i32, b: i32) -> i32 {
    if b == 0 {
        0
    } else {
        a.wrapping_div(b)
    }
}

fn __modsi3(_env: &mut Environment, a: i32, b: i32) -> i32 {
    if b == 0 {
        0
    } else {
        a.wrapping_rem(b)
    }
}

// 64-bit division helpers (long long on 32-bit ARM).
fn __udivdi3(_env: &mut Environment, n: u64, d: u64) -> u64 {
    if d == 0 {
        0
    } else {
        n / d
    }
}

fn __umoddi3(_env: &mut Environment, n: u64, d: u64) -> u64 {
    if d == 0 {
        0
    } else {
        n % d
    }
}

fn __divdi3(_env: &mut Environment, a: i64, b: i64) -> i64 {
    if b == 0 {
        0
    } else {
        a.wrapping_div(b)
    }
}

fn __moddi3(_env: &mut Environment, a: i64, b: i64) -> i64 {
    if b == 0 {
        0
    } else {
        a.wrapping_rem(b)
    }
}

// Integer to floating-point conversions.
fn __floatdidf(_env: &mut Environment, i: i32) -> f64 {
    i as f64
}

fn __floatdisf(_env: &mut Environment, i: i32) -> f32 {
    i as f32
}

fn __floatundidf(_env: &mut Environment, u: u32) -> f64 {
    u as f64
}

fn __floatundisf(_env: &mut Environment, u: u32) -> f32 {
    u as f32
}

// Floating-point to unsigned 64-bit conversions. Rust's `as` casts saturate
// and map NaN to 0, which matches the tolerant behavior we want here.
fn __fixunsdfdi(_env: &mut Environment, d: f64) -> u64 {
    d as u64
}

fn __fixunssfdi(_env: &mut Environment, f: f32) -> u64 {
    f as u64
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(__udivsi3(_, _)),
    export_c_func!(__umodsi3(_, _)),
    export_c_func!(__divsi3(_, _)),
    export_c_func!(__modsi3(_, _)),
    export_c_func!(__udivdi3(_, _)),
    export_c_func!(__umoddi3(_, _)),
    export_c_func!(__divdi3(_, _)),
    export_c_func!(__moddi3(_, _)),
    export_c_func!(__floatdidf(_)),
    export_c_func!(__floatdisf(_)),
    export_c_func!(__floatundidf(_)),
    export_c_func!(__floatundisf(_)),
    export_c_func!(__fixunsdfdi(_)),
    export_c_func!(__fixunssfdi(_)),
];
