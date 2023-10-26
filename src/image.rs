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
    pixels: PixelStore,
    dimensions: (u32, u32),
}

enum PixelStore {
    StbImage(*mut c_uchar),
    Vec(Vec<u8>),
}

impl Image {
    pub fn from_bytes(bytes: &[u8]) -> Result<Image, String> {
        let len: c_int = bytes.len().try_into().unwrap();

        let mut x: c_int = 0;
        let mut y: c_int = 0;
        let mut _channels_in_file: c_int = 0;

        // TODO: we're currently assuming this is sRGB, can we check somehow?

        let pixels = unsafe {
            // stb_image's support for CgBI images is a bit incomplete:
            // - If we don't ask it to "convert to RGB" for us, it will load
            //   CgBI PNGs in BGR and normal PNGs as RGB, with no way to
            //   distinguish them!
            // - If we don't ask it to "unpremultiply" for us, it won't do the
            //   RGB conversion.
            // So this is the only correct way to use it. :(
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

        // (Un-un-)premultiply pixels to match iPhone OS's image loading.
        {
            let len = width as usize * height as usize * 4;
            let pixels = unsafe { std::slice::from_raw_parts_mut(pixels, len) };
            let mut i = 0;
            while i < pixels.len() {
                let a = pixels[i + 3] as f32 / 255.0;
                pixels[i] = (pixels[i] as f32 * a) as u8;
                pixels[i + 1] = (pixels[i + 1] as f32 * a) as u8;
                pixels[i + 2] = (pixels[i + 2] as f32 * a) as u8;
                i += 4;
            }
        }

        Ok(Image {
            pixels: PixelStore::StbImage(pixels),
            dimensions: (width, height),
        })
    }

    /// TODO: This shouldn't really exist, it's a workaround for `CGImage`
    /// relying on this type and should be removed once it can be refactored.
    pub fn from_pixel_vec(pixels: Vec<u8>, dimensions: (u32, u32)) -> Image {
        assert!(dimensions.0 as usize * 4 * dimensions.1 as usize == pixels.len());
        Image {
            pixels: PixelStore::Vec(pixels),
            dimensions,
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }

    /// Get image data as bytes (8 bits per channel sRGB RGBA with premultiplied
    /// alpha). Rows are in top-to-bottom order.
    pub fn pixels(&self) -> &[u8] {
        match self.pixels {
            PixelStore::Vec(ref vec) => vec,
            PixelStore::StbImage(ptr) => unsafe {
                std::slice::from_raw_parts(
                    ptr,
                    self.dimensions.0 as usize * self.dimensions.1 as usize * 4,
                )
            },
        }
    }

    fn pixels_mut(&mut self) -> &mut [u8] {
        match self.pixels {
            PixelStore::Vec(ref mut vec) => vec,
            PixelStore::StbImage(ptr) => unsafe {
                std::slice::from_raw_parts_mut(
                    ptr,
                    self.dimensions.0 as usize * self.dimensions.1 as usize * 4,
                )
            },
        }
    }

    /// Get value of a pixel as linear RGBA (not sRGB!) with premultiplied
    /// alpha. 0 on the y axis is the top of the image.
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

    // TODO: Eventually this should be in Core Animation instead?
    /// Modify the image to mask it with anti-aliased rounded corners.
    pub fn round_corners(&mut self, radius: f32) {
        let (width, height) = self.dimensions();
        let right_corners_begin = width as f32 - 1.0 - radius;
        let bottom_corners_begin = height as f32 - 1.0 - radius;
        for y in 0..height {
            for x in 0..width {
                let corner_x = (radius - x as f32).max(x as f32 - right_corners_begin);
                let corner_y = (radius - y as f32).max(y as f32 - bottom_corners_begin);
                let opacity = if corner_x > 0.0 && corner_y > 0.0 {
                    let distance = (corner_x * corner_x + corner_y * corner_y).sqrt();
                    // Bad approximation of the pixel coverage of a filled arc.
                    let distance = (distance - radius).max(0.0).min(1.0);
                    let area = distance * distance;
                    1.0 - area
                } else {
                    1.0
                };
                let rgba =
                    &mut self.pixels_mut()[y as usize * width as usize * 4 + x as usize * 4..][..4];
                for channel in rgba.iter_mut() {
                    *channel = (*channel as f32 * opacity) as u8;
                }
            }
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        match self.pixels {
            PixelStore::StbImage(ptr) => unsafe { stbi_image_free(ptr.cast()) },
            PixelStore::Vec(_) => (),
        }
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
