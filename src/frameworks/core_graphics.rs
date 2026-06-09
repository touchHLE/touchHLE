/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Core Graphics framework.
//!
//! Useful resources:
//! - Apple's [Quartz 2D Programming Guide](https://developer.apple.com/library/archive/documentation/GraphicsImaging/Conceptual/drawingwithquartz2d/Introduction/Introduction.html)

pub mod cg_affine_transform;
pub mod cg_bitmap_context;
pub mod cg_color;
pub mod cg_color_space;
pub mod cg_context;
pub mod cg_data_provider;
pub mod cg_font;
pub mod cg_geometry;
pub mod cg_image;

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics",
    aliases: &[],
    class_exports: &[
        cg_data_provider::CLASSES,
        cg_color::CLASSES,
        cg_color_space::CLASSES,
        cg_context::CLASSES,
        cg_font::CLASSES,
        cg_image::CLASSES,
    ],
    constant_exports: &[
        cg_affine_transform::CONSTANTS,
        cg_color_space::CONSTANTS,
        cg_geometry::CONSTANTS,
    ],
    function_exports: &[
        cg_affine_transform::FUNCTIONS,
        cg_bitmap_context::FUNCTIONS,
        cg_color::FUNCTIONS,
        cg_color_space::FUNCTIONS,
        cg_context::FUNCTIONS,
        cg_data_provider::FUNCTIONS,
        cg_font::FUNCTIONS,
        cg_geometry::FUNCTIONS,
        cg_image::FUNCTIONS,
    ],
};

pub type CGFloat = f32;

pub use cg_geometry::{CGPoint, CGRect, CGSize};
