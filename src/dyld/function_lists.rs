//! Separate module just for the function lists, since this will probably be a
//! very long and frequently-updated list.

use crate::frameworks;
use crate::libc;

/// All the lists of functions that the linker should search through.
pub const FUNCTION_LISTS: &[super::FunctionExports] = &[
    libc::pthread::FUNCTIONS,
    libc::stdlib::FUNCTIONS,
    crate::objc::FUNCTIONS,
    frameworks::uikit::FUNCTIONS,
];
