/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for `ImageIO.framework/ImageIO`.
//!
//! ImageIO provides reading and writing of image files (PNG, JPEG, etc.)
//! via `CGImageSource` and `CGImageDestination`. touchHLE already handles
//! most image decoding through CoreGraphics and stb_image.
//!
//! This stub satisfies the Mach-O dependency so the "missing dylib"
//! warning is suppressed. Real ImageIO pipeline classes can be added
//! here as needed.

use crate::dyld::FunctionExports;

pub const FUNCTIONS: FunctionExports = &[];

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/System/Library/Frameworks/ImageIO.framework/ImageIO",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[],
    function_exports: &[FUNCTIONS],
};
