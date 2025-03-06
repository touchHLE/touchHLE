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
pub mod cg_geometry;
pub mod cg_image;

pub type CGFloat = f32;

pub type CGBlendMode = i32;
/* TODO: Make CGBlendMode type safe using enum:
   normal = 0,
   multiply = 1,
   screen = 2,
   overlay = 3,
   darken = 4,
   lighten = 5,
   colorDodge = 6,
   colorBurn = 7,
   softLight = 8,
   hardLight = 9,
   difference = 10,
   exclusion = 11,
   hue = 12,
   saturation = 13,
   color = 14,
   luminosity = 15,
   clear = 16,
   copy = 17,
   sourceIn = 18,
   sourceOut = 19,
   sourceAtop = 20,
   destinationOver = 21,
   destinationIn = 22,
   destinationOut = 23,
   destinationAtop = 24,
   xor = 25,
   plusDarker = 26,
   plusLighter = 27,
*/

pub use cg_geometry::{CGPoint, CGRect, CGSize};
