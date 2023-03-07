/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Image decoding. Currently only supports PNG files (treated as 8-bit sRGB).
//!
//! Implemented as a wrapper around the C library stb_image, since it supports
//! "CgBI" PNG files (an Apple proprietary extension used in iPhone OS apps).

use std::ffi::{c_int, c_uchar, CStr};

use touchHLE_stb_image_wrapper::*;

pub struct Image {
    pixels: *mut c_uchar,
    dimensions: (u32, u32),
}

impl Image {
    pub fn from_bytes(bytes: &[u8]) -> Result<Image, String> {
        let len: c_int = bytes.len().try_into().unwrap();

        let mut x: c_int = 0;
        let mut y: c_int = 0;
        let mut _channels_in_file: c_int = 0;

        // TODO: we're currently assuming this is sRGB, can we check somehow?

        let pixels = unsafe {
            stbi_convert_iphone_png_to_rgb(1);
            stbi_set_unpremultiply_on_load(1);
            stbi_load_from_memory(
                bytes.as_ptr(),
                len,
                &mut x,
                &mut y,
                &mut _channels_in_file,
                4,
            )
        };
        if pixels.is_null() {
            let reason = unsafe { CStr::from_ptr(stbi_failure_reason()) };
            return Err(reason.to_str().unwrap().to_string());
        }

        let width: u32 = x.try_into().unwrap();
        let height: u32 = y.try_into().unwrap();

        Ok(Image {
            pixels,
            dimensions: (width, height),
        })
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }

    /// Get image data as bytes (8 bits per channel sRGB RGBA). Rows are in
    /// top-to-bottom order.
    pub fn pixels(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.pixels,
                self.dimensions.0 as usize * self.dimensions.1 as usize * 4,
            )
        }
    }

    /// Get value of a pixel as linear RGBA (not sRGB!). 0 on the y axis is the
    /// top of the image.
    ///
    /// Returns [None] if `at` is out-of-bounds.
    pub fn get_pixel(&self, at: (i32, i32)) -> Option<(f32, f32, f32, f32)> {
        let (x, y) = at;
        let (x_usize, y_usize) = (x as usize, y as usize);
        let (width, height) = self.dimensions;
        let (width, height) = (width as usize, height as usize);
        if x >= 0 && x_usize < width && y >= 0 && y_usize < height {
            let rgba = &self.pixels()[y_usize * width * 4 + x_usize * 4..][..4];
            let [r, g, b, a]: [u8; 4] = rgba.try_into().unwrap();
            Some((
                gamma_decode(r as f32 / 255.0),
                gamma_decode(g as f32 / 255.0),
                gamma_decode(b as f32 / 255.0),
                a as f32 / 255.0, // alpha is linear
            ))
        } else {
            None
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe { stbi_image_free(self.pixels.cast()) }
    }
}

/// Approximate implementation of sRGB gamma encoding.
pub fn gamma_encode(intensity: f32) -> f32 {
    // TODO: This doesn't implement the linear section near zero.
    intensity.powf(1.0 / 2.2)
}
/// Approximate implementation of sRGB gamma decoding.
pub fn gamma_decode(intensity: f32) -> f32 {
    // TODO: This doesn't implement the linear section near zero.
    intensity.powf(2.2)
}
