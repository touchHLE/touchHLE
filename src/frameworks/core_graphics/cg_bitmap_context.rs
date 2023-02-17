/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGBitmapContext.h`

use super::cg_color_space::{kCGColorSpaceGenericRGB, CGColorSpaceHostObject, CGColorSpaceRef};
use super::cg_context::{CGContextHostObject, CGContextRef, CGContextSubclass};
use super::cg_image::{
    kCGImageAlphaFirst, kCGImageAlphaLast, kCGImageAlphaNone, kCGImageAlphaNoneSkipFirst,
    kCGImageAlphaNoneSkipLast, kCGImageAlphaOnly, kCGImageAlphaPremultipliedFirst,
    kCGImageAlphaPremultipliedLast, CGImageAlphaInfo,
};
use super::{CGFloat, CGRect};
use crate::dyld::{export_c_func, FunctionExports};
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
            alpha_info: bitmap_info,
        }),
        // TODO: is this the correct default?
        rgb_fill_color: (0.0, 0.0, 0.0, 0.0),
    };
    let isa = env
        .objc
        .get_known_class("_touchHLE_CGContext", &mut env.mem);
    env.objc
        .alloc_object(isa, Box::new(host_object), &mut env.mem)
}

fn components_for_rgb(alpha_info: CGImageAlphaInfo) -> Result<GuestUSize, ()> {
    match alpha_info {
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

/// Approximate implementation of sRGB gamma encoding.
fn gamma_encode(intensity: f32) -> f32 {
    // TODO: This doesn't implement the linear section near zero.
    intensity.powf(1.0 / 2.2)
}
/// Approximate implementation of sRGB gamma decoding.
fn gamma_decode(intensity: f32) -> f32 {
    // TODO: This doesn't implement the linear section near zero.
    intensity.powf(2.2)
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
        } = objc.borrow(context);

        let pixels = get_pixels(&bitmap_info, mem);

        CGBitmapContextDrawer {
            bitmap_info,
            rgb_fill_color,
            pixels,
        }
    }

    pub fn width(&self) -> GuestUSize {
        self.bitmap_info.width
    }
    pub fn height(&self) -> GuestUSize {
        self.bitmap_info.height
    }
    /// Get the current fill color. The returned color is linear RGB, not sRGB!
    pub fn rgb_fill_color(&self) -> (CGFloat, CGFloat, CGFloat, CGFloat) {
        (
            gamma_decode(self.rgb_fill_color.0),
            gamma_decode(self.rgb_fill_color.1),
            gamma_decode(self.rgb_fill_color.2),
            gamma_decode(self.rgb_fill_color.3),
        )
    }
    /// Set the pixel at `coords` to `color`. `color` must be linear RGB, not
    /// sRGB!
    pub fn put_pixel(&mut self, coords: (i32, i32), color: (CGFloat, CGFloat, CGFloat, CGFloat)) {
        put_pixel(&self.bitmap_info, self.pixels, coords, color)
    }
}

/// Implementation of `CGContextFillRect` for `CGBitmapContext`.
pub(super) fn fill_rect(env: &mut Environment, context: CGContextRef, rect: CGRect) {
    let mut drawer = CGBitmapContextDrawer::new(&env.objc, &mut env.mem, context);

    // TODO: correct anti-aliasing
    let x_start = (rect.origin.x.round() as GuestUSize).min(0);
    let y_start = (rect.origin.y.round() as GuestUSize).min(0);
    let x_end = ((rect.origin.x + rect.size.width).round() as GuestUSize).max(drawer.width());
    let y_end = ((rect.origin.y + rect.size.height).round() as GuestUSize).max(drawer.height());

    let color = drawer.rgb_fill_color();
    for y in y_start..y_end {
        for x in x_start..x_end {
            drawer.put_pixel((x as _, y as _), color)
        }
    }
}

pub const FUNCTIONS: FunctionExports =
    &[export_c_func!(CGBitmapContextCreate(_, _, _, _, _, _, _))];
