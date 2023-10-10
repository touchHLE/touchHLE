/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Separate module just for the constant lists, since this will probably be a
//! very long and frequently-updated list.

use crate::frameworks::{core_foundation, core_graphics, foundation, media_player, opengles};
use crate::libc;

/// All the lists of constants that the linker should search through.
pub const CONSTANT_LISTS: &[super::ConstantExports] = &[
    libc::ctype::CONSTANTS,
    libc::stdio::CONSTANTS,
    core_foundation::cf_allocator::CONSTANTS,
    core_foundation::cf_run_loop::CONSTANTS,
    core_graphics::cg_affine_transform::CONSTANTS,
    core_graphics::cg_color_space::CONSTANTS,
    core_graphics::cg_geometry::CONSTANTS,
    foundation::ns_exception::CONSTANTS,
    foundation::ns_locale::CONSTANTS,
    foundation::ns_run_loop::CONSTANTS,
    media_player::movie_player::CONSTANTS,
    opengles::eagl::CONSTANTS,
];
