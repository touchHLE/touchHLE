/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Image decoding. Currently only supports PNG files (treated as 8-bit sRGB).
//!
//! Implemented as a wrapper around the C library stb_image, since it supports
//! "CgBI" PNG files (an Apple proprietary extension used in iPhone OS apps).
//!
//! This module also exposes decompression for Imagination Technologies' PVRTC
//! format, implementing as a wrapper around their decoder from the PowerVR
//! SDK.

use std::ffi::{c_int, c_uchar, CStr};

use touchHLE_pvrt_decompress_wrapper::*;
use touchHLE_stb_image_wrapper::*;

pub struct Image {
    len: u32,
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
            len: width * height * 4,
            pixels,
            dimensions: (width, height),
        })
    }

    pub fn len(&self) -> u32 {
        self.len
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

/// Decodes Imagination Technologies' PVRTC texture compression format to
/// RGBA (8 bits per channel).
pub fn decode_pvrtc(pvrtc_data: &[u8], is_2bit: bool, width: u32, height: u32) -> Vec<u32> {
    // This formula is from the IMG_texture_compression_pvrtc extension spec.
    let expected_size = if is_2bit {
        (width.max(16) as usize * height.max(8) as usize * 2 + 7) / 8
    } else {
        (width.max(8) as usize * height.max(8) as usize * 4 + 7) / 8
    };
    assert!(pvrtc_data.len() == expected_size);

    let rgba8_word_count = width as usize * height as usize;
    let mut rgba8_data = Vec::with_capacity(rgba8_word_count);
    unsafe {
        let consumed_size = touchHLE_decompress_pvrtc(
            pvrtc_data.as_ptr() as *const _,
            is_2bit,
            width,
            height,
            // The interface says `uint8_t *` but the source seems to work with
            // 32-bit words, so using Vec<u32> seems more appropriate.
            rgba8_data.as_mut_ptr() as *mut u8,
        );
        assert_eq!(consumed_size as usize, expected_size);
        rgba8_data.set_len(rgba8_word_count);
    };
    rgba8_data
}
