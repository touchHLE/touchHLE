//! Image decoding. Currently only supports PNG.
//!
//! Implemented as a wrapper around the C library stb_image, since it supports
//! "CgBI" PNG files (an Apple proprietary extension used in iOS apps).

use std::ffi::{c_int, c_uchar, c_void};

// See build.rs, src/image/stb_image_wrapper.c and vendor/stb/stb_image.h
extern "C" {
    fn stbi_convert_iphone_png_to_rgb(flag_true_if_should_convert: c_int);
    fn stbi_set_unpremultiply_on_load(flag_true_if_should_unpremultiply: c_int);
    fn stbi_load_from_memory(
        buffer: *const c_uchar,
        len: c_int,
        x: *mut c_int,
        y: *mut c_int,
        channels_in_file: *mut c_int,
        desired_channels: c_int,
    ) -> *mut c_uchar;
    fn stbi_image_free(retval_from_stbi_load: *mut c_void);
}

pub struct Image {
    pixels: *mut c_uchar,
    dimensions: (u32, u32),
}

impl Image {
    pub fn from_bytes(bytes: &[u8]) -> Result<Image, ()> {
        let len: c_int = bytes.len().try_into().map_err(|_| ())?;

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
            return Err(());
        }

        let width: u32 = x.try_into().unwrap();
        let height: u32 = y.try_into().unwrap();

        Ok(Image {
            pixels,
            dimensions: (width, height),
        })
    }

    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Image, ()> {
        Self::from_bytes(&std::fs::read(path).map_err(|_| ())?)
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
