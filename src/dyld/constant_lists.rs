//! Separate module just for the constant lists, since this will probably be a
//! very long and frequently-updated list.

use crate::frameworks;

/// All the lists of constants that the linker should search through.
pub const CONSTANT_LISTS: &[super::ConstantExports] = &[frameworks::opengles::eagl::CONSTANTS];
