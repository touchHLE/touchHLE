/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGColor.h`

use std::ops::{Add, Mul, Sub};

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::core_graphics::cg_color_space::{
    kCGColorSpaceGenericCMYK, kCGColorSpaceGenericGray, kCGColorSpaceGenericRGB,
    CGColorSpaceHostObject, CGColorSpaceRef,
};
use crate::frameworks::core_graphics::CGFloat;
use crate::mem::{ConstPtr, MutPtr};
use crate::objc::{objc_classes, ClassExports, HostObject, ObjC};
use crate::Environment;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// CGColor seems to be a CFType-based type, but in our implementation
// those are just Objective-C types, so we need a class for it, but its name is
// not visible anywhere.
@implementation _touchHLE_CGColor: NSObject
@end

};

#[derive(Copy, Clone, Default)]
pub struct CGColorHostObject {
    pub color_space_name: &'static str,
    // this assumes usage of CGColorSpaceGenericRGB
    // TODO: support other color spaces
    pub r: CGFloat,
    pub g: CGFloat,
    pub b: CGFloat,
    pub a: CGFloat,
}
impl HostObject for CGColorHostObject {}
// Implemented to aid animation code.
// Theres are the operations needed for the interpolation.
impl Mul<f32> for CGColorHostObject {
    type Output = CGColorHostObject;

    fn mul(self, rhs: f32) -> Self::Output {
        CGColorHostObject {
            color_space_name: self.color_space_name,
            r: self.r * rhs,
            g: self.g * rhs,
            b: self.b * rhs,
            a: self.a * rhs,
        }
    }
}
impl Add<CGColorHostObject> for CGColorHostObject {
    type Output = CGColorHostObject;

    fn add(self, rhs: CGColorHostObject) -> Self::Output {
        CGColorHostObject {
            color_space_name: self.color_space_name,
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
            a: self.a + rhs.a,
        }
    }
}
impl Sub<CGColorHostObject> for CGColorHostObject {
    type Output = CGColorHostObject;

    fn sub(self, rhs: CGColorHostObject) -> Self::Output {
        CGColorHostObject {
            color_space_name: self.color_space_name,
            r: self.r - rhs.r,
            g: self.g - rhs.g,
            b: self.b - rhs.b,
            a: self.a - rhs.a,
        }
    }
}

pub type CGColorRef = CFTypeRef;
pub fn CGColorRelease(env: &mut Environment, c: CGColorRef) {
    if !c.is_null() {
        CFRelease(env, c);
    }
}
pub fn CGColorRetain(env: &mut Environment, c: CGColorRef) -> CGColorRef {
    if !c.is_null() {
        CFRetain(env, c)
    } else {
        c
    }
}

fn CGColorCreate(
    env: &mut Environment,
    space: CGColorSpaceRef,
    components: MutPtr<CGFloat>,
) -> CGColorRef {
    if space.is_null() || components.is_null() {
        return crate::objc::nil;
    }
    // Per Apple's CGColorCreate docs, `components` holds n+1 values: the n
    // colour components for the given colour space, followed by the alpha
    // component. The number of colour components depends on the colour space
    // model, so we must not assume RGB here (e.g. LetterSchool creates colours
    // in the Generic Gray space, which has a single grey component + alpha).
    let color_space = env.objc.borrow::<CGColorSpaceHostObject>(space).name;
    match color_space {
        kCGColorSpaceGenericGray => {
            // Monochrome: 1 grey component + alpha. Expand grey to RGB.
            let gray = env.mem.read(components);
            let a = env.mem.read(components + 1);
            from_rgba(env, (gray, gray, gray, a))
        }
        kCGColorSpaceGenericCMYK => {
            // CMYK: 4 components + alpha. Convert CMYK → RGB.
            let cyan = env.mem.read(components);
            let magenta = env.mem.read(components + 1);
            let yellow = env.mem.read(components + 2);
            let black = env.mem.read(components + 3);
            let a = env.mem.read(components + 4);
            let r = (1.0 - cyan) * (1.0 - black);
            let g = (1.0 - magenta) * (1.0 - black);
            let b = (1.0 - yellow) * (1.0 - black);
            from_rgba(env, (r, g, b, a))
        }
        // GenericRGB (and anything else, which CGColorSpaceCreateWithName
        // canonicalizes to one of our three models): 3 components + alpha.
        _ => {
            let r = env.mem.read(components);
            let g = env.mem.read(components + 1);
            let b = env.mem.read(components + 2);
            let a = env.mem.read(components + 3);
            from_rgba(env, (r, g, b, a))
        }
    }
}

fn CGColorCreateGenericRGB(
    env: &mut Environment,
    r: CGFloat,
    g: CGFloat,
    b: CGFloat,
    a: CGFloat,
) -> CGColorRef {
    from_rgba(env, (r, g, b, a))
}

fn CGColorEqualToColor(env: &mut Environment, a: CGColorRef, b: CGColorRef) -> bool {
    to_rgba(&env.objc, a) == to_rgba(&env.objc, b)
}

fn CGColorCreateGenericGray(env: &mut Environment, gray: CGFloat, alpha: CGFloat) -> CGColorRef {
    // Expand grey to RGB.
    from_rgba(env, (gray, gray, gray, alpha))
}

fn CGColorCreateGenericCMYK(
    env: &mut Environment,
    cyan: CGFloat,
    magenta: CGFloat,
    yellow: CGFloat,
    black: CGFloat,
    alpha: CGFloat,
) -> CGColorRef {
    // Convert CMYK → RGB.
    let r = (1.0 - cyan) * (1.0 - black);
    let g = (1.0 - magenta) * (1.0 - black);
    let b = (1.0 - yellow) * (1.0 - black);
    from_rgba(env, (r, g, b, alpha))
}

fn CGColorCreateCopy(env: &mut Environment, color: CGColorRef) -> CGColorRef {
    if color.is_null() {
        return color;
    }
    let &CGColorHostObject { r, g, b, a, .. } = env.objc.borrow(color);
    from_rgba(env, (r, g, b, a))
}

fn CGColorCreateCopyWithAlpha(
    env: &mut Environment,
    color: CGColorRef,
    alpha: CGFloat,
) -> CGColorRef {
    if color.is_null() {
        return color;
    }
    let &CGColorHostObject { r, g, b, .. } = env.objc.borrow(color);
    from_rgba(env, (r, g, b, alpha))
}

// MARK: - Accessors

fn CGColorGetAlpha(env: &mut Environment, color: CGColorRef) -> CGFloat {
    if color.is_null() {
        return 0.0;
    }
    env.objc.borrow::<CGColorHostObject>(color).a
}

fn CGColorGetColorSpace(env: &mut Environment, color: CGColorRef) -> CGColorSpaceRef {
    if color.is_null() {
        return crate::objc::nil;
    }
    // Return a retained colour space matching the stored name.
    // We only support GenericRGB right now.
    crate::frameworks::core_graphics::cg_color_space::CGColorSpaceCreateDeviceRGB(env)
}

/// Returns a pointer directly into guest memory where the four RGBA floats
/// of this colour live. Real CG stores these inline; we allocate a small
/// guest buffer on demand and write current values into it.
pub fn CGColorGetComponents(env: &mut Environment, color: CGColorRef) -> ConstPtr<CGFloat> {
    if color.is_null() {
        return ConstPtr::null();
    }
    let &CGColorHostObject { r, g, b, a, .. } = env.objc.borrow(color);
    // Allocate a 4-float guest buffer each call (simple, no lifetime issues).
    let buf: MutPtr<CGFloat> = env.mem.alloc(4 * 4).cast(); // 4 × sizeof(f32)
    env.mem.write(buf, r);
    env.mem.write(buf + 1, g);
    env.mem.write(buf + 2, b);
    env.mem.write(buf + 3, a);
    buf.cast_const()
}

fn CGColorGetNumberOfComponents(
    _env: &mut Environment,
    color: CGColorRef,
) -> crate::frameworks::foundation::NSUInteger {
    if color.is_null() {
        return 0;
    }
    // GenericRGB always has 4 components (r, g, b, alpha).
    4
}

fn CGColorIsValid(_env: &mut Environment, color: CGColorRef) -> bool {
    !color.is_null()
}

// MARK: - CGColorGetConstantColor

// Returns a constant `CGColor` for well-known names
// (`kCGColorBlack`, `kCGColorWhite`, `kCGColorClear`).
// The returned colour is *not* retained by the caller.
fn CGColorGetConstantColor(env: &mut Environment, name: ConstPtr<u8>) -> CGColorRef {
    if name.is_null() {
        return crate::objc::nil;
    }
    let s = env.mem.cstr_at_utf8(name).unwrap_or_default().to_owned();
    let rgba: (CGFloat, CGFloat, CGFloat, CGFloat) = match s.as_str() {
        "kCGColorWhite" => (1.0, 1.0, 1.0, 1.0),
        "kCGColorBlack" => (0.0, 0.0, 0.0, 1.0),
        "kCGColorClear" => (0.0, 0.0, 0.0, 0.0),
        other => {
            log!(
                "CGColorGetConstantColor: unknown color name {:?}, returning clear",
                other
            );
            (0.0, 0.0, 0.0, 0.0)
        }
    };
    // Constant colours are not owned by the caller — return autoreleased.
    let color = from_rgba(env, rgba);
    crate::objc::autorelease(env, color)
}

// MARK: - sRGB / gamma

fn CGColorCreateSRGB(
    env: &mut Environment,
    r: CGFloat,
    g: CGFloat,
    b: CGFloat,
    a: CGFloat,
) -> CGColorRef {
    // We store everything as linear-ish RGB — treat sRGB as the same space.
    from_rgba(env, (r, g, b, a))
}

fn CGColorCreateGenericGrayGamma2_2(
    env: &mut Environment,
    gray: CGFloat,
    alpha: CGFloat,
) -> CGColorRef {
    from_rgba(env, (gray, gray, gray, alpha))
}

// MARK: - HSB colour space

fn CGColorCreateGenericHSB(
    env: &mut Environment,
    hue: CGFloat,
    saturation: CGFloat,
    brightness: CGFloat,
    alpha: CGFloat,
) -> CGColorRef {
    // Convert HSB → RGB inline (same algorithm as UIColor).
    let (r, g, b) = if saturation == 0.0 {
        (brightness, brightness, brightness)
    } else {
        let h6 = (hue * 6.0).rem_euclid(6.0);
        let i = h6 as i32;
        let f = h6 - i as CGFloat;
        let p = brightness * (1.0 - saturation);
        let q = brightness * (1.0 - saturation * f);
        let t = brightness * (1.0 - saturation * (1.0 - f));
        match i {
            0 => (brightness, t, p),
            1 => (q, brightness, p),
            2 => (p, brightness, t),
            3 => (p, q, brightness),
            4 => (t, p, brightness),
            _ => (brightness, p, q),
        }
    };
    from_rgba(env, (r, g, b, alpha))
}

// MARK: - Pattern colour (stub)

fn CGColorCreateWithPattern(
    env: &mut Environment,
    _space: CGColorSpaceRef,
    _pattern: crate::mem::MutVoidPtr, // CGPatternRef
    _components: crate::mem::MutPtr<CGFloat>,
) -> CGColorRef {
    log!("CGColorCreateWithPattern: stubbed, returning clear");
    from_rgba(env, (0.0, 0.0, 0.0, 0.0))
}

// MARK: - Color space matching (stub)

fn CGColorCreateCopyByMatchingToColorSpace(
    env: &mut Environment,
    _space: CGColorSpaceRef,
    _intent: i32, // CGColorRenderingIntent
    color: CGColorRef,
    _options: crate::mem::MutVoidPtr, // CFDictionaryRef
) -> CGColorRef {
    // No real colour management — return a copy in the original space.
    CGColorCreateCopy(env, color)
}

// MARK: - Additional accessors

/// Returns whether two CGColors have the same colour space.
fn CGColorHasAlpha(env: &mut Environment, color: CGColorRef) -> bool {
    if color.is_null() {
        return false;
    }
    env.objc.borrow::<CGColorHostObject>(color).a < 1.0
}

/// Convert to grayscale luminance.
fn CGColorGetLuminance(env: &mut Environment, color: CGColorRef) -> CGFloat {
    if color.is_null() {
        return 0.0;
    }
    let &CGColorHostObject { r, g, b, .. } = env.objc.borrow(color);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGColorRetain(_)),
    export_c_func!(CGColorRelease(_)),
    export_c_func!(CGColorCreate(_, _)),
    export_c_func!(CGColorCreateGenericRGB(_, _, _, _)),
    export_c_func!(CGColorCreateGenericGray(_, _)),
    export_c_func!(CGColorCreateGenericGrayGamma2_2(_, _)),
    export_c_func!(CGColorCreateGenericCMYK(_, _, _, _, _)),
    export_c_func!(CGColorCreateGenericHSB(_, _, _, _)),
    export_c_func!(CGColorCreateSRGB(_, _, _, _)),
    export_c_func!(CGColorCreateCopyWithAlpha(_, _)),
    export_c_func!(CGColorCreateCopy(_)),
    export_c_func!(CGColorCreateWithPattern(_, _, _)),
    export_c_func!(CGColorCreateCopyByMatchingToColorSpace(_, _, _, _)),
    export_c_func!(CGColorEqualToColor(_, _)),
    export_c_func!(CGColorGetAlpha(_)),
    export_c_func!(CGColorGetColorSpace(_)),
    export_c_func!(CGColorGetComponents(_)),
    export_c_func!(CGColorGetNumberOfComponents(_)),
    export_c_func!(CGColorGetConstantColor(_)),
    export_c_func!(CGColorIsValid(_)),
    export_c_func!(CGColorHasAlpha(_)),
    export_c_func!(CGColorGetLuminance(_)),
];

pub const kCGColorWhite: &str = "kCGColorWhite";
pub const kCGColorBlack: &str = "kCGColorBlack";
pub const kCGColorClear: &str = "kCGColorClear";

pub const CONSTANTS: ConstantExports = &[
    ("_kCGColorWhite", HostConstant::NSString(kCGColorWhite)),
    ("_kCGColorBlack", HostConstant::NSString(kCGColorBlack)),
    ("_kCGColorClear", HostConstant::NSString(kCGColorClear)),
];

/// Shortcut for use by `UIColor`: directly construct a `CGColor` instance from
/// an rgba tuple of CGFloats.
pub fn from_rgba(env: &mut Environment, rgba: (CGFloat, CGFloat, CGFloat, CGFloat)) -> CGColorRef {
    let (r, g, b, a) = rgba;
    let host_obj = Box::new(CGColorHostObject {
        color_space_name: kCGColorSpaceGenericRGB,
        r,
        g,
        b,
        a,
    });
    let class = env.objc.get_known_class("_touchHLE_CGColor", &mut env.mem);
    env.objc.alloc_object(class, host_obj, &mut env.mem)
}

/// Shortcut for use by `UIColor`
pub fn to_rgba(objc: &ObjC, color: CGColorRef) -> (CGFloat, CGFloat, CGFloat, CGFloat) {
    let &CGColorHostObject {
        color_space_name,
        r,
        g,
        b,
        a,
        ..
    } = objc.borrow(color);
    assert_eq!(color_space_name, kCGColorSpaceGenericRGB);
    (r, g, b, a)
}
