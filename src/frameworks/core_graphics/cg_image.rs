/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGImage.h`

use super::cg_color_space::{kCGColorSpaceGenericRGB, CGColorSpaceCreateWithName, CGColorSpaceRef};
use super::cg_data_provider::{self, CGDataProviderRef};
use super::CGFloat;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::foundation::ns_string;
use crate::image::Image;
use crate::mem::{ConstPtr, GuestUSize};
use crate::objc::{autorelease, nil, objc_classes, ClassExports, HostObject, ObjC};
use crate::Environment;

pub type CGImageAlphaInfo = u32;
pub const kCGImageAlphaNone: CGImageAlphaInfo = 0;
pub const kCGImageAlphaPremultipliedLast: CGImageAlphaInfo = 1;
pub const kCGImageAlphaPremultipliedFirst: CGImageAlphaInfo = 2;
pub const kCGImageAlphaLast: CGImageAlphaInfo = 3;
pub const kCGImageAlphaFirst: CGImageAlphaInfo = 4;
pub const kCGImageAlphaNoneSkipLast: CGImageAlphaInfo = 5;
pub const kCGImageAlphaNoneSkipFirst: CGImageAlphaInfo = 6;
pub const kCGImageAlphaOnly: CGImageAlphaInfo = 7;

pub type CGImageByteOrderInfo = u32;
pub const kCGImageByteOrderMask: CGImageByteOrderInfo = 0x7000;
pub const kCGImageByteOrderDefault: CGImageByteOrderInfo = 0 << 12;
#[allow(dead_code)]
pub const kCGImageByteOrder16Little: CGImageByteOrderInfo = 1 << 12;
#[allow(dead_code)]
pub const kCGImageByteOrder32Little: CGImageByteOrderInfo = 2 << 12;
#[allow(dead_code)]
pub const kCGImageByteOrder16Big: CGImageByteOrderInfo = 3 << 12;
pub const kCGImageByteOrder32Big: CGImageByteOrderInfo = 4 << 12;

pub type CGBitmapInfo = u32;
pub const kCGBitmapAlphaInfoMask: CGBitmapInfo = 0x1F; // huh, it's not 0x7?
pub const kCGBitmapByteOrderMask: CGBitmapInfo = kCGImageByteOrderMask;
// TODO: other stuff in this enum (for now, always assert the rest is 0)

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// CGImage seems to be a CFType-based type, but in our implementation those
// are just Objective-C types, so we need a class for it, but its name is not
// visible anywhere.
@implementation _touchHLE_CGImage: NSObject
@end

};

struct CGImageHostObject {
    image: Image,
}
impl HostObject for CGImageHostObject {}

pub type CGImageRef = CFTypeRef;
pub fn CGImageRelease(env: &mut Environment, c: CGImageRef) {
    if !c.is_null() {
        CFRelease(env, c);
    }
}
pub fn CGImageRetain(env: &mut Environment, c: CGImageRef) -> CGImageRef {
    if !c.is_null() {
        CFRetain(env, c)
    } else {
        c
    }
}

/// Shortcut for use by `UIImage`: directly construct a `CGImage` instance from
/// an [Image] instance.
pub fn from_image(env: &mut Environment, image: Image) -> CGImageRef {
    let host_obj = Box::new(CGImageHostObject { image });
    let class = env.objc.get_known_class("_touchHLE_CGImage", &mut env.mem);
    env.objc.alloc_object(class, host_obj, &mut env.mem)
}

/// Shortcut for use by `CGBitmapContext` etc: borrow the [Image] from a
/// `CGImage` instance.
pub fn borrow_image(objc: &ObjC, image: CGImageRef) -> &Image {
    &objc.borrow::<CGImageHostObject>(image).image
}

/// Shortcut used by the app picker, counterpart to [borrow_image].
/// FIXME: This should not exist!
pub fn borrow_image_mut(objc: &mut ObjC, image: CGImageRef) -> &mut Image {
    &mut objc.borrow_mut::<CGImageHostObject>(image).image
}

// TODO: More create methods.

fn CGImageCreateWithPNGDataProvider(
    env: &mut Environment,
    source: CGDataProviderRef,
    decode: ConstPtr<CGFloat>,
    _should_interpolate: bool, // TODO
    _intent: i32,              // TODO (should be CGColorRenderingIntent)
) -> CGImageRef {
    assert!(decode.is_null()); // TODO

    let bytes = cg_data_provider::borrow_bytes(env, source);
    let Ok(image) = Image::from_bytes(bytes) else {
        // Docs don't say what happens on failure, but this would make sense.
        return nil;
    };

    from_image(env, image)
}

fn CGImageGetAlphaInfo(_env: &mut Environment, _image: CGImageRef) -> CGImageAlphaInfo {
    // our Image type always returns premultiplied RGBA
    // (the premultiplied part must match what the real UIImage does, but
    // considering CgBI's design, maybe the order doesn't?)
    kCGImageAlphaPremultipliedLast
}

fn CGImageGetColorSpace(env: &mut Environment, _image: CGImageRef) -> CGColorSpaceRef {
    // Caller must release
    // FIXME: what if a loaded image is not sRGB?

    let srgb_name = ns_string::get_static_str(env, kCGColorSpaceGenericRGB);
    CGColorSpaceCreateWithName(env, srgb_name)
}

fn CGImageGetWidth(env: &mut Environment, image: CGImageRef) -> GuestUSize {
    let (width, _height) = env
        .objc
        .borrow::<CGImageHostObject>(image)
        .image
        .dimensions();
    width
}
fn CGImageGetHeight(env: &mut Environment, image: CGImageRef) -> GuestUSize {
    let (_width, height) = env
        .objc
        .borrow::<CGImageHostObject>(image)
        .image
        .dimensions();
    height
}
fn CGImageGetBitsPerPixel(_env: &mut Environment, _image: CGImageRef) -> GuestUSize {
    32
}

fn CGImageGetDataProvider(env: &mut Environment, image: CGImageRef) -> CGDataProviderRef {
    // CGImageGetDataProvider() seems to be intended to return the underlying
    // data provider that is retained by the CGImage. That's not how CGImage is
    // implemented here though, so instead we make a data provider that
    // retains the CGImage: exactly the opposite approach!
    let cg_data_provider = cg_data_provider::from_cg_image(env, image);
    // CGImageGetDataProvider() isn't meant to return a new object, so the
    // caller won't free this. The CGImage can't retain the CGDataProvider
    // without causing a cycle, so let's autorelease it instead.
    autorelease(env, cg_data_provider)
}

fn CGImageGetBitsPerComponent(_: &mut Environment, _: CGImageRef) -> GuestUSize {
    8 // Fix this when we support anything else
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGImageRelease(_)),
    export_c_func!(CGImageRetain(_)),
    export_c_func!(CGImageCreateWithPNGDataProvider(_, _, _, _)),
    export_c_func!(CGImageGetAlphaInfo(_)),
    export_c_func!(CGImageGetColorSpace(_)),
    export_c_func!(CGImageGetWidth(_)),
    export_c_func!(CGImageGetHeight(_)),
    export_c_func!(CGImageGetBitsPerPixel(_)),
    export_c_func!(CGImageGetDataProvider(_)),
    export_c_func!(CGImageGetBitsPerComponent(_)),
];
