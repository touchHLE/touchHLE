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
pub mod cg_color_space;
pub mod cg_context;
pub mod cg_data_provider;
pub mod cg_geometry;
pub mod cg_image;

pub type CGFloat = f32;

pub use cg_geometry::{CGPoint, CGRect, CGSize};
