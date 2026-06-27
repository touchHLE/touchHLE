/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for `CoreImage.framework/CoreImage`.
//!
//! CoreImage is the GPU-accelerated image-processing framework. iOS 5+
//! apps reach a handful of `CIContext` / `CIFilter` constants by Mach-O
//! symbol lookup even when they don't actually run any filters (they're
//! often referenced from defensive `if (kCISomething != nil)` guards).
//! Without a [crate::dyld::HostDylib] entry for these names, the linker
//! leaves them NULL and the first dereference takes down the emulator.

use crate::dyld::{ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::core_graphics::CGFloat;
use crate::frameworks::foundation::ns_string;
use crate::objc::{autorelease, id, msg, nil, objc_classes, ClassExports, HostObject, NSZonePtr};

/// Backing for `CIColor`. Apple's `CIColor.h` documents the framework's
/// own colour model: RGBA components in [0.0, 1.0] plus a colour space.
/// We keep components in sRGB / "device RGB" (CoreImage's default working
/// space) which matches what `+colorWithCIColor:` on `UIColor` expects.
#[derive(Default)]
struct CIColorHostObject {
    red: CGFloat,
    green: CGFloat,
    blue: CGFloat,
    alpha: CGFloat,
}
impl HostObject for CIColorHostObject {}

/// Read RGBA components from a `CIColor`. Used by
/// `+[UIColor colorWithCIColor:]`.
pub fn ci_color_components(env: &crate::Environment, color: id) -> (CGFloat, CGFloat, CGFloat, CGFloat) {
    let h = env.objc.borrow::<CIColorHostObject>(color);
    (h.red, h.green, h.blue, h.alpha)
}

/// Parse a `CIColor`-style component string. Apple documents
/// `colorWithString:` and `-stringRepresentation` as producing a
/// space-separated component list (`"r g b"` or `"r g b a"`), see
/// <https://developer.apple.com/documentation/coreimage/cicolor/1438098-colorwithstring>.
fn parse_ci_color_string(s: &str) -> Option<(CGFloat, CGFloat, CGFloat, CGFloat)> {
    let mut parts = s.split_whitespace();
    let r: CGFloat = parts.next()?.parse().ok()?;
    let g: CGFloat = parts.next()?.parse().ok()?;
    let b: CGFloat = parts.next()?.parse().ok()?;
    let a: CGFloat = parts.next().map(|x| x.parse().unwrap_or(1.0)).unwrap_or(1.0);
    Some((r, g, b, a))
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// `CIColor` (iOS 5.0+). Apple's `CIColor.h`. We back it with a 4-component
// sRGB tuple. Toll-free bridging with `UIColor` is implemented separately
// in `+[UIColor colorWithCIColor:]` (see `uikit/ui_color.rs`).
// <https://developer.apple.com/documentation/coreimage/cicolor>
@implementation CIColor: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(CIColorHostObject {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 0.0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// `+ (instancetype)colorWithRed:(CGFloat)r green:(CGFloat)g blue:(CGFloat)b`
+ (id)colorWithRed:(CGFloat)r green:(CGFloat)g blue:(CGFloat)b {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRed:r green:g blue:b];
    autorelease(env, new)
}

// `+ (instancetype)colorWithRed:(CGFloat)r green:(CGFloat)g blue:(CGFloat)b
//                          alpha:(CGFloat)a`
+ (id)colorWithRed:(CGFloat)r green:(CGFloat)g blue:(CGFloat)b alpha:(CGFloat)a {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRed:r green:g blue:b alpha:a];
    autorelease(env, new)
}

// `+ (instancetype)colorWithString:(NSString *)representation`
+ (id)colorWithString:(id)representation {
    if representation == nil {
        return nil;
    }
    let rust_s = ns_string::to_rust_string(env, representation);
    let Some((r, g, b, a)) = parse_ci_color_string(&rust_s) else {
        log!(
            "Warning: +[CIColor colorWithString:{:?}] could not parse {:?}",
            representation, rust_s
        );
        return nil;
    };
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRed:r green:g blue:b alpha:a];
    autorelease(env, new)
}

+ (id)blackColor {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRed:(0.0 as CGFloat) green:(0.0 as CGFloat) blue:(0.0 as CGFloat) alpha:(1.0 as CGFloat)];
    autorelease(env, new)
}
+ (id)whiteColor {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRed:(1.0 as CGFloat) green:(1.0 as CGFloat) blue:(1.0 as CGFloat) alpha:(1.0 as CGFloat)];
    autorelease(env, new)
}
+ (id)redColor {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRed:(1.0 as CGFloat) green:(0.0 as CGFloat) blue:(0.0 as CGFloat) alpha:(1.0 as CGFloat)];
    autorelease(env, new)
}
+ (id)greenColor {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRed:(0.0 as CGFloat) green:(1.0 as CGFloat) blue:(0.0 as CGFloat) alpha:(1.0 as CGFloat)];
    autorelease(env, new)
}
+ (id)blueColor {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRed:(0.0 as CGFloat) green:(0.0 as CGFloat) blue:(1.0 as CGFloat) alpha:(1.0 as CGFloat)];
    autorelease(env, new)
}
+ (id)clearColor {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithRed:(0.0 as CGFloat) green:(0.0 as CGFloat) blue:(0.0 as CGFloat) alpha:(0.0 as CGFloat)];
    autorelease(env, new)
}

- (id)initWithRed:(CGFloat)r green:(CGFloat)g blue:(CGFloat)b {
    msg![env; this initWithRed:r green:g blue:b alpha:(1.0 as CGFloat)]
}

- (id)initWithRed:(CGFloat)r green:(CGFloat)g blue:(CGFloat)b alpha:(CGFloat)a {
    let h = env.objc.borrow_mut::<CIColorHostObject>(this);
    h.red = r;
    h.green = g;
    h.blue = b;
    h.alpha = a;
    this
}

- (CGFloat)red   { env.objc.borrow::<CIColorHostObject>(this).red }
- (CGFloat)green { env.objc.borrow::<CIColorHostObject>(this).green }
- (CGFloat)blue  { env.objc.borrow::<CIColorHostObject>(this).blue }
- (CGFloat)alpha { env.objc.borrow::<CIColorHostObject>(this).alpha }

// `-numberOfComponents` — `CIColor` always exposes RGBA, so 4.
- (i32)numberOfComponents { 4 }

// `- (NSString *)stringRepresentation` — same format
// `+colorWithString:` expects.
- (id)stringRepresentation {
    let (r, g, b, a) = ci_color_components(env, this);
    let s = format!("{} {} {} {}", r, g, b, a);
    ns_string::from_rust_string(env, s)
}

@end

};

pub const CONSTANTS: ConstantExports = &[
    // CIContext options.
    (
        "_kCIContextWorkingColorSpace",
        HostConstant::NSString("kCIContextWorkingColorSpace"),
    ),
    (
        "_kCIContextOutputColorSpace",
        HostConstant::NSString("kCIContextOutputColorSpace"),
    ),
    (
        "_kCIContextUseSoftwareRenderer",
        HostConstant::NSString("kCIContextUseSoftwareRenderer"),
    ),
    (
        "_kCIContextPriorityRequestLow",
        HostConstant::NSString("kCIContextPriorityRequestLow"),
    ),
    (
        "_kCIContextOutputPremultiplied",
        HostConstant::NSString("kCIContextOutputPremultiplied"),
    ),
    (
        "_kCIContextHighQualityDownsample",
        HostConstant::NSString("kCIContextHighQualityDownsample"),
    ),
    // CIFilter input keys (the most common ones).
    (
        "_kCIInputImageKey",
        HostConstant::NSString("inputImage"),
    ),
    (
        "_kCIInputBackgroundImageKey",
        HostConstant::NSString("inputBackgroundImage"),
    ),
    (
        "_kCIInputTimeKey",
        HostConstant::NSString("inputTime"),
    ),
    (
        "_kCIInputTransformKey",
        HostConstant::NSString("inputTransform"),
    ),
    (
        "_kCIInputScaleKey",
        HostConstant::NSString("inputScale"),
    ),
    (
        "_kCIInputAspectRatioKey",
        HostConstant::NSString("inputAspectRatio"),
    ),
    (
        "_kCIInputCenterKey",
        HostConstant::NSString("inputCenter"),
    ),
    (
        "_kCIInputRadiusKey",
        HostConstant::NSString("inputRadius"),
    ),
    (
        "_kCIInputAngleKey",
        HostConstant::NSString("inputAngle"),
    ),
    (
        "_kCIInputRefractionKey",
        HostConstant::NSString("inputRefraction"),
    ),
    (
        "_kCIInputWidthKey",
        HostConstant::NSString("inputWidth"),
    ),
    (
        "_kCIInputSharpnessKey",
        HostConstant::NSString("inputSharpness"),
    ),
    (
        "_kCIInputIntensityKey",
        HostConstant::NSString("inputIntensity"),
    ),
    (
        "_kCIInputEVKey",
        HostConstant::NSString("inputEV"),
    ),
    (
        "_kCIInputSaturationKey",
        HostConstant::NSString("inputSaturation"),
    ),
    (
        "_kCIInputColorKey",
        HostConstant::NSString("inputColor"),
    ),
    (
        "_kCIInputBrightnessKey",
        HostConstant::NSString("inputBrightness"),
    ),
    (
        "_kCIInputContrastKey",
        HostConstant::NSString("inputContrast"),
    ),
    (
        "_kCIInputGradientImageKey",
        HostConstant::NSString("inputGradientImage"),
    ),
    (
        "_kCIInputMaskImageKey",
        HostConstant::NSString("inputMaskImage"),
    ),
    (
        "_kCIInputTargetImageKey",
        HostConstant::NSString("inputTargetImage"),
    ),
    (
        "_kCIInputExtentKey",
        HostConstant::NSString("inputExtent"),
    ),
    // CIFilter output key.
    (
        "_kCIOutputImageKey",
        HostConstant::NSString("outputImage"),
    ),
    // Common filter category constants.
    (
        "_kCICategoryDistortionEffect",
        HostConstant::NSString("CICategoryDistortionEffect"),
    ),
    (
        "_kCICategoryGeometryAdjustment",
        HostConstant::NSString("CICategoryGeometryAdjustment"),
    ),
    (
        "_kCICategoryCompositeOperation",
        HostConstant::NSString("CICategoryCompositeOperation"),
    ),
    (
        "_kCICategoryHalftoneEffect",
        HostConstant::NSString("CICategoryHalftoneEffect"),
    ),
    (
        "_kCICategoryColorAdjustment",
        HostConstant::NSString("CICategoryColorAdjustment"),
    ),
    (
        "_kCICategoryColorEffect",
        HostConstant::NSString("CICategoryColorEffect"),
    ),
    (
        "_kCICategoryTransition",
        HostConstant::NSString("CICategoryTransition"),
    ),
    (
        "_kCICategoryTileEffect",
        HostConstant::NSString("CICategoryTileEffect"),
    ),
    (
        "_kCICategoryGenerator",
        HostConstant::NSString("CICategoryGenerator"),
    ),
    (
        "_kCICategoryReduction",
        HostConstant::NSString("CICategoryReduction"),
    ),
    (
        "_kCICategoryGradient",
        HostConstant::NSString("CICategoryGradient"),
    ),
    (
        "_kCICategoryStylize",
        HostConstant::NSString("CICategoryStylize"),
    ),
    (
        "_kCICategorySharpen",
        HostConstant::NSString("CICategorySharpen"),
    ),
    (
        "_kCICategoryBlur",
        HostConstant::NSString("CICategoryBlur"),
    ),
    (
        "_kCICategoryVideo",
        HostConstant::NSString("CICategoryVideo"),
    ),
    (
        "_kCICategoryStillImage",
        HostConstant::NSString("CICategoryStillImage"),
    ),
    (
        "_kCICategoryInterlaced",
        HostConstant::NSString("CICategoryInterlaced"),
    ),
    (
        "_kCICategoryNonSquarePixels",
        HostConstant::NSString("CICategoryNonSquarePixels"),
    ),
    (
        "_kCICategoryHighDynamicRange",
        HostConstant::NSString("CICategoryHighDynamicRange"),
    ),
    (
        "_kCICategoryBuiltIn",
        HostConstant::NSString("CICategoryBuiltIn"),
    ),
    (
        "_kCIAttributeFilterName",
        HostConstant::NSString("CIAttributeFilterName"),
    ),
    (
        "_kCIAttributeFilterDisplayName",
        HostConstant::NSString("CIAttributeFilterDisplayName"),
    ),
    (
        "_kCIAttributeFilterCategories",
        HostConstant::NSString("CIAttributeFilterCategories"),
    ),
];

pub const FUNCTIONS: FunctionExports = &[];
