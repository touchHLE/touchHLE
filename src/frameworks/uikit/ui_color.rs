/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIColor`.

use super::ui_graphics::UIGraphicsGetCurrentContext;
use crate::frameworks::core_graphics::cg_color::{CGColorRef, CGColorRelease, CGColorRetain};
use crate::frameworks::core_graphics::cg_context::CGContextSetRGBFillColor;
use crate::frameworks::core_graphics::{cg_color, CGFloat};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::NSInteger;
use crate::mem::MutPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr, ObjC, SEL,
};
use crate::Environment;
use std::collections::HashMap;

#[derive(Default)]
pub struct State {
    standard_colors: HashMap<SEL, id>,
}

fn get_standard_color(
    env: &mut Environment,
    sel: SEL,
    r: CGFloat,
    g: CGFloat,
    b: CGFloat,
    a: CGFloat,
) -> id {
    if let Some(&existing) = env.framework_state.uikit.ui_color.standard_colors.get(&sel) {
        existing
    } else {
        let new: id = msg_class![env; _touchHLE_UIColor_Static alloc];
        let new: id = msg![env; new initWithRed:r green:g blue:b alpha:a];
        env.framework_state
            .uikit
            .ui_color
            .standard_colors
            .insert(sel, new);
        new
    }
}

#[derive(Default)]
struct UIColorHostObject {
    cg_color: CGColorRef,
    /// Non-nil when this color was created via `colorWithPatternImage:`.
    pattern_image: id,
}
impl HostObject for UIColorHostObject {}

/// Returns the pattern image associated with a UIColor, or nil if it is not a
/// pattern color.
pub fn get_pattern_image(objc: &ObjC, color: id) -> id {
    objc.borrow::<UIColorHostObject>(color).pattern_image
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIColor: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIColorHostObject {
        cg_color: nil,
        pattern_image: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)colorWithCGColor:(CGColorRef)cg_color {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithCGColor:cg_color];
    autorelease(env, new)
}

// `+ (UIColor *)colorWithCIColor:(CIColor *)ciColor` — toll-free
// bridge from CoreImage. Per Apple's docs, components on the CIColor
// are interpreted in the CIColor's colour space, which defaults to
// `kCGColorSpaceDeviceRGB` (sRGB on iOS). Returns a UIColor with
// equivalent RGBA components.
// <https://developer.apple.com/documentation/uikit/uicolor/1621941-colorwithcicolor>
+ (id)colorWithCIColor:(id)ci_color {
    if ci_color == nil {
        return nil;
    }
    let (r, g, b, a) =
        crate::frameworks::core_image::ci_color_components(env, ci_color);
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRed:r green:g blue:b alpha:a];
    autorelease(env, new)
}

+ (id)colorWithRed:(CGFloat)r
             green:(CGFloat)g
              blue:(CGFloat)b
             alpha:(CGFloat)a {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRed:r green:g blue:b alpha:a];
    autorelease(env, new)
}

+ (id)colorWithWhite:(CGFloat)w alpha:(CGFloat)a {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithWhite:w alpha:a];
    autorelease(env, new)
}

+ (id)clearColor    { get_standard_color(env, _cmd, 0.0, 0.0, 0.0, 0.0) }
+ (id)blackColor    { get_standard_color(env, _cmd, 0.0, 0.0, 0.0, 1.0) }
+ (id)whiteColor    { get_standard_color(env, _cmd, 1.0, 1.0, 1.0, 1.0) }
+ (id)darkGrayColor {
    get_standard_color(env, _cmd, 1.0/3.0, 1.0/3.0, 1.0/3.0, 1.0)
}
+ (id)grayColor {
    get_standard_color(env, _cmd, 1.0/2.0, 1.0/2.0, 1.0/2.0, 1.0)
}
+ (id)lightGrayColor {
    get_standard_color(env, _cmd, 2.0/3.0, 2.0/3.0, 2.0/3.0, 1.0)
}
+ (id)blueColor     { get_standard_color(env, _cmd, 0.0, 0.0, 1.0, 1.0) }
+ (id)brownColor    { get_standard_color(env, _cmd, 0.6, 0.4, 0.2, 1.0) }
+ (id)cyanColor     { get_standard_color(env, _cmd, 0.0, 1.0, 1.0, 1.0) }
+ (id)greenColor    { get_standard_color(env, _cmd, 0.0, 1.0, 0.0, 1.0) }
+ (id)magentaColor  { get_standard_color(env, _cmd, 1.0, 0.0, 1.0, 1.0) }
+ (id)orangeColor   { get_standard_color(env, _cmd, 1.0, 0.5, 0.0, 1.0) }
+ (id)purpleColor   { get_standard_color(env, _cmd, 0.5, 0.0, 1.5, 1.0) }
+ (id)redColor      { get_standard_color(env, _cmd, 1.0, 0.0, 0.0, 1.0) }
+ (id)yellowColor   { get_standard_color(env, _cmd, 1.0, 1.0, 0.0, 1.0) }

// Системные цвета для текста
+ (id)lightTextColor { get_standard_color(env, _cmd, 1.0, 1.0, 1.0, 0.6) }
+ (id)darkTextColor  { get_standard_color(env, _cmd, 0.0, 0.0, 0.0, 1.0) }

+ (id)colorWithHue:(CGFloat)h
        saturation:(CGFloat)s
        brightness:(CGFloat)v
             alpha:(CGFloat)a {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithHue:h saturation:s brightness:v alpha:a];
    autorelease(env, new)
}

+ (id)colorWithPatternImage:(id)image { // UIImage*
    let new: id = msg![env; this alloc];
    // Use a fully transparent CGColor; the actual rendering is done by
    // checking `pattern_image` on the UIView side.
    env.objc.borrow_mut::<UIColorHostObject>(new).cg_color =
        cg_color::from_rgba(env, (0.0, 0.0, 0.0, 0.0));
    retain(env, image);
    env.objc.borrow_mut::<UIColorHostObject>(new).pattern_image = image;
    autorelease(env, new)
}

// iOS 13+ dynamic color — we always use the light variant.
+ (id)colorWithDynamicProvider:(id)_block {
    log!("UIColor colorWithDynamicProvider: stubbed, returning clearColor");
    msg_class![env; UIColor clearColor]
}

// MARK: - Additional standard colors (iOS 7+)

+ (id)groupTableViewBackgroundColor {
    get_standard_color(env, _cmd, 0.937, 0.937, 0.957, 1.0)
}
+ (id)viewFlipsideBackgroundColor {
    get_standard_color(env, _cmd, 0.12, 0.12, 0.12, 1.0)
}
+ (id)scrollViewTexturedBackgroundColor {
    get_standard_color(env, _cmd, 0.43, 0.43, 0.43, 1.0)
}
+ (id)underPageBackgroundColor {
    get_standard_color(env, _cmd, 0.15, 0.15, 0.15, 1.0)
}

// iOS 13+ semantic colors — approximated with fixed values.
+ (id)labelColor              { get_standard_color(env, _cmd, 0.0, 0.0, 0.0, 1.0) }
+ (id)secondaryLabelColor     { get_standard_color(env, _cmd, 0.24, 0.24, 0.26, 0.6) }
+ (id)tertiaryLabelColor      { get_standard_color(env, _cmd, 0.24, 0.24, 0.26, 0.3) }
+ (id)quaternaryLabelColor    { get_standard_color(env, _cmd, 0.24, 0.24, 0.26, 0.18) }
+ (id)systemFillColor         { get_standard_color(env, _cmd, 0.47, 0.47, 0.50, 0.2) }
+ (id)secondarySystemFillColor { get_standard_color(env, _cmd, 0.47, 0.47, 0.50, 0.16) }
+ (id)tertiarySystemFillColor  { get_standard_color(env, _cmd, 0.47, 0.47, 0.50, 0.12) }
+ (id)quaternarySystemFillColor { get_standard_color(env, _cmd, 0.45, 0.45, 0.50, 0.08) }
+ (id)placeholderTextColor    { get_standard_color(env, _cmd, 0.24, 0.24, 0.26, 0.3) }
+ (id)systemBackgroundColor   { get_standard_color(env, _cmd, 1.0, 1.0, 1.0, 1.0) }
+ (id)secondarySystemBackgroundColor { get_standard_color(env, _cmd, 0.95, 0.95, 0.97, 1.0) }
+ (id)tertiarySystemBackgroundColor  { get_standard_color(env, _cmd, 1.0, 1.0, 1.0, 1.0) }
+ (id)systemGroupedBackgroundColor   { get_standard_color(env, _cmd, 0.95, 0.95, 0.97, 1.0) }
+ (id)secondarySystemGroupedBackgroundColor { get_standard_color(env, _cmd, 1.0, 1.0, 1.0, 1.0) }
+ (id)tertiarySystemGroupedBackgroundColor  { get_standard_color(env, _cmd, 0.95, 0.95, 0.97, 1.0) }
+ (id)separatorColor          { get_standard_color(env, _cmd, 0.24, 0.24, 0.26, 0.29) }
+ (id)opaqueSeparatorColor    { get_standard_color(env, _cmd, 0.78, 0.78, 0.78, 1.0) }
+ (id)linkColor               { get_standard_color(env, _cmd, 0.0, 0.48, 1.0, 1.0) }
+ (id)systemBlueColor         { get_standard_color(env, _cmd, 0.0, 0.48, 1.0, 1.0) }
+ (id)systemGreenColor        { get_standard_color(env, _cmd, 0.20, 0.78, 0.35, 1.0) }
+ (id)systemIndigoColor       { get_standard_color(env, _cmd, 0.35, 0.34, 0.84, 1.0) }
+ (id)systemOrangeColor       { get_standard_color(env, _cmd, 1.0, 0.58, 0.0, 1.0) }
+ (id)systemPinkColor         { get_standard_color(env, _cmd, 1.0, 0.18, 0.33, 1.0) }
+ (id)systemPurpleColor       { get_standard_color(env, _cmd, 0.69, 0.32, 0.87, 1.0) }
+ (id)systemRedColor          { get_standard_color(env, _cmd, 1.0, 0.23, 0.19, 1.0) }
+ (id)systemTealColor         { get_standard_color(env, _cmd, 0.35, 0.78, 0.98, 1.0) }
+ (id)systemYellowColor       { get_standard_color(env, _cmd, 1.0, 0.80, 0.0, 1.0) }
+ (id)systemGrayColor         { get_standard_color(env, _cmd, 0.56, 0.56, 0.58, 1.0) }
+ (id)systemGray2Color        { get_standard_color(env, _cmd, 0.68, 0.68, 0.70, 1.0) }
+ (id)systemGray3Color        { get_standard_color(env, _cmd, 0.78, 0.78, 0.80, 1.0) }
+ (id)systemGray4Color        { get_standard_color(env, _cmd, 0.82, 0.82, 0.84, 1.0) }
+ (id)systemGray5Color        { get_standard_color(env, _cmd, 0.90, 0.90, 0.92, 1.0) }
+ (id)systemGray6Color        { get_standard_color(env, _cmd, 0.95, 0.95, 0.97, 1.0) }

// MARK: - HSB init

- (id)initWithHue:(CGFloat)h
       saturation:(CGFloat)s
       brightness:(CGFloat)v
            alpha:(CGFloat)a {
    // Convert HSB → RGB.
    let (r, g, b) = hsb_to_rgb(h, s, v);
    msg![env; this initWithRed:r green:g blue:b alpha:a]
}

// MARK: - Component accessors

- (bool)getWhite:(MutPtr<CGFloat>)white alpha:(MutPtr<CGFloat>)alpha {
    let color = env.objc.borrow::<UIColorHostObject>(this).cg_color;
    let (r, g, b, a) = cg_color::to_rgba(&env.objc, color);
    // Return luminance as white component.
    let w = 0.299 * r + 0.587 * g + 0.114 * b;
    if !white.is_null() { env.mem.write(white, w); }
    if !alpha.is_null() { env.mem.write(alpha, a); }
    true
}

- (bool)getHue:(MutPtr<CGFloat>)hue
    saturation:(MutPtr<CGFloat>)saturation
    brightness:(MutPtr<CGFloat>)brightness
         alpha:(MutPtr<CGFloat>)alpha {
    let color = env.objc.borrow::<UIColorHostObject>(this).cg_color;
    let (r, g, b, a) = cg_color::to_rgba(&env.objc, color);
    let (h, s, v) = rgb_to_hsb(r, g, b);
    if !hue.is_null()        { env.mem.write(hue, h); }
    if !saturation.is_null() { env.mem.write(saturation, s); }
    if !brightness.is_null() { env.mem.write(brightness, v); }
    if !alpha.is_null()      { env.mem.write(alpha, a); }
    true
}

// MARK: - Drawing

- (())setStroke {
    use crate::frameworks::core_graphics::cg_context::CGContextSetRGBStrokeColor;
    let context = UIGraphicsGetCurrentContext(env);
    if context == nil { return; }
    let (r, g, b, a) = get_rgba(&env.objc, this);
    CGContextSetRGBStrokeColor(env, context, r, g, b, a);
}

// MARK: - Equality / description

- (bool)isEqual:(id)other {
    if this == other { return true; }
    let (r1, g1, b1, a1) = get_rgba(&env.objc, this);
    let (r2, g2, b2, a2) = get_rgba(&env.objc, other);
    r1 == r2 && g1 == g2 && b1 == b2 && a1 == a2
}

- (id)description {
    let (r, g, b, a) = get_rgba(&env.objc, this);
    let s = format!("UIDeviceRGBColorSpace {} {} {} {}", r, g, b, a);
    let ns = crate::frameworks::foundation::ns_string::from_rust_string(env, s);
    autorelease(env, ns)
}

- (id)initWithCGColor:(CGColorRef)cg_color {
    CGColorRetain(env, cg_color);
    env.objc.borrow_mut::<UIColorHostObject>(this).cg_color = cg_color;
    this
}

- (id)initWithWhite:(CGFloat)w alpha:(CGFloat)a {
    let w = w.clamp(0.0, 1.0);
    let a = a.clamp(0.0, 1.0);

    let rgba = (w, w, w, a);
    env.objc.borrow_mut::<UIColorHostObject>(this).cg_color =
        cg_color::from_rgba(env, rgba);

    this
}

- (id)initWithRed:(CGFloat)r
            green:(CGFloat)g
             blue:(CGFloat)b
            alpha:(CGFloat)a {
    env.objc.borrow_mut::<UIColorHostObject>(this).cg_color =
        cg_color::from_rgba(env, (r, g, b, a));
    this
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    let key_ns_string = get_static_str(env, "UIAlpha");
    let a: CGFloat = msg![env; coder decodeFloatForKey:key_ns_string];

    let key_ns_string = get_static_str(env, "UIColorComponentCount");
    let count: NSInteger = msg![env; coder decodeIntegerForKey:key_ns_string];

    match count {
        // ----------------------------------------------------------------
        // 4 components — RGBA
        // ----------------------------------------------------------------
        4 => {
            let key_r = get_static_str(env, "UIRed");
            let key_g = get_static_str(env, "UIGreen");
            let key_b = get_static_str(env, "UIBlue");

            let r: CGFloat = msg![env; coder decodeFloatForKey:key_r];
            let g: CGFloat = msg![env; coder decodeFloatForKey:key_g];
            let b: CGFloat = msg![env; coder decodeFloatForKey:key_b];

            log_dbg!("UIColor initWithCoder RGBA ({},{},{},{})", r, g, b, a);
            msg![env; this initWithRed:r green:g blue:b alpha:a]
        }

        // ----------------------------------------------------------------
        // 2 components — grayscale (white + alpha)
        // ----------------------------------------------------------------
        2 => {
            let key_w = get_static_str(env, "UIWhite");
            let w: CGFloat = msg![env; coder decodeFloatForKey:key_w];

            log_dbg!("UIColor initWithCoder Gray ({},{})", w, a);
            msg![env; this initWithWhite:w alpha:a]
        }

        // ----------------------------------------------------------------
        // 5 components — CMYK (cyan, magenta, yellow, black + alpha)
        // Convert to RGBA before storing.
        // ----------------------------------------------------------------
        5 => {
            let key_c = get_static_str(env, "UICyan");
            let key_m = get_static_str(env, "UIMagenta");
            let key_y = get_static_str(env, "UIYellow");
            let key_k = get_static_str(env, "UIBlack");

            let c: CGFloat = msg![env; coder decodeFloatForKey:key_c];
            let m: CGFloat = msg![env; coder decodeFloatForKey:key_m];
            let y: CGFloat = msg![env; coder decodeFloatForKey:key_y];
            let k: CGFloat = msg![env; coder decodeFloatForKey:key_k];

            // Standard CMYK → RGB conversion.
            let r = (1.0 - c) * (1.0 - k);
            let g = (1.0 - m) * (1.0 - k);
            let b = (1.0 - y) * (1.0 - k);

            log_dbg!("UIColor initWithCoder CMYK ({},{},{},{}) => RGB ({},{},{})", c, m, y, k, r, g, b);
            msg![env; this initWithRed:r green:g blue:b alpha:a]
        }

        // ----------------------------------------------------------------
        // 3 components — HSB (hue, saturation, brightness + alpha)
        // Convert to RGBA before storing.
        // ----------------------------------------------------------------
        3 => {
            // Try HSB first, fall back to RGB with missing blue.
            let key_h = get_static_str(env, "UIHue");
            let has_hue: bool = msg![env; coder containsValueForKey:key_h];

            if has_hue {
                let key_s = get_static_str(env, "UISaturation");
                let key_bri = get_static_str(env, "UIBrightness");

                let h: CGFloat = msg![env; coder decodeFloatForKey:key_h];
                let s: CGFloat = msg![env; coder decodeFloatForKey:key_s];
                let v: CGFloat = msg![env; coder decodeFloatForKey:key_bri];

                let (r, g, b) = hsb_to_rgb(h, s, v);
                log_dbg!("UIColor initWithCoder HSB ({},{},{}) => RGB ({},{},{})", h, s, v, r, g, b);
                msg![env; this initWithRed:r green:g blue:b alpha:a]
            } else {
                // 3-component RGB without separate alpha stored in count.
                let key_r = get_static_str(env, "UIRed");
                let key_g = get_static_str(env, "UIGreen");
                let key_b = get_static_str(env, "UIBlue");

                let r: CGFloat = msg![env; coder decodeFloatForKey:key_r];
                let g: CGFloat = msg![env; coder decodeFloatForKey:key_g];
                let b: CGFloat = msg![env; coder decodeFloatForKey:key_b];

                log_dbg!("UIColor initWithCoder RGB3 ({},{},{})", r, g, b);
                msg![env; this initWithRed:r green:g blue:b alpha:a]
            }
        }

        // ----------------------------------------------------------------
        // 1 component — alpha only (clear / black with alpha)
        // ----------------------------------------------------------------
        1 => {
            log_dbg!("UIColor initWithCoder Alpha-only ({})", a);
            msg![env; this initWithWhite:0.0 alpha:a]
        }

        // ----------------------------------------------------------------
        // 0 or unknown — try to decode a system color name, otherwise
        // fall back to clear color.
        // ----------------------------------------------------------------
        _ => {
            let key_sys = get_static_str(env, "UISystemColorName");
            let has_name: bool = msg![env; coder containsValueForKey:key_sys];

            if has_name {
                let name_obj: id = msg![env; coder decodeObjectForKey:key_sys];
                if name_obj != nil {
                    let name = crate::frameworks::foundation::ns_string::to_rust_string(env, name_obj);
                    log_dbg!("UIColor initWithCoder system color name: {}", name);
                    // Map well-known system color names to RGBA.
                    let (r, g, b, alpha) = system_color_rgba(&name, a);
                    return msg![env; this initWithRed:r green:g blue:b alpha:alpha];
                }
            }

            // Fallback: try to decode a CGColor or pattern image.
            let key_cg = get_static_str(env, "UICGColor");
            let has_cg: bool = msg![env; coder containsValueForKey:key_cg];
            if has_cg {
                let _cg_data: id = msg![env; coder decodeObjectForKey:key_cg];
                log_dbg!("UIColor initWithCoder CGColor data — falling back to clear");
            }

            log!(
                "Warning: UIColor initWithCoder: unknown component count {} — \
                 defaulting to clear color",
                count
            );
            msg![env; this initWithWhite:0.0 alpha:0.0]
        }
    }
}

- (bool)getRed:(MutPtr<CGFloat>)r
         green:(MutPtr<CGFloat>)g
          blue:(MutPtr<CGFloat>)b
         alpha:(MutPtr<CGFloat>)a {
    let color = env.objc.borrow::<UIColorHostObject>(this).cg_color;
    let (r_, g_, b_, a_) = cg_color::to_rgba(&env.objc, color);
    env.mem.write(r, r_);
    env.mem.write(g, g_);
    env.mem.write(b, b_);
    env.mem.write(a, a_);
    true
}

- (())set {
    msg![env; this setFill]
}

- (())setFill {
    let context = UIGraphicsGetCurrentContext(env);
    assert_ne!(context, nil);
    let (r, g, b, a) = get_rgba(&env.objc, this);
    CGContextSetRGBFillColor(env, context, r, g, b, a);
}

- (CGColorRef)CGColor {
    env.objc.borrow::<UIColorHostObject>(this).cg_color
}

- (())dealloc {
    let host = env.objc.borrow_mut::<UIColorHostObject>(this);
    let color = host.cg_color;
    let pattern = host.pattern_image;
    CGColorRelease(env, color);
    release(env, pattern);

    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)colorWithAlphaComponent:(CGFloat)a {
    let a = a.clamp(0.0, 1.0);
    let (r, g, b, _) = get_rgba(&env.objc, this);
    msg_class![env; UIColor colorWithRed:r green:g blue:b alpha:a]
}

@end

@implementation UICGColor: UIColor
@end
@implementation UIDeviceRGBColor: UIColor
@end
@implementation UICachedDeviceWhiteColor: UIColor
@end

@implementation _touchHLE_UIColor_Static: UIColor

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIColorHostObject {
        cg_color: nil,
        pattern_image: nil,
    });
    env.objc.alloc_static_object(this, host_object, &mut env.mem)
}

- (id) retain { this }
- (()) release {}
- (id) autorelease { this }

@end

};

pub fn get_rgba(objc: &ObjC, ui_color: id) -> (CGFloat, CGFloat, CGFloat, CGFloat) {
    let color = objc.borrow::<UIColorHostObject>(ui_color).cg_color;
    cg_color::to_rgba(objc, color)
}

fn hsb_to_rgb(h: CGFloat, s: CGFloat, v: CGFloat) -> (CGFloat, CGFloat, CGFloat) {
    if s == 0.0 {
        return (v, v, v); // achromatic
    }
    let h6 = (h * 6.0).fract() * 6.0; // hue sector 0-6
    let i = h6 as i32;
    let f = h6 - i as CGFloat;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

/// Map a UIKit system color name string to an RGBA tuple.
/// Unknown names fall back to (0, 0, 0, alpha).
fn system_color_rgba(name: &str, alpha: CGFloat) -> (CGFloat, CGFloat, CGFloat, CGFloat) {
    match name {
        "systemRedColor" | "redColor" => (1.0, 0.235, 0.188, alpha),
        "systemGreenColor" | "greenColor" => (0.204, 0.78, 0.349, alpha),
        "systemBlueColor" | "blueColor" => (0.0, 0.478, 1.0, alpha),
        "systemOrangeColor" | "orangeColor" => (1.0, 0.584, 0.0, alpha),
        "systemYellowColor" | "yellowColor" => (1.0, 0.8, 0.0, alpha),
        "systemPinkColor" => (1.0, 0.176, 0.333, alpha),
        "systemPurpleColor" | "purpleColor" => (0.686, 0.322, 0.871, alpha),
        "systemTealColor" => (0.353, 0.784, 0.98, alpha),
        "systemIndigoColor" => (0.345, 0.337, 0.839, alpha),
        "systemBrownColor" | "brownColor" => (0.635, 0.518, 0.369, alpha),
        "systemMintColor" => (0.0, 0.78, 0.745, alpha),
        "systemCyanColor" | "cyanColor" => (0.196, 0.678, 0.902, alpha),
        "whiteColor" | "systemWhiteColor" => (1.0, 1.0, 1.0, alpha),
        "blackColor" | "systemBlackColor" => (0.0, 0.0, 0.0, alpha),
        "grayColor" | "systemGrayColor" => (0.557, 0.557, 0.576, alpha),
        "lightGrayColor" => (0.667, 0.667, 0.667, alpha),
        "darkGrayColor" => (0.333, 0.333, 0.333, alpha),
        "clearColor" => (0.0, 0.0, 0.0, 0.0),
        "labelColor" | "darkTextColor" => (0.0, 0.0, 0.0, alpha),
        "secondaryLabelColor" => (0.235, 0.235, 0.263, alpha * 0.6),
        "placeholderTextColor" => (0.235, 0.235, 0.263, alpha * 0.3),
        "linkColor" => (0.0, 0.478, 1.0, alpha),
        "separatorColor" => (0.235, 0.235, 0.263, alpha * 0.29),
        "groupTableViewBackgroundColor" | "systemGroupedBackgroundColor" => {
            (0.949, 0.949, 0.969, alpha)
        }
        "tableCellGroupedBackgroundColor" | "secondarySystemGroupedBackgroundColor" => {
            (1.0, 1.0, 1.0, alpha)
        }
        _ => {
            log_dbg!(
                "UIColor system color name {:?} not recognised — using black",
                name
            );
            (0.0, 0.0, 0.0, alpha)
        }
    }
}

/// Convert RGB to HSB (each in 0.0–1.0).
fn rgb_to_hsb(r: CGFloat, g: CGFloat, b: CGFloat) -> (CGFloat, CGFloat, CGFloat) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let v = max;
    let s = if max == 0.0 { 0.0 } else { delta / max };
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / delta).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / delta + 2.0) / 6.0
    } else {
        ((r - g) / delta + 4.0) / 6.0
    };
    (h, s, v)
}
