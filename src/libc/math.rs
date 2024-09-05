/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `math.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::set_errno;
use crate::mem::MutPtr;
use crate::Environment;

// The sections in this file are organized to match the C standard.

// FIXME: Many functions in this file should theoretically set errno or affect
//        the floating-point environment. We're hoping apps won't rely on that.

// Trigonometric functions

// TODO: These should also have `long double` variants, which can probably just
// alias the `double` ones.

fn abs(_env: &mut Environment, arg: i32) -> i32 {
    arg.abs()
}
fn sin(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.sin()
}
fn sinf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.sin()
}
fn cos(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.cos()
}
fn cosf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.cos()
}
fn tan(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.tan()
}
fn tanf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.tan()
}

fn asin(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.asin()
}
fn asinf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.asin()
}
fn acos(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.acos()
}
fn acosf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.acos()
}
fn atan(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.atan()
}
fn atanf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.atan()
}

fn atan2f(env: &mut Environment, arg1: f32, arg2: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg1.atan2(arg2)
}
fn atan2(env: &mut Environment, arg1: f64, arg2: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg1.atan2(arg2)
}

// Hyperbolic functions

fn sinh(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.sinh()
}
fn sinhf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.sinh()
}
fn cosh(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.cosh()
}
fn coshf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.cosh()
}
fn tanh(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.tanh()
}
fn tanhf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.tanh()
}

fn asinh(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.asinh()
}
fn asinhf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.asinh()
}
fn acosh(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.acosh()
}
fn acoshf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.acosh()
}
fn atanh(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.atanh()
}
fn atanhf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.atanh()
}

// Exponential and logarithmic functions
// TODO: implement the rest
fn log(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.ln()
}
fn logf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.ln()
}
fn log1p(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.ln_1p()
}
fn log1pf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.ln_1p()
}
fn log2(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.log2()
}
fn log2f(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.log2()
}
fn log10(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.log10()
}
fn log10f(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.log10()
}
fn exp(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.exp()
}
fn expf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.exp()
}
fn expm1(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.exp_m1()
}
fn expm1f(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.exp_m1()
}
fn exp2(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.exp2()
}
fn exp2f(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.exp2()
}

// Power functions
// TODO: implement the rest
fn pow(env: &mut Environment, arg1: f64, arg2: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg1.powf(arg2)
}
fn powf(env: &mut Environment, arg1: f32, arg2: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg1.powf(arg2)
}
fn sqrt(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.sqrt()
}
fn sqrtf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.sqrt()
}

// Nearest integer functions
// TODO: implement the rest
fn ceil(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.ceil()
}
fn ceilf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.ceil()
}
fn floor(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.floor()
}
fn floorf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.floor()
}
fn round(env: &mut Environment, arg: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.round()
}
fn roundf(env: &mut Environment, arg: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg.round()
}
fn trunc(_env: &mut Environment, arg: f64) -> f64 {
    arg.trunc()
}
fn truncf(_env: &mut Environment, arg: f32) -> f32 {
    arg.trunc()
}
fn modff(env: &mut Environment, val: f32, iptr: MutPtr<f32>) -> f32 {
    let ivalue = truncf(env, val);
    env.mem.write(iptr, ivalue);
    val - ivalue
}

// Remainder functions
// TODO: implement the rest
fn fmod(env: &mut Environment, arg1: f64, arg2: f64) -> f64 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg1 % arg2
}
fn fmodf(env: &mut Environment, arg1: f32, arg2: f32) -> f32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    arg1 % arg2
}

// Maximum, minimum and positive difference functions
// TODO: implement fdim
fn fmax(_env: &mut Environment, arg1: f64, arg2: f64) -> f64 {
    arg1.max(arg2)
}
fn fmaxf(_env: &mut Environment, arg1: f32, arg2: f32) -> f32 {
    arg1.max(arg2)
}
fn fmin(_env: &mut Environment, arg1: f64, arg2: f64) -> f64 {
    arg1.min(arg2)
}
fn fminf(_env: &mut Environment, arg1: f32, arg2: f32) -> f32 {
    arg1.min(arg2)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(abs(_)),
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
    export_c_func!(pow(_, _)),
    export_c_func!(powf(_, _)),
    export_c_func!(sqrt(_)),
    export_c_func!(sqrtf(_)),
    // Nearest integer functions
    export_c_func!(ceil(_)),
    export_c_func!(ceilf(_)),
    export_c_func!(floor(_)),
    export_c_func!(floorf(_)),
    export_c_func!(round(_)),
    export_c_func!(roundf(_)),
    export_c_func!(trunc(_)),
    export_c_func!(truncf(_)),
    export_c_func!(modff(_, _)),
    // Remainder functions
    export_c_func!(fmod(_, _)),
    export_c_func!(fmodf(_, _)),
    // Maximum, minimum and positive difference functions
    export_c_func!(fmax(_, _)),
    export_c_func!(fmaxf(_, _)),
    export_c_func!(fmin(_, _)),
    export_c_func!(fminf(_, _)),
];
