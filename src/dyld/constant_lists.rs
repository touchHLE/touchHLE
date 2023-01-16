//! Separate module just for the constant lists, since this will probably be a
//! very long and frequently-updated list.

use crate::frameworks::{core_foundation, opengles};
use crate::libc;

/// All the lists of constants that the linker should search through.
pub const CONSTANT_LISTS: &[super::ConstantExports] = &[
    libc::ctype::CONSTANTS,
    core_foundation::cf_allocator::CONSTANTS,
    core_foundation::cf_run_loop::CONSTANTS,
    opengles::eagl::CONSTANTS,
];
