/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Shared utilities.

use crate::window::gles11::types::{GLenum, GLfixed, GLfloat, GLint};

/// Convert a fixed-point scalar to a floating-point scalar.
///
/// Beware: Rust's type checker won't complain if you mix up [GLfixed] with
/// [GLint], but they have very different meanings.
pub fn fixed_to_float(fixed: GLfixed) -> GLfloat {
    ((fixed as f64) / ((1 << 16) as f64)) as f32
}

/// Convert a fixed-point 4-by-4 matrix to floating-point.
pub unsafe fn matrix_fixed_to_float(m: *const GLfixed) -> [GLfloat; 16] {
    let mut matrix = [0f32; 16];
    for (i, cell) in matrix.iter_mut().enumerate() {
        *cell = fixed_to_float(*m.add(i));
    }
    matrix
}

/// Type of a parameter, used in [ParamTable].
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ParamType {
    /// `GLboolean`
    Boolean,
    /// `GLfloat`
    Float,
    /// `GLint`
    Int,
    /// Placeholder type for things like colors which are floating-point
    /// but don't have the usual conversion behavior to/from integers etc.
    /// [ParamTable] will accept it for floating-point inputs only.
    /// TODO: Remove this and add proper types for colors etc.
    FloatSpecial,
    /// Hack to achieve `#[non_exhaustive]`-like behavior within this crate,
    /// since more types might be added in future
    _NonExhaustive,
}

/// Table of parameter names, component types and component counts.
///
/// This is a helper for implementing the common pattern in OpenGL where a set
/// of parameters named by [GLenum] values can be accessed via functions with
/// suffixes like `f`, `fv`, `i`, `iv`, etc.
pub struct ParamTable(pub &'static [(GLenum, ParamType, u8)]);

impl ParamTable {
    /// Look up the component type and count for a parameter. Panics if the name
    /// is not recognized.
    pub fn get_type_info(&self, pname: GLenum) -> (ParamType, u8) {
        match self.0.iter().find(|&&(pname2, _, _)| pname == pname2) {
            Some(&(_, type_, count)) => (type_, count),
            None => panic!("Unhandled parameter name: {:#x}", pname),
        }
    }

    /// Assert that a parameter name is recognized.
    pub fn assert_known_param(&self, pname: GLenum) {
        self.get_type_info(pname);
    }

    /// Assert that a parameter name is recognized and that the parameter has a
    /// particular component count.
    pub fn assert_component_count(&self, pname: GLenum, provided_count: u8) {
        let (_type, actual_count) = self.get_type_info(pname);
        if actual_count != provided_count {
            panic!(
                "Parameter {:#x} has component count {}, {} given.",
                pname, actual_count, provided_count
            );
        }
    }

    /// Implements a fixed-point scalar (`x`) setter by calling a provided
    /// floating-point scalar (`f`) or integer scalar (`i`) setter as
    /// as appropriate.
    ///
    /// This will panic if the name is not recognized or the parameter is not
    /// a scalar.
    pub unsafe fn setx<FF, FI>(&self, setf: FF, seti: FI, pname: GLenum, param: GLfixed)
    where
        FF: FnOnce(GLfloat),
        FI: FnOnce(GLint),
    {
        let (type_, component_count) = self.get_type_info(pname);
        assert!(component_count == 1);
        // Yes, the OpenGL standard lets you mismatch types. Yes, it requires
        // an implicit conversion. Yes, it requires no scaling of fixed-point
        // values when converting to integer. :(
        match type_ {
            ParamType::Float => setf(fixed_to_float(param)),
            ParamType::FloatSpecial => todo!(),
            _ => seti(param),
        }
    }

    /// Implements a fixed-point vector (`xv`) setter by calling a provided
    /// floating-point vector (`fv`) or integer vector (`iv`) setter as
    /// as appropriate.
    ///
    /// This will panic if the name is not recognized.
    pub unsafe fn setxv<FFV, FIV>(
        &self,
        setfv: FFV,
        setiv: FIV,
        pname: GLenum,
        params: *const GLfixed,
    ) where
        FFV: FnOnce(*const GLfloat),
        FIV: FnOnce(*const GLint),
    {
        let (type_, count) = self.get_type_info(pname);
        // Yes, the OpenGL standard is like this (see above).
        match type_ {
            ParamType::Float => {
                let mut params_float = [0.0; 16]; // probably the max?
                let params_float = &mut params_float[..usize::from(count)];
                for (i, param_float) in params_float.iter_mut().enumerate() {
                    *param_float = fixed_to_float(params.add(i).read())
                }
                setfv(params_float.as_ptr())
            }
            ParamType::FloatSpecial => todo!(),
            _ => setiv(params),
        }
    }
}
