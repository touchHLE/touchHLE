/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `math.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::Environment;

// The sections in this file are organized to match the C standard.

// Trigonometric functions

// FIXME: These should theoretically set errno, though it's unlikely apps
// actually check it.
// TODO: These should also have `long double` variants, which can probably just
// alias the `double` ones.

fn sin(_env: &mut Environment, arg: f64) -> f64 {
    arg.sin()
}
fn sinf(_env: &mut Environment, arg: f32) -> f32 {
    arg.sin()
}
fn cos(_env: &mut Environment, arg: f64) -> f64 {
    arg.cos()
}
fn cosf(_env: &mut Environment, arg: f32) -> f32 {
    arg.cos()
}
fn tan(_env: &mut Environment, arg: f64) -> f64 {
    arg.tan()
}
fn tanf(_env: &mut Environment, arg: f32) -> f32 {
    arg.tan()
}

fn asin(_env: &mut Environment, arg: f64) -> f64 {
    arg.asin()
}
fn asinf(_env: &mut Environment, arg: f32) -> f32 {
    arg.asin()
}
fn acos(_env: &mut Environment, arg: f64) -> f64 {
    arg.acos()
}
fn acosf(_env: &mut Environment, arg: f32) -> f32 {
    arg.acos()
}
fn atan(_env: &mut Environment, arg: f64) -> f64 {
    arg.atan()
}
fn atanf(_env: &mut Environment, arg: f32) -> f32 {
    arg.atan()
}

fn atan2f(_env: &mut Environment, arg1: f32, arg2: f32) -> f32 {
    arg1.atan2(arg2)
}
fn atan2(_env: &mut Environment, arg1: f64, arg2: f64) -> f64 {
    arg1.atan2(arg2)
}

// Hyperbolic functions

fn sinh(_env: &mut Environment, arg: f64) -> f64 {
    arg.sinh()
}
fn sinhf(_env: &mut Environment, arg: f32) -> f32 {
    arg.sinh()
}
fn cosh(_env: &mut Environment, arg: f64) -> f64 {
    arg.cosh()
}
fn coshf(_env: &mut Environment, arg: f32) -> f32 {
    arg.cosh()
}
fn tanh(_env: &mut Environment, arg: f64) -> f64 {
    arg.tanh()
}
fn tanhf(_env: &mut Environment, arg: f32) -> f32 {
    arg.tanh()
}

fn asinh(_env: &mut Environment, arg: f64) -> f64 {
    arg.asinh()
}
fn asinhf(_env: &mut Environment, arg: f32) -> f32 {
    arg.asinh()
}
fn acosh(_env: &mut Environment, arg: f64) -> f64 {
    arg.acosh()
}
fn acoshf(_env: &mut Environment, arg: f32) -> f32 {
    arg.acosh()
}
fn atanh(_env: &mut Environment, arg: f64) -> f64 {
    arg.atanh()
}
fn atanhf(_env: &mut Environment, arg: f32) -> f32 {
    arg.atanh()
}

// Exponential and logarithmic functions
// FIXME: These should theoretically set errno, though it's unlikely apps
// actually check it.
// TODO: implement the rest
fn log(_env: &mut Environment, arg: f64) -> f64 {
    arg.ln()
}
fn logf(_env: &mut Environment, arg: f32) -> f32 {
    arg.ln()
}
fn log1p(_env: &mut Environment, arg: f64) -> f64 {
    arg.ln_1p()
}
fn log1pf(_env: &mut Environment, arg: f32) -> f32 {
    arg.ln_1p()
}
fn log2(_env: &mut Environment, arg: f64) -> f64 {
    arg.log2()
}
fn log2f(_env: &mut Environment, arg: f32) -> f32 {
    arg.log2()
}
fn log10(_env: &mut Environment, arg: f64) -> f64 {
    arg.log10()
}
fn log10f(_env: &mut Environment, arg: f32) -> f32 {
    arg.log10()
}
fn exp(_env: &mut Environment, arg: f64) -> f64 {
    arg.exp()
}
fn expf(_env: &mut Environment, arg: f32) -> f32 {
    arg.exp()
}
fn expm1(_env: &mut Environment, arg: f64) -> f64 {
    arg.exp_m1()
}
fn expm1f(_env: &mut Environment, arg: f32) -> f32 {
    arg.exp_m1()
}
fn exp2(_env: &mut Environment, arg: f64) -> f64 {
    arg.exp2()
}
fn exp2f(_env: &mut Environment, arg: f32) -> f32 {
    arg.exp2()
}

// Power functions
// FIXME: These should theoretically set errno, though it's unlikely apps
// actually check it.
// TODO: implement the rest
fn sqrt(_env: &mut Environment, arg: f64) -> f64 {
    arg.sqrt()
}
fn sqrtf(_env: &mut Environment, arg: f32) -> f32 {
    arg.sqrt()
}

pub const FUNCTIONS: FunctionExports = &[
    // Trigonometric functions
    export_c_func!(sin(_)),
    export_c_func!(sinf(_)),
    export_c_func!(cos(_)),
    export_c_func!(cosf(_)),
    export_c_func!(tan(_)),
    export_c_func!(tanf(_)),
    export_c_func!(asin(_)),
    export_c_func!(asinf(_)),
    export_c_func!(acos(_)),
    export_c_func!(acosf(_)),
    export_c_func!(atan(_)),
    export_c_func!(atanf(_)),
    export_c_func!(atan2(_, _)),
    export_c_func!(atan2f(_, _)),
    // Hyperbolic functions
    export_c_func!(sinh(_)),
    export_c_func!(sinhf(_)),
    export_c_func!(cosh(_)),
    export_c_func!(coshf(_)),
    export_c_func!(tanh(_)),
    export_c_func!(tanhf(_)),
    export_c_func!(asinh(_)),
    export_c_func!(asinhf(_)),
    export_c_func!(acosh(_)),
    export_c_func!(acoshf(_)),
    export_c_func!(atanh(_)),
    export_c_func!(atanhf(_)),
    // Exponential and logarithmic functions
    export_c_func!(log(_)),
    export_c_func!(logf(_)),
    export_c_func!(log1p(_)),
    export_c_func!(log1pf(_)),
    export_c_func!(log2(_)),
    export_c_func!(log2f(_)),
    export_c_func!(log10(_)),
    export_c_func!(log10f(_)),
    export_c_func!(exp(_)),
    export_c_func!(expf(_)),
    export_c_func!(expm1(_)),
    export_c_func!(expm1f(_)),
    export_c_func!(exp2(_)),
    export_c_func!(exp2f(_)),
    // Power functions
    export_c_func!(sqrt(_)),
    export_c_func!(sqrtf(_)),
];
