/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! This package contains OpenGL bindings generated with the `gl_generator`
//! crate.

// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]

#[allow(warnings)]
pub mod gl21compat {
    include!(concat!(env!("OUT_DIR"), "/gl21compat.rs"));
}
#[allow(warnings)]
pub mod gles11 {
    include!(concat!(env!("OUT_DIR"), "/gles11.rs"));
}
