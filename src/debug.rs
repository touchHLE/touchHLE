/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Debugging utility functions. These are "dead code" that you can use in
//! debug hacks.
#![allow(dead_code)]

use std::fs::File;

/// Dump RGB8 pixel data to a file in PPM format.
pub fn write_ppm(path: &str, width: u32, height: u32, pixels: &[u8]) {
    use std::io::Write;

    let mut file = File::create(path).unwrap();
    writeln!(file, "P6 {} {} 255", width, height).unwrap();
    file.write_all(pixels).unwrap();
}

/// Convert RGBA8 pixel data to RGB8 pixel data by discarding alpha component.
/// Useful with [write_ppm] for example.
pub fn rgba8_to_rgb8(pixels: &[u8]) -> Vec<u8> {
    assert!(pixels.len() % 4 == 0);
    let mut res = Vec::with_capacity((pixels.len() / 4) * 3);
    for rgba in pixels.chunks(4) {
        res.extend_from_slice(&rgba[..3]);
    }
    res
}

/// Dump a region of the current OpenGL ES framebuffer to a file.
pub fn dump_framebuffer(
    path: &str,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    gles: &mut dyn crate::gles::GLES,
) {
    let mut rgba8_pixels = Vec::<u8>::with_capacity(width as usize * height as usize * 4);
    // 0x7F grey chosen to make missing data obvious: it's more likely to
    // contrast with typical backgrounds like black or white
    rgba8_pixels.resize(rgba8_pixels.capacity(), 0x7F);
    unsafe {
        gles.ReadPixels(
            x.try_into().unwrap(),
            y.try_into().unwrap(),
            width.try_into().unwrap(),
            height.try_into().unwrap(),
            crate::gles::gles11_raw::RGBA,
            crate::gles::gles11_raw::UNSIGNED_BYTE,
            rgba8_pixels.as_mut_ptr() as *mut _,
        );
        rgba8_pixels.set_len(rgba8_pixels.capacity());
    }
    write_ppm(path, width, height, &rgba8_to_rgb8(&rgba8_pixels));
}
