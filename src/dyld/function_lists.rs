//! Separate module just for the function lists, since this will probably be a
//! very long and frequently-updated list.

/// All the lists of functions that the linker should search through.
pub const FUNCTION_LISTS: &[super::FunctionExports] = &[crate::objc::FUNCTIONS];
