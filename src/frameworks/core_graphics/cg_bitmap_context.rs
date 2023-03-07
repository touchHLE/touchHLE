/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGBitmapContext.h`

use super::cg_color_space::{kCGColorSpaceGenericRGB, CGColorSpaceHostObject, CGColorSpaceRef};
use super::cg_context::{CGContextHostObject, CGContextRef, CGContextSubclass};
use super::cg_image::{
    self, kCGBitmapAlphaInfoMask, kCGBitmapByteOrderMask, kCGImageAlphaFirst, kCGImageAlphaLast,
    kCGImageAlphaNone, kCGImageAlphaNoneSkipFirst, kCGImageAlphaNoneSkipLast, kCGImageAlphaOnly,
    kCGImageAlphaPremultipliedFirst, kCGImageAlphaPremultipliedLast, kCGImageByteOrder32Big,
    kCGImageByteOrderDefault, CGBitmapInfo, CGImageAlphaInfo, CGImageRef,
};
use super::{CGFloat, CGRect};
use crate::dyld::{export_c_func, FunctionExports};
use crate::image::{gamma_decode, gamma_encode};
use crate::mem::{GuestUSize, Mem, MutVoidPtr};
use crate::objc::ObjC;
use crate::Environment;

#[derive(Copy, Clone)]
pub(super) struct CGBitmapContextData {
    data: MutVoidPtr,
    width: GuestUSize,
    height: GuestUSize,
    bits_per_component: GuestUSize,
    bytes_per_row: GuestUSize,
    color_space: &'static str,
    alpha_info: CGImageAlphaInfo,
}

fn CGBitmapContextCreate(
    env: &mut Environment,
    data: MutVoidPtr,
    width: GuestUSize,
    height: GuestUSize,
    bits_per_component: GuestUSize,
    bytes_per_row: GuestUSize,
    color_space: CGColorSpaceRef,
    bitmap_info: u32,
) -> CGContextRef {
    assert!(!data.is_null()); // TODO: support memory allocation
    assert!(bits_per_component == 8); // TODO: support other bit depths
    assert!(components_for_rgb(bitmap_info).is_ok());

    let color_space = env.objc.borrow::<CGColorSpaceHostObject>(color_space).name;
    // TODO: support other color spaces
    assert!(color_space == kCGColorSpaceGenericRGB);

    let host_object = CGContextHostObject {
        subclass: CGContextSubclass::CGBitmapContext(CGBitmapContextData {
            data,
            width,
            height,
            bits_per_component,
            bytes_per_row,
            color_space: kCGColorSpaceGenericRGB,
            alpha_info: bitmap_info & kCGBitmapAlphaInfoMask,
        }),
        // TODO: is this the correct default?
        rgb_fill_color: (0.0, 0.0, 0.0, 0.0),
        translation: (0.0, 0.0),
    };
    let isa = env
        .objc
        .get_known_class("_touchHLE_CGContext", &mut env.mem);
    env.objc
        .alloc_object(isa, Box::new(host_object), &mut env.mem)
}

fn components_for_rgb(bitmap_info: CGBitmapInfo) -> Result<GuestUSize, ()> {
    let byte_order = bitmap_info & kCGBitmapByteOrderMask;
    if byte_order != kCGImageByteOrderDefault && byte_order != kCGImageByteOrder32Big {
        return Err(()); // TODO: handle other byte orders
    }

    let alpha_info = bitmap_info & kCGBitmapAlphaInfoMask;
    if (alpha_info | byte_order) != bitmap_info {
        return Err(()); // TODO: handle other cases (float)
    }
    match alpha_info & kCGBitmapAlphaInfoMask {
        kCGImageAlphaNone => Ok(3), // RGB
        kCGImageAlphaPremultipliedLast
        | kCGImageAlphaPremultipliedFirst
        | kCGImageAlphaLast
        | kCGImageAlphaFirst
        | kCGImageAlphaNoneSkipLast
        | kCGImageAlphaNoneSkipFirst => Ok(4), // RGBA/ARGB/RGBX/XRGB
        kCGImageAlphaOnly => Ok(1), // A
        _ => Err(()),               // unknown values
    }
}

fn bytes_per_pixel(data: &CGBitmapContextData) -> GuestUSize {
    let &CGBitmapContextData {
        bits_per_component,
        color_space,
        alpha_info,
        ..
    } = data;
    assert!(bits_per_component == 8);
    assert!(color_space == kCGColorSpaceGenericRGB);
    components_for_rgb(alpha_info).unwrap()
}

fn get_pixels<'a>(data: &CGBitmapContextData, mem: &'a mut Mem) -> &'a mut [u8] {
    let pixel_data_size = data.height.checked_mul(data.bytes_per_row).unwrap();
    mem.bytes_at_mut(data.data.cast(), pixel_data_size)
}

fn put_pixel(
    data: &CGBitmapContextData,
    pixels: &mut [u8],
    coords: (i32, i32),
    pixel: (CGFloat, CGFloat, CGFloat, CGFloat),
) {
    let (x, y) = coords;
    if x < 0 || y < 0 {
        return;
    }
    let (x, y) = (x as GuestUSize, y as GuestUSize);
    if x >= data.width || y >= data.height {
        return;
    }

    // CG's co-ordinate system puts the origin in the bottom-left corner, but it
    // *seems* like the rows are nonetheless in top-to-bottom order?
    let y = data.height - 1 - y;

    let pixel_size = bytes_per_pixel(data);
    let first_component_idx = (y * data.bytes_per_row + x * pixel_size) as usize;

    let (r, g, b, a) = pixel;
    // Blending like this must be done in linear RGB, so this must come before
    // gamma encoding.
    let (r, g, b) = match data.alpha_info {
        kCGImageAlphaPremultipliedLast | kCGImageAlphaPremultipliedFirst => (r * a, g * a, b * a),
        _ => (r, g, b),
    };
    // Alpha is always linear.
    let (r, g, b) = (gamma_encode(r), gamma_encode(g), gamma_encode(b));
    match data.alpha_info {
        kCGImageAlphaNone => {
            pixels[first_component_idx] = (r * 255.0) as u8;
            pixels[first_component_idx + 1] = (g * 255.0) as u8;
            pixels[first_component_idx + 2] = (b * 255.0) as u8;
        }
        kCGImageAlphaPremultipliedLast | kCGImageAlphaLast => {
            pixels[first_component_idx] = (r * 255.0) as u8;
            pixels[first_component_idx + 1] = (g * 255.0) as u8;
            pixels[first_component_idx + 2] = (b * 255.0) as u8;
            pixels[first_component_idx + 3] = (a * 255.0) as u8;
        }
        kCGImageAlphaPremultipliedFirst | kCGImageAlphaFirst => {
            pixels[first_component_idx] = (a * 255.0) as u8;
            pixels[first_component_idx + 1] = (r * 255.0) as u8;
            pixels[first_component_idx + 2] = (g * 255.0) as u8;
            pixels[first_component_idx + 3] = (b * 255.0) as u8;
        }
        kCGImageAlphaNoneSkipLast => {
            pixels[first_component_idx] = (r * 255.0) as u8;
            pixels[first_component_idx + 1] = (g * 255.0) as u8;
            pixels[first_component_idx + 2] = (b * 255.0) as u8;
            // alpha component skipped
        }
        kCGImageAlphaNoneSkipFirst => {
            // alpha component skipped
            pixels[first_component_idx + 1] = (r * 255.0) as u8;
            pixels[first_component_idx + 2] = (g * 255.0) as u8;
            pixels[first_component_idx + 3] = (b * 255.0) as u8;
        }
        kCGImageAlphaOnly => {
            pixels[first_component_idx] = (a * 255.0) as u8;
        }
        _ => unreachable!(), // checked by bytes_per_pixel
    }
}

/// Abstract interface for use by host code that wants to draw in a bitmap
/// context.
pub struct CGBitmapContextDrawer<'a> {
    bitmap_info: CGBitmapContextData,
    rgb_fill_color: (CGFloat, CGFloat, CGFloat, CGFloat),
    translation: (CGFloat, CGFloat),
    pixels: &'a mut [u8],
}
impl CGBitmapContextDrawer<'_> {
    pub fn new<'a>(
        objc: &ObjC,
        mem: &'a mut Mem,
        context: CGContextRef,
    ) -> CGBitmapContextDrawer<'a> {
        let &CGContextHostObject {
            subclass: CGContextSubclass::CGBitmapContext(bitmap_info),
            rgb_fill_color,
            translation,
        } = objc.borrow(context);

        let pixels = get_pixels(&bitmap_info, mem);

        CGBitmapContextDrawer {
            bitmap_info,
            rgb_fill_color,
            translation,
            pixels,
        }
    }

    pub fn width(&self) -> GuestUSize {
        self.bitmap_info.width
    }
    pub fn height(&self) -> GuestUSize {
        self.bitmap_info.height
    }
    pub fn translation(&self) -> (CGFloat, CGFloat) {
        self.translation
    }
    /// Get the current fill color. The returned color is linear RGB, not sRGB!
    pub fn rgb_fill_color(&self) -> (CGFloat, CGFloat, CGFloat, CGFloat) {
        (
            gamma_decode(self.rgb_fill_color.0),
            gamma_decode(self.rgb_fill_color.1),
            gamma_decode(self.rgb_fill_color.2),
            self.rgb_fill_color.3, // alpha is always linear
        )
    }
    /// Set the pixel at `coords` to `color`. `color` must be linear RGB, not
    /// sRGB! Note that `coords` are absolute: you must do translation yourself.
    pub fn put_pixel(&mut self, coords: (i32, i32), color: (CGFloat, CGFloat, CGFloat, CGFloat)) {
        put_pixel(&self.bitmap_info, self.pixels, coords, color)
    }
}

/// Implementation of `CGContextFillRect` (`clear` == [false]) and
/// `CGContextClearRect` (`clear` == [true]) for `CGBitmapContext`.
pub(super) fn fill_rect(env: &mut Environment, context: CGContextRef, rect: CGRect, clear: bool) {
    let mut drawer = CGBitmapContextDrawer::new(&env.objc, &mut env.mem, context);

    // TODO: correct anti-aliasing
    let translation = drawer.translation();
    let origin = (translation.0 + rect.origin.x, translation.1 + rect.origin.y);
    let x_start = origin.0.round().max(0.0) as GuestUSize;
    let y_start = origin.1.round().max(0.0) as GuestUSize;
    let x_end = (origin.0 + rect.size.width)
        .round()
        .min(drawer.width() as f32) as GuestUSize;
    let y_end = (origin.1 + rect.size.height)
        .round()
        .min(drawer.height() as f32) as GuestUSize;

    let color = if clear {
        (0.0, 0.0, 0.0, 0.0)
    } else {
        drawer.rgb_fill_color()
    };
    for y in y_start..y_end {
        for x in x_start..x_end {
            drawer.put_pixel((x as _, y as _), color)
        }
    }
}

/// Implementation of `CGContextDrawImage` for `CGBitmapContext`.
pub(super) fn draw_image(
    env: &mut Environment,
    context: CGContextRef,
    rect: CGRect,
    image: CGImageRef,
) {
    let image = cg_image::borrow_image(&env.objc, image);

    let mut drawer = CGBitmapContextDrawer::new(&env.objc, &mut env.mem, context);

    // let _ = std::fs::write(format!("image-{:?}-{:?}.data", (image as *const _ as *const ()), image.dimensions()), image.pixels());

    // let _ = std::fs::write(format!("bitmap-{:?}-{:?}-before.data", (image as *const _ as *const ()), (drawer.width(), drawer.height())), &drawer.pixels);

    // TODO: correct anti-aliasing
    let translation = drawer.translation();
    let origin = (translation.0 + rect.origin.x, translation.1 + rect.origin.y);
    let x_start = origin.0.round() as i32;
    let y_start = origin.1.round() as i32;
    let x_end = (origin.0 + rect.size.width).round() as i32;
    let y_end = (origin.1 + rect.size.height).round() as i32;
    let dest_width = x_end - x_start;
    let dest_height = y_end - y_start;

    let (image_width, image_height) = image.dimensions();

    // TODO: non-nearest-neighbour filtering? (what does CG actually do?)
    for y in y_start..y_end {
        for x in x_start..x_end {
            // Note: this clamping needs to be done here, not above, so that
            // the image will be clipped correctly if it overhangs the canvas.
            if x < 0 || y < 0 || x as u32 >= drawer.width() || y as u32 >= drawer.height() {
                continue;
            }

            let texel_x = (0.5 + (x - x_start) as f32) / dest_width as f32;
            let texel_y = (0.5 + (y - y_start) as f32) / dest_height as f32;
            let texel_x = (image_width as f32 * texel_x) as i32;
            // Image is in top-to-bottom order, but the bitmap is bottom-to-top
            let texel_y = (image_height as f32 * (1.0 - texel_y)) as i32;
            if let Some(color) = image.get_pixel((texel_x, texel_y)) {
                drawer.put_pixel((x, y), color)
            }
        }
    }

    // let _ = std::fs::write(format!("bitmap-{:?}-{:?}-after.data", (image as *const _ as *const ()), (drawer.width(), drawer.height())), &drawer.pixels);
}

pub const FUNCTIONS: FunctionExports =
    &[export_c_func!(CGBitmapContextCreate(_, _, _, _, _, _, _))];
