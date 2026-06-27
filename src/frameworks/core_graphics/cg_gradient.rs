/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#![allow(dead_code)]
//! `CGGradient.h`

use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::core_graphics::cg_color_space::CGColorSpaceRef;
use crate::frameworks::core_graphics::CGFloat;
use crate::mem::{ConstPtr, GuestUSize};
use crate::objc::{objc_classes, ClassExports, HostObject};
use crate::Environment;

pub type CGGradientRef = CFTypeRef;

/// Drawing options — can be combined.
type CGGradientDrawingOptions = u32;
const kCGGradientDrawsBeforeStartLocation: CGGradientDrawingOptions = 1 << 0;
const kCGGradientDrawsAfterEndLocation: CGGradientDrawingOptions = 1 << 1;

/// One colour stop: a normalised location in [0, 1] plus RGBA components.
#[derive(Clone, Debug)]
struct ColorStop {
    location: CGFloat,
    r: CGFloat,
    g: CGFloat,
    b: CGFloat,
    a: CGFloat,
}

#[derive(Default)]
struct CGGradientHostObject {
    stops: Vec<ColorStop>,
}
impl HostObject for CGGradientHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_CGGradient: NSObject

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

// MARK: - Internal helpers

fn alloc_gradient(env: &mut Environment, stops: Vec<ColorStop>) -> CGGradientRef {
    let class = env
        .objc
        .get_known_class("_touchHLE_CGGradient", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CGGradientHostObject { stops }),
        &mut env.mem,
    )
}

/// Interpolate the gradient colour at a normalised position `t` in [0, 1].
/// Returns `(r, g, b, a)`.
pub fn gradient_color_at(
    env: &Environment,
    gradient: CGGradientRef,
    t: CGFloat,
) -> (CGFloat, CGFloat, CGFloat, CGFloat) {
    let stops = &env.objc.borrow::<CGGradientHostObject>(gradient).stops;
    if stops.is_empty() {
        return (0.0, 0.0, 0.0, 1.0);
    }
    if t <= stops.first().unwrap().location {
        let s = stops.first().unwrap();
        return (s.r, s.g, s.b, s.a);
    }
    if t >= stops.last().unwrap().location {
        let s = stops.last().unwrap();
        return (s.r, s.g, s.b, s.a);
    }
    // Find the surrounding pair and lerp.
    for i in 0..stops.len() - 1 {
        let lo = &stops[i];
        let hi = &stops[i + 1];

        if t >= lo.location && t <= hi.location {
            let span = hi.location - lo.location;
            let f = if span > 0.0 {
                (t - lo.location) / span
            } else {
                0.0
            };
            return (
                lo.r + f * (hi.r - lo.r),
                lo.g + f * (hi.g - lo.g),
                lo.b + f * (hi.b - lo.b),
                lo.a + f * (hi.a - lo.a),
            );
        }
    }
    let s = stops.last().unwrap();
    (s.r, s.g, s.b, s.a)
}

// MARK: - Lifecycle

pub fn CGGradientRetain(env: &mut Environment, gradient: CGGradientRef) -> CGGradientRef {
    if !gradient.is_null() {
        CFRetain(env, gradient)
    } else {
        gradient
    }
}

pub fn CGGradientRelease(env: &mut Environment, gradient: CGGradientRef) {
    if !gradient.is_null() {
        CFRelease(env, gradient);
    }
}

// MARK: - Constructors

/// Create from a flat array of `CGFloat` colour components.
/// `components` layout depends on the colour space:
///   - RGB  → [r, g, b, a,  r, g, b, a, ...]
///   - Grey → [w, a,  w, a, ...]
fn CGGradientCreateWithColorComponents(
    env: &mut Environment,
    color_space: CGColorSpaceRef,
    components: ConstPtr<CGFloat>,
    locations: ConstPtr<CGFloat>,
    count: GuestUSize,
) -> CGGradientRef {
    if count == 0 {
        return alloc_gradient(env, vec![]);
    }

    // Determine components-per-colour from the colour space model.
    // We support RGB (4 components incl. alpha) and Grey (2 components).
    let components_per_color: usize = if color_space.is_null() {
        4
    } else {
        // CGColorSpaceGetNumberOfComponents returns the non-alpha count.
        let n: GuestUSize =
            crate::frameworks::core_graphics::cg_color_space::CGColorSpaceGetNumberOfComponents(
                env,
                color_space,
            );
        (n + 1) as usize // +1 for alpha
    };

    let mut stops = Vec::with_capacity(count as usize);

    for i in 0..count as usize {
        let loc: CGFloat = if locations.is_null() {
            // Evenly spaced when no locations are provided.
            if count == 1 {
                0.0
            } else {
                i as CGFloat / (count as CGFloat - 1.0)
            }
        } else {
            env.mem.read(locations + i as u32)
        };

        let base = i * components_per_color;

        let (r, g, b, a) = if components_per_color >= 4 {
            (
                env.mem.read(components + (base) as u32),
                env.mem.read(components + (base + 1) as u32),
                env.mem.read(components + (base + 2) as u32),
                env.mem.read(components + (base + 3) as u32),
            )
        } else {
            // Greyscale: w, a
            let w = env.mem.read(components + (base) as u32);
            let a = env.mem.read(components + (base + 1) as u32);
            (w, w, w, a)
        };

        stops.push(ColorStop {
            location: loc,
            r,
            g,
            b,
            a,
        });
    }

    // Ensure stops are sorted by location.
    stops.sort_by(|a, b| a.location.partial_cmp(&b.location).unwrap());

    alloc_gradient(env, stops)
}

/// Create from an array of `CGColorRef` (which in our impl are just
/// `_touchHLE_CGColor` objects carrying RGBA floats).
fn CGGradientCreateWithColors(
    env: &mut Environment,
    _color_space: CGColorSpaceRef,
    colors: crate::objc::id, // CFArrayRef / NSArray* of CGColorRef
    locations: ConstPtr<CGFloat>,
) -> CGGradientRef {
    use crate::frameworks::core_graphics::cg_color::CGColorGetComponents;
    use crate::objc::msg;

    let count: GuestUSize = msg![env; colors count];
    if count == 0 {
        return alloc_gradient(env, vec![]);
    }

    let mut stops = Vec::with_capacity(count as usize);

    for i in 0..count as usize {
        let color: crate::objc::id = msg![env; colors objectAtIndex:(i as GuestUSize)];
        let comps: ConstPtr<CGFloat> = CGColorGetComponents(env, color);

        let loc: CGFloat = if locations.is_null() {
            if count == 1 {
                0.0
            } else {
                i as CGFloat / (count as CGFloat - 1.0)
            }
        } else {
            env.mem.read(locations + i as u32)
        };

        let r = env.mem.read(comps);
        let g = env.mem.read(comps + 1);
        let b = env.mem.read(comps + 2);
        let a = env.mem.read(comps + 3);

        stops.push(ColorStop {
            location: loc,
            r,
            g,
            b,
            a,
        });
    }

    stops.sort_by(|a, b| a.location.partial_cmp(&b.location).unwrap());

    alloc_gradient(env, stops)
}

// MARK: - Drawing (delegated to cg_context)
// The actual rasterisation lives in CGContextDrawLinearGradient /
// CGContextDrawRadialGradient in cg_context.rs, which call gradient_color_at.
// These stubs exist so the symbols resolve and forward to that module.

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGGradientRetain(_)),
    export_c_func!(CGGradientRelease(_)),
    export_c_func!(CGGradientCreateWithColorComponents(_, _, _, _)),
    export_c_func!(CGGradientCreateWithColors(_, _, _)),
];
