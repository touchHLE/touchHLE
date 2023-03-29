/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Shared utilities.

use crate::window::gl21compat::types::GLenum;

/// Type of a parameter, used in [ParamTable].
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ParamType {
    /// `GLfloat`
    Float,
    /// `GLint`
    Int,
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
}
