//! `math.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::Environment;

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
];
