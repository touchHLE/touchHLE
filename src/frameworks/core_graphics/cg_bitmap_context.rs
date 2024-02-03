/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGBitmapContext.h`

use super::cg_affine_transform::{CGAffineTransform, CGAffineTransformIdentity};
use super::cg_color_space::{
    kCGColorSpaceGenericGray, kCGColorSpaceGenericRGB, CGColorSpaceHostObject, CGColorSpaceRef,
};
use super::cg_context::{CGContextHostObject, CGContextRef, CGContextSubclass};
use super::cg_image::{
    self, kCGBitmapAlphaInfoMask, kCGBitmapByteOrderMask, kCGImageAlphaFirst, kCGImageAlphaLast,
    kCGImageAlphaNone, kCGImageAlphaNoneSkipFirst, kCGImageAlphaNoneSkipLast, kCGImageAlphaOnly,
    kCGImageAlphaPremultipliedFirst, kCGImageAlphaPremultipliedLast, kCGImageByteOrder32Big,
    kCGImageByteOrderDefault, CGBitmapInfo, CGImageAlphaInfo, CGImageRef,
};
use super::{CGFloat, CGPoint, CGRect};
use crate::dyld::{export_c_func, FunctionExports};
use crate::image::{gamma_decode, gamma_encode, Image};
use crate::mem::{GuestUSize, Mem, MutVoidPtr};
use crate::objc::ObjC;
use crate::Environment;

#[derive(Copy, Clone)]
pub(super) struct CGBitmapContextData {
    pub(super) data: MutVoidPtr,
    pub(super) data_is_owned: bool,
    width: GuestUSize,
    height: GuestUSize,
    bits_per_component: GuestUSize,
    bytes_per_row: GuestUSize,
    color_space: &'static str,
    alpha_info: CGImageAlphaInfo,
}

pub fn CGBitmapContextCreate(
    env: &mut Environment,
    data: MutVoidPtr,
    width: GuestUSize,
    height: GuestUSize,
    bits_per_component: GuestUSize,
    bytes_per_row: GuestUSize,
    color_space: CGColorSpaceRef,
    bitmap_info: u32,
) -> CGContextRef {
    assert!(bits_per_component == 8); // TODO: support other bit depths

    let color_space = env.objc.borrow::<CGColorSpaceHostObject>(color_space).name;

    let component_count = match color_space {
        kCGColorSpaceGenericRGB => components_for_rgb(bitmap_info).unwrap(),
        kCGColorSpaceGenericGray => components_for_gray(bitmap_info).unwrap(),
        _ => unimplemented!("support other color spaces"),
    };

    let (data, data_is_owned, bytes_per_row) = if data.is_null() {
        let bytes_per_row = if bytes_per_row == 0 {
            width.checked_mul(component_count).unwrap()
        } else {
            bytes_per_row
        };
        let total_size = bytes_per_row.checked_mul(height).unwrap();
        let data = env.mem.alloc(total_size);
        (data, true, bytes_per_row)
    } else {
        assert!(bytes_per_row != 0);
        (data, false, bytes_per_row)
    };

    let host_object = CGContextHostObject {
        subclass: CGContextSubclass::CGBitmapContext(CGBitmapContextData {
            data,
            data_is_owned,
            width,
            height,
            bits_per_component,
            bytes_per_row,
            color_space,
            alpha_info: bitmap_info & kCGBitmapAlphaInfoMask,
        }),
        // TODO: is this the correct default?
        rgb_fill_color: (0.0, 0.0, 0.0, 0.0),
        transform: CGAffineTransformIdentity,
        state_stack: Vec::new(),
    };
    let isa = env
        .objc
        .get_known_class("_touchHLE_CGContext", &mut env.mem);
    env.objc
        .alloc_object(isa, Box::new(host_object), &mut env.mem)
}

fn CGBitmapContextGetData(env: &mut Environment, context: CGContextRef) -> MutVoidPtr {
    let host_obj = env.objc.borrow::<CGContextHostObject>(context);
    let CGContextSubclass::CGBitmapContext(bitmap_data) = host_obj.subclass;
    bitmap_data.data
}

pub fn CGBitmapContextGetWidth(env: &mut Environment, context: CGContextRef) -> GuestUSize {
    let host_obj = env.objc.borrow::<CGContextHostObject>(context);
    let CGContextSubclass::CGBitmapContext(bitmap_data) = host_obj.subclass;
    bitmap_data.width
}

pub fn CGBitmapContextGetHeight(env: &mut Environment, context: CGContextRef) -> GuestUSize {
    let host_obj = env.objc.borrow::<CGContextHostObject>(context);
    let CGContextSubclass::CGBitmapContext(bitmap_data) = host_obj.subclass;
    bitmap_data.height
}

pub fn CGBitmapContextCreateImage(env: &mut Environment, context: CGContextRef) -> CGImageRef {
    // TODO: Image::from_pixel_vec() should not exist, and this function should
    // support any bitmap format.
    let host_obj = env.objc.borrow::<CGContextHostObject>(context);
    let CGContextSubclass::CGBitmapContext(bitmap_data) = host_obj.subclass;
    assert!(
        bitmap_data.bits_per_component == 8
            && bitmap_data.bytes_per_row == bitmap_data.width * 4
            && bitmap_data.color_space == kCGColorSpaceGenericRGB
            && matches!(
                bitmap_data.alpha_info,
                kCGImageAlphaNoneSkipLast | kCGImageAlphaPremultipliedLast
            )
    );
    let pixels = env
        .mem
        .bytes_at(
            bitmap_data.data.cast(),
            bitmap_data.bytes_per_row * bitmap_data.height,
        )
        .to_vec();
    cg_image::from_image(
        env,
        Image::from_pixel_vec(pixels, (bitmap_data.width, bitmap_data.height)),
    )
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

fn components_for_gray(bitmap_info: CGBitmapInfo) -> Result<GuestUSize, ()> {
    let byte_order = bitmap_info & kCGBitmapByteOrderMask;
    if byte_order != kCGImageByteOrderDefault && byte_order != kCGImageByteOrder32Big {
        return Err(()); // TODO: handle other byte orders
    }

    let alpha_info = bitmap_info & kCGBitmapAlphaInfoMask;
    if (alpha_info | byte_order) != bitmap_info {
        return Err(()); // TODO: handle other cases (float)
    }
    match alpha_info & kCGBitmapAlphaInfoMask {
        kCGImageAlphaNone => Ok(1), // gray
        kCGImageAlphaPremultipliedLast
        | kCGImageAlphaPremultipliedFirst
        | kCGImageAlphaLast
        | kCGImageAlphaFirst
        | kCGImageAlphaNoneSkipLast
        | kCGImageAlphaNoneSkipFirst => Ok(2), // gray + alpha
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
    match color_space {
        kCGColorSpaceGenericRGB => components_for_rgb(alpha_info).unwrap(),
        kCGColorSpaceGenericGray => components_for_gray(alpha_info).unwrap(),
        _ => unimplemented!("support other color spaces"),
    }
}

fn get_pixels<'a>(data: &CGBitmapContextData, mem: &'a mut Mem) -> &'a mut [u8] {
    let pixel_data_size = data.height.checked_mul(data.bytes_per_row).unwrap();
    mem.bytes_at_mut(data.data.cast(), pixel_data_size)
}

fn blend_alpha(bg: f32, fg: f32) -> f32 {
    // Alpha is blended the same way in
    // premultiplied and straight representation.
    fg + bg * (1.0 - fg)
}

/// Blends two RGBA non gamma-encoded values, with straight alpha.
fn blend_straight(bg: (f32, f32, f32, f32), fg: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    if fg.3 == 0.0 {
        // If fg.3 == 0.0 we attempt to blend fully transparent color.
        bg
    } else {
        let new_a = blend_alpha(bg.3, fg.3); // Can't be 0 if fg.3 != 0
        (
            (fg.0 * fg.3 + bg.0 * bg.3 * (1.0 - fg.3)) / new_a,
            (fg.1 * fg.3 + bg.1 * bg.3 * (1.0 - fg.3)) / new_a,
            (fg.2 * fg.3 + bg.2 * bg.3 * (1.0 - fg.3)) / new_a,
            new_a,
        )
    }
}

/// Blends two RGBA non gamma-encoded values, with premultiplied alpha.
fn blend_premultiplied(bg: (f32, f32, f32, f32), fg: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
    (
        fg.0 + bg.0 * (1.0 - fg.3),
        fg.1 + bg.1 * (1.0 - fg.3),
        fg.2 + bg.2 * (1.0 - fg.3),
        blend_alpha(bg.3, fg.3),
    )
}

/// per component offsets (r, g, b, a)
fn pixel_offsets(data: &CGBitmapContextData) -> (usize, usize, usize, Option<usize>) {
    match data.color_space {
        kCGColorSpaceGenericRGB => {
            match data.alpha_info {
                kCGImageAlphaNone => (0, 1, 2, None),
                kCGImageAlphaPremultipliedLast | kCGImageAlphaLast => (0, 1, 2, Some(3)),
                kCGImageAlphaPremultipliedFirst | kCGImageAlphaFirst => (1, 2, 3, Some(0)),
                kCGImageAlphaNoneSkipLast => (0, 1, 2, None),
                kCGImageAlphaNoneSkipFirst => (1, 2, 3, None),
                kCGImageAlphaOnly => (0, 0, 0, Some(0)),
                _ => unreachable!(), // checked by bytes_per_pixel
            }
        }
        kCGColorSpaceGenericGray => {
            // TODO: this is probably isn't doing RGB to grayscale conversion
            // properly
            match data.alpha_info {
                kCGImageAlphaNone => (0, 0, 0, None),
                kCGImageAlphaPremultipliedLast | kCGImageAlphaLast => (0, 0, 0, Some(1)),
                kCGImageAlphaPremultipliedFirst | kCGImageAlphaFirst => (1, 1, 1, Some(0)),
                kCGImageAlphaNoneSkipLast => (0, 0, 0, None),
                kCGImageAlphaNoneSkipFirst => (1, 1, 1, None),
                kCGImageAlphaOnly => (0, 0, 0, Some(0)),
                _ => unreachable!(), // checked by bytes_per_pixel
            }
        }
        _ => unimplemented!(),
    }
}

/// Get gamma-decoded RGBA value.
fn get_pixel(
    data: &CGBitmapContextData,
    pixels: &mut [u8],
    first_component_idx: usize,
) -> (f32, f32, f32, f32) {
    let pixel_offset = pixel_offsets(data);
    let pixel = (
        pixels[first_component_idx + pixel_offset.0] as f32 / 255.0,
        pixels[first_component_idx + pixel_offset.1] as f32 / 255.0,
        pixels[first_component_idx + pixel_offset.2] as f32 / 255.0,
        if let Some(alpha_offest) = pixel_offset.3 {
            pixels[first_component_idx + alpha_offest] as f32 / 255.0
        } else {
            1.0
        },
    );

    (
        gamma_decode(pixel.0),
        gamma_decode(pixel.1),
        gamma_decode(pixel.2),
        pixel.3,
    )
}

fn put_pixel(
    data: &CGBitmapContextData,
    pixels: &mut [u8],
    coords: (i32, i32),
    pixel: (CGFloat, CGFloat, CGFloat, CGFloat),
    blend: bool,
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

    let bg_pixel = get_pixel(data, pixels, first_component_idx);

    // Blending like this must be done in linear RGB, so this must come before
    // gamma encoding.
    let (r, g, b, a) = if blend {
        match data.alpha_info {
            kCGImageAlphaLast | kCGImageAlphaFirst => blend_straight(bg_pixel, pixel),
            kCGImageAlphaPremultipliedLast | kCGImageAlphaPremultipliedFirst => {
                blend_premultiplied(bg_pixel, pixel)
            }
            kCGImageAlphaOnly => (pixel.0, pixel.1, pixel.2, blend_alpha(bg_pixel.3, pixel.3)),
            _ => pixel,
        }
    } else {
        pixel
    };

    // Alpha is always linear.
    let (r, g, b) = (gamma_encode(r), gamma_encode(g), gamma_encode(b));
    let pixel_offset = pixel_offsets(data);
    match data.alpha_info {
        kCGImageAlphaOnly => {
            pixels[first_component_idx] = (a * 255.0) as u8;
        }
        _ => {
            pixels[first_component_idx + pixel_offset.0] = (r * 255.0) as u8;
            pixels[first_component_idx + pixel_offset.1] = (g * 255.0) as u8;
            pixels[first_component_idx + pixel_offset.2] = (b * 255.0) as u8;
            if let Some(alpha_offset) = pixel_offset.3 {
                pixels[first_component_idx + alpha_offset] = (a * 255.0) as u8;
            }
        }
    }
}

/// Abstract interface for use by host code that wants to draw in a bitmap
/// context.
pub struct CGBitmapContextDrawer<'a> {
    bitmap_info: CGBitmapContextData,
    rgb_fill_color: (CGFloat, CGFloat, CGFloat, CGFloat),
    transform: CGAffineTransform,
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
            transform,
            ..
        } = objc.borrow(context);

        let pixels = get_pixels(&bitmap_info, mem);

        CGBitmapContextDrawer {
            bitmap_info,
            rgb_fill_color,
            transform,
            pixels,
        }
    }

    pub fn width(&self) -> GuestUSize {
        self.bitmap_info.width
    }
    pub fn height(&self) -> GuestUSize {
        self.bitmap_info.height
    }
    /// Get the current fill color. The returned color is linear RGB, not sRGB.
    /// It has premultiplied alpha if the context does.
    pub fn rgb_fill_color(&self) -> (CGFloat, CGFloat, CGFloat, CGFloat) {
        let multiply_by = match self.bitmap_info.alpha_info {
            kCGImageAlphaPremultipliedLast | kCGImageAlphaPremultipliedFirst => {
                self.rgb_fill_color.3
            }
            _ => 1.0,
        };
        // Multiplying before decoding matches the Simulator's output.
        (
            gamma_decode(self.rgb_fill_color.0 * multiply_by),
            gamma_decode(self.rgb_fill_color.1 * multiply_by),
            gamma_decode(self.rgb_fill_color.2 * multiply_by),
            self.rgb_fill_color.3, // alpha is always linear
        )
    }
    /// Set the pixel at `coords` to `color`. `color` must be linear RGB, not
    /// sRGB! Note that `coords` are absolute: you must do transformation
    /// yourself.
    pub fn put_pixel(
        &mut self,
        coords: (i32, i32),
        color: (CGFloat, CGFloat, CGFloat, CGFloat),
        blend: bool,
    ) {
        put_pixel(&self.bitmap_info, self.pixels, coords, color, blend)
    }

    /// Takes a [CGRect] and applies the current transform to it, and iterates
    /// over the transformed, clipped, absolute integer pixel co-ordinates in
    /// raster order for the target bitmap while providing floating-point
    /// co-ordinates from (0,0) to (1,1) as a reference for sampling e.g. a
    /// texture.
    pub fn iter_transformed_pixels(
        &self,
        untransformed_rect: CGRect,
    ) -> impl Iterator<Item = ((i32, i32), (f32, f32))> {
        let bounding_rect = self.transform.apply_to_rect(untransformed_rect);

        let x_start = bounding_rect.origin.x.round().max(0.0) as GuestUSize;
        let y_start = bounding_rect.origin.y.round().max(0.0) as GuestUSize;
        let x_end = (bounding_rect.origin.x + bounding_rect.size.width)
            .round()
            .min(self.width() as f32) as GuestUSize;
        let y_end = (bounding_rect.origin.y + bounding_rect.size.height)
            .round()
            .min(self.height() as f32) as GuestUSize;

        let inverse_transform = self.transform.invert();

        // TODO: Doing a matrix multiply per-pixel is not optimally efficient.
        // A scanline rasterizer would be better, though we should probably use
        // an existing library for this.
        (y_start..y_end).flat_map(move |y| {
            (x_start..x_end).flat_map(move |x| {
                let untransformed = inverse_transform.apply_to_point(CGPoint {
                    x: x as f32 + 0.5,
                    y: y as f32 + 0.5,
                });
                let x_within =
                    (untransformed.x - untransformed_rect.origin.x) / untransformed_rect.size.width;
                let y_within = (untransformed.y - untransformed_rect.origin.y)
                    / untransformed_rect.size.height;
                if !(0.0..1.0).contains(&x_within) || !(0.0..1.0).contains(&y_within) {
                    None
                } else {
                    Some(((x as i32, y as i32), (x_within, y_within)))
                }
            })
        })
    }
}

#[cfg(test)]
#[test]
fn test_iter_transformed_pixels() {
    use super::CGSize;

    fn make_context(
        width: GuestUSize,
        height: GuestUSize,
        transform: CGAffineTransform,
    ) -> CGBitmapContextDrawer<'static> {
        CGBitmapContextDrawer {
            bitmap_info: CGBitmapContextData {
                data: crate::mem::Ptr::null(),
                data_is_owned: false,
                width,
                height,
                bits_per_component: 8,
                bytes_per_row: 3 * width,
                color_space: "kCGColorSpaceGenericRGB",
                alpha_info: 0,
            },
            rgb_fill_color: (0.0, 0.0, 0.0, 0.0),
            transform,
            pixels: &mut [],
        }
    }

    let square_2x2_at_0_0 = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width: 2.0,
            height: 2.0,
        },
    };
    let square_2x2_at_2_2 = CGRect {
        origin: CGPoint { x: 2.0, y: 2.0 },
        size: CGSize {
            width: 2.0,
            height: 2.0,
        },
    };
    let square_4x4_at_0_0 = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width: 4.0,
            height: 4.0,
        },
    };

    let upright_square_2x2_at_0_0 = [
        ((0, 0), (0.25, 0.25)),
        ((1, 0), (0.75, 0.25)),
        ((0, 1), (0.25, 0.75)),
        ((1, 1), (0.75, 0.75)),
    ];
    let inverted_square_2x2_at_0_0 = [
        ((0, 0), (0.75, 0.75)),
        ((1, 0), (0.25, 0.75)),
        ((0, 1), (0.75, 0.25)),
        ((1, 1), (0.25, 0.25)),
    ];
    let corner_pixel_of_upright_square_2x2_at_1_1 = [((1, 1), (0.25, 0.25))];

    // Constructed by hand so the results are precise
    let rotation_by_180deg = CGAffineTransform {
        a: -1.0,
        c: 0.0,
        b: 0.0,
        d: -1.0,
        tx: 0.0,
        ty: 0.0,
    };

    assert!(make_context(2, 2, CGAffineTransformIdentity)
        .iter_transformed_pixels(square_2x2_at_0_0)
        .eq(upright_square_2x2_at_0_0.clone().into_iter()));
    assert!(
        make_context(2, 2, CGAffineTransform::make_translation(-2.0, -2.0))
            .iter_transformed_pixels(square_2x2_at_2_2)
            .eq(upright_square_2x2_at_0_0.clone().into_iter())
    );
    assert!(
        make_context(2, 2, CGAffineTransform::make_translation(-1.0, -1.0))
            .iter_transformed_pixels(square_2x2_at_2_2)
            .eq(corner_pixel_of_upright_square_2x2_at_1_1
                .clone()
                .into_iter())
    );
    assert!(make_context(2, 2, CGAffineTransform::make_scale(0.5, 0.5))
        .iter_transformed_pixels(square_4x4_at_0_0)
        .eq(upright_square_2x2_at_0_0.clone().into_iter()));
    assert!(make_context(2, 2, rotation_by_180deg.translate(-2.0, -2.0))
        .iter_transformed_pixels(square_2x2_at_0_0)
        .eq(inverted_square_2x2_at_0_0.clone().into_iter()));
}

/// Implementation of `CGContextFillRect` (`clear` == [false]) and
/// `CGContextClearRect` (`clear` == [true]) for `CGBitmapContext`.
pub(super) fn fill_rect(env: &mut Environment, context: CGContextRef, rect: CGRect, clear: bool) {
    let mut drawer = CGBitmapContextDrawer::new(&env.objc, &mut env.mem, context);
    let color = if clear {
        (0.0, 0.0, 0.0, 0.0)
    } else {
        drawer.rgb_fill_color()
    };
    // TODO: correct anti-aliasing
    for ((x, y), _) in drawer.iter_transformed_pixels(rect) {
        drawer.put_pixel((x, y), color, /* blend: */ !clear)
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

    //let _ = std::fs::write(
    //  format!(
    //      "image-{:?}-{:?}.data",
    //      (image as *const _ as *const ()),
    //      image.dimensions()
    //  ),
    //  image.pixels()
    //);

    //let _ = std::fs::write(
    //  format!(
    //      "bitmap-{:?}-{:?}-before.data",
    //      (image as *const _ as *const ()),
    //      (drawer.width(), drawer.height())
    //  ),
    //  &drawer.pixels
    //);

    let (image_width, image_height) = image.dimensions();

    // TODO: non-nearest-neighbour filtering? (what does CG actually do?)

    for ((x, y), (texel_x, texel_y)) in drawer.iter_transformed_pixels(rect) {
        let texel_x = (image_width as f32 * texel_x) as i32;
        // Image is in top-to-bottom order, but the bitmap is bottom-to-top
        let texel_y = (image_height as f32 * (1.0 - texel_y)) as i32;
        // FIXME: might need alpha format conversion here
        if let Some(color) = image.get_pixel((texel_x, texel_y)) {
            drawer.put_pixel((x, y), color, /* blend: */ true)
        }
    }

    //let _ = std::fs::write(
    //  format!(
    //      "bitmap-{:?}-{:?}-after.data",
    //      (image as *const _ as *const ()),
    //      (drawer.width(), drawer.height())
    //  ),
    //  &drawer.pixels
    //);
}

#[allow(rustdoc::broken_intra_doc_links)] // https://github.com/rust-lang/rust/issues/83049
/// Shortcut for [crate::frameworks::core_animation::composition]. This is a
/// workaround for not having a `&mut Environment` that should eventually be
/// removed somehow (TODO).
pub fn get_data(objc: &ObjC, context: CGContextRef) -> (GuestUSize, GuestUSize, MutVoidPtr) {
    let host_obj = objc.borrow::<CGContextHostObject>(context);
    let CGContextSubclass::CGBitmapContext(bitmap_data) = host_obj.subclass;
    (bitmap_data.width, bitmap_data.height, bitmap_data.data)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGBitmapContextCreate(_, _, _, _, _, _, _)),
    export_c_func!(CGBitmapContextCreateImage(_)),
    export_c_func!(CGBitmapContextGetData(_)),
    export_c_func!(CGBitmapContextGetWidth(_)),
    export_c_func!(CGBitmapContextGetHeight(_)),
];
