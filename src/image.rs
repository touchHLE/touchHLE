/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Image decoding. Currently only supports PNG.
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

    /// Get image data as bytes (8 bits per channel RGBA)
    pub fn pixels(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self.pixels,
                self.dimensions.0 as usize * self.dimensions.1 as usize * 4,
            )
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe { stbi_image_free(self.pixels.cast()) }
    }
}
