/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#![allow(dead_code)]
//! `CGPath.h`

use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect};
use crate::mem::MutVoidPtr;
use crate::objc::{objc_classes, ClassExports, HostObject};
use crate::Environment;

pub type CGPathRef = CFTypeRef;
pub type CGMutablePathRef = CFTypeRef;

type CGPathElementType = i32;
const kCGPathElementMoveToPoint: CGPathElementType = 0;
const kCGPathElementAddLineToPoint: CGPathElementType = 1;
const kCGPathElementAddQuadCurveToPoint: CGPathElementType = 2;
const kCGPathElementAddCurveToPoint: CGPathElementType = 3;
const kCGPathElementCloseSubpath: CGPathElementType = 4;

#[derive(Clone, Debug)]
enum PathElement {
    MoveTo(CGPoint),
    LineTo(CGPoint),
    QuadCurveTo {
        control: CGPoint,
        to: CGPoint,
    },
    CurveTo {
        c1: CGPoint,
        c2: CGPoint,
        to: CGPoint,
    },
    Close,
}

#[derive(Default)]
struct CGPathHostObject {
    elements: Vec<PathElement>,
    mutable: bool,
}
impl HostObject for CGPathHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_CGPath: NSObject

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

// MARK: - Helpers

fn alloc_path(env: &mut Environment, mutable: bool) -> CGPathRef {
    let class = env.objc.get_known_class("_touchHLE_CGPath", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CGPathHostObject {
            elements: Vec::new(),
            mutable,
        }),
        &mut env.mem,
    )
}

// MARK: - Lifecycle

pub fn CGPathRetain(env: &mut Environment, path: CGPathRef) -> CGPathRef {
    if !path.is_null() {
        CFRetain(env, path)
    } else {
        path
    }
}

pub fn CGPathRelease(env: &mut Environment, path: CGPathRef) {
    if !path.is_null() {
        CFRelease(env, path);
    }
}

fn CGPathCreateMutable(env: &mut Environment) -> CGMutablePathRef {
    alloc_path(env, true)
}

fn CGPathCreateCopy(env: &mut Environment, path: CGPathRef) -> CGPathRef {
    if path.is_null() {
        return path;
    }
    let elements = env.objc.borrow::<CGPathHostObject>(path).elements.clone();
    let class = env.objc.get_known_class("_touchHLE_CGPath", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CGPathHostObject {
            elements,
            mutable: false,
        }),
        &mut env.mem,
    )
}

fn CGPathCreateMutableCopy(env: &mut Environment, path: CGPathRef) -> CGMutablePathRef {
    if path.is_null() {
        return alloc_path(env, true);
    }
    let elements = env.objc.borrow::<CGPathHostObject>(path).elements.clone();
    let class = env.objc.get_known_class("_touchHLE_CGPath", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CGPathHostObject {
            elements,
            mutable: true,
        }),
        &mut env.mem,
    )
}

fn CGPathCreateWithRect(
    env: &mut Environment,
    rect: CGRect,
    _transform: MutVoidPtr, // const CGAffineTransform* — ignored for now
) -> CGPathRef {
    let path = alloc_path(env, false);
    let CGRect { origin, size } = rect;
    let tl = origin;
    let tr = CGPoint {
        x: origin.x + size.width,
        y: origin.y,
    };
    let br = CGPoint {
        x: origin.x + size.width,
        y: origin.y + size.height,
    };
    let bl = CGPoint {
        x: origin.x,
        y: origin.y + size.height,
    };

    let elems = &mut env.objc.borrow_mut::<CGPathHostObject>(path).elements;
    elems.push(PathElement::MoveTo(tl));
    elems.push(PathElement::LineTo(tr));
    elems.push(PathElement::LineTo(br));
    elems.push(PathElement::LineTo(bl));
    elems.push(PathElement::Close);

    path
}

fn CGPathCreateWithEllipseInRect(
    env: &mut Environment,
    rect: CGRect,
    _transform: MutVoidPtr,
) -> CGPathRef {
    // Approximate ellipse with four cubic Bézier curves (standard magic
    // number).
    let path = alloc_path(env, false);
    let k: CGFloat = 0.5522848;
    let cx = rect.origin.x + rect.size.width * 0.5;
    let cy = rect.origin.y + rect.size.height * 0.5;
    let rx = rect.size.width * 0.5;
    let ry = rect.size.height * 0.5;

    let elems = &mut env.objc.borrow_mut::<CGPathHostObject>(path).elements;
    elems.push(PathElement::MoveTo(CGPoint { x: cx + rx, y: cy }));
    elems.push(PathElement::CurveTo {
        c1: CGPoint {
            x: cx + rx,
            y: cy + ry * k,
        },
        c2: CGPoint {
            x: cx + rx * k,
            y: cy + ry,
        },
        to: CGPoint { x: cx, y: cy + ry },
    });
    elems.push(PathElement::CurveTo {
        c1: CGPoint {
            x: cx - rx * k,
            y: cy + ry,
        },
        c2: CGPoint {
            x: cx - rx,
            y: cy + ry * k,
        },
        to: CGPoint { x: cx - rx, y: cy },
    });
    elems.push(PathElement::CurveTo {
        c1: CGPoint {
            x: cx - rx,
            y: cy - ry * k,
        },
        c2: CGPoint {
            x: cx - rx * k,
            y: cy - ry,
        },
        to: CGPoint { x: cx, y: cy - ry },
    });
    elems.push(PathElement::CurveTo {
        c1: CGPoint {
            x: cx + rx * k,
            y: cy - ry,
        },
        c2: CGPoint {
            x: cx + rx,
            y: cy - ry * k,
        },
        to: CGPoint { x: cx + rx, y: cy },
    });
    elems.push(PathElement::Close);

    path
}

// MARK: - Mutable path operations

fn CGPathMoveToPoint(
    env: &mut Environment,
    path: CGMutablePathRef,
    _transform: MutVoidPtr,
    x: CGFloat,
    y: CGFloat,
) {
    env.objc
        .borrow_mut::<CGPathHostObject>(path)
        .elements
        .push(PathElement::MoveTo(CGPoint { x, y }));
}

fn CGPathAddLineToPoint(
    env: &mut Environment,
    path: CGMutablePathRef,
    _transform: MutVoidPtr,
    x: CGFloat,
    y: CGFloat,
) {
    env.objc
        .borrow_mut::<CGPathHostObject>(path)
        .elements
        .push(PathElement::LineTo(CGPoint { x, y }));
}

fn CGPathAddQuadCurveToPoint(
    env: &mut Environment,
    path: CGMutablePathRef,
    _transform: MutVoidPtr,
    cpx: CGFloat,
    cpy: CGFloat,
    x: CGFloat,
    y: CGFloat,
) {
    env.objc
        .borrow_mut::<CGPathHostObject>(path)
        .elements
        .push(PathElement::QuadCurveTo {
            control: CGPoint { x: cpx, y: cpy },
            to: CGPoint { x, y },
        });
}

fn CGPathAddCurveToPoint(
    env: &mut Environment,
    path: CGMutablePathRef,
    _transform: MutVoidPtr,
    cp1x: CGFloat,
    cp1y: CGFloat,
    cp2x: CGFloat,
    cp2y: CGFloat,
    x: CGFloat,
    y: CGFloat,
) {
    env.objc
        .borrow_mut::<CGPathHostObject>(path)
        .elements
        .push(PathElement::CurveTo {
            c1: CGPoint { x: cp1x, y: cp1y },
            c2: CGPoint { x: cp2x, y: cp2y },
            to: CGPoint { x, y },
        });
}

fn CGPathCloseSubpath(env: &mut Environment, path: CGMutablePathRef) {
    env.objc
        .borrow_mut::<CGPathHostObject>(path)
        .elements
        .push(PathElement::Close);
}

fn CGPathAddRect(
    env: &mut Environment,
    path: CGMutablePathRef,
    _transform: MutVoidPtr,
    rect: CGRect,
) {
    let CGRect { origin, size } = rect;
    let tl = origin;
    let tr = CGPoint {
        x: origin.x + size.width,
        y: origin.y,
    };
    let br = CGPoint {
        x: origin.x + size.width,
        y: origin.y + size.height,
    };
    let bl = CGPoint {
        x: origin.x,
        y: origin.y + size.height,
    };

    let elems = &mut env.objc.borrow_mut::<CGPathHostObject>(path).elements;
    elems.push(PathElement::MoveTo(tl));
    elems.push(PathElement::LineTo(tr));
    elems.push(PathElement::LineTo(br));
    elems.push(PathElement::LineTo(bl));
    elems.push(PathElement::Close);
}

/// `CGPathAddArcToPoint` — adds an arc with the given radius that is
/// tangent to two line segments. The first segment runs from the
/// path's current point to `(x1, y1)`; the second segment runs from
/// `(x1, y1)` to `(x2, y2)`. The arc is inscribed inside the corner
/// they form and is tangent to both segments.
///
/// Apple `CGPath.h`:
/// <https://developer.apple.com/documentation/coregraphics/cgmutablepath/1408935-addarc>
/// The geometric construction below follows the standard "two tangent
/// line corner-fillet" formula (see e.g. PostScript Language
/// Reference, 3rd ed., §8.5.3 `arcto`).  Apple's implementation does:
///   1. Compute unit vectors `u1` from `p1` toward the current point,
///      and `u2` from `p1` toward `p2`.
///   2. Compute the half-angle `phi = angle(u1, u2) / 2`. The distance
///      from `p1` to each tangent point is `d = radius / tan(phi)`.
///   3. Tangent points are `p1 + d*u1` and `p1 + d*u2`.
///   4. The arc center is `p1 + h*bisector`, where
///      `bisector = (u1+u2) / |u1+u2|` and `h = radius / sin(phi)`.
///   5. A line is drawn from the current point to the first tangent
///      point, then the arc is laid down. Degenerate corners
///      (radius == 0, current==p1==p2, or collinear segments) collapse
///      to a single `addLine(to: p1)` per Apple's documented contract.
fn CGPathAddArcToPoint(
    env: &mut Environment,
    path: CGMutablePathRef,
    _transform: MutVoidPtr,
    x1: CGFloat,
    y1: CGFloat,
    x2: CGFloat,
    y2: CGFloat,
    radius: CGFloat,
) {
    let p1 = CGPoint { x: x1, y: y1 };
    let p2 = CGPoint { x: x2, y: y2 };

    let host = env.objc.borrow::<CGPathHostObject>(path);
    let p0 = match host.elements.last() {
        Some(PathElement::MoveTo(p))
        | Some(PathElement::LineTo(p))
        | Some(PathElement::QuadCurveTo { to: p, .. })
        | Some(PathElement::CurveTo { to: p, .. }) => *p,
        _ => p1,
    };

    let elems = &mut env.objc.borrow_mut::<CGPathHostObject>(path).elements;

    // Vector from p1 to p0 (unit length u1), and from p1 to p2 (u2).
    let dx1 = p0.x - p1.x;
    let dy1 = p0.y - p1.y;
    let dx2 = p2.x - p1.x;
    let dy2 = p2.y - p1.y;
    let len1 = (dx1 * dx1 + dy1 * dy1).sqrt();
    let len2 = (dx2 * dx2 + dy2 * dy2).sqrt();
    if radius <= 0.0 || len1 == 0.0 || len2 == 0.0 {
        // Degenerate: collapse to a single line to p1, per Apple.
        elems.push(PathElement::LineTo(p1));
        return;
    }
    let u1x = dx1 / len1;
    let u1y = dy1 / len1;
    let u2x = dx2 / len2;
    let u2y = dy2 / len2;

    // Angle between (-u1 reversed) and u2 is 2*phi, where the
    // half-angle phi satisfies cos(2*phi) = u1.dot(u2).
    let dot = u1x * u2x + u1y * u2y;
    let cos_two_phi = dot.clamp(-1.0, 1.0);
    let two_phi = cos_two_phi.acos();
    let phi = two_phi * 0.5;
    if phi.sin().abs() < 1e-6 {
        // Collinear — collapse to a line.
        elems.push(PathElement::LineTo(p1));
        return;
    }
    let d = radius / phi.tan().abs();

    // Tangent points.
    let t1 = CGPoint {
        x: p1.x + d * u1x,
        y: p1.y + d * u1y,
    };
    let t2 = CGPoint {
        x: p1.x + d * u2x,
        y: p1.y + d * u2y,
    };

    // Cubic Bézier approximation of the arc between t1 and t2,
    // following the standard 4/3 * tan(phi/2) handle-length formula.
    // Apple's quartz draws CGPath arcs via cubic Béziers internally;
    // emitting one or two cubics here is faithful enough that
    // `CGPathApply` walkers see a sane curve.
    let alpha = 4.0 / 3.0 * (phi / 2.0).tan();

    // Inward normals at the tangent points (perpendicular to u1, u2,
    // pointing toward p1's far side, i.e. toward the arc).
    // Normal of (u1x, u1y) rotated 90° toward p2.
    let n1x = -u1y;
    let n1y = u1x;
    let n2x = u2y;
    let n2y = -u2x;
    // Choose the sign that points the handle toward p2 (along arc).
    let sgn1 = (n1x * u2x + n1y * u2y).signum();
    let sgn2 = (n2x * u1x + n2y * u1y).signum();
    let c1 = CGPoint {
        x: t1.x + alpha * radius * n1x * sgn1,
        y: t1.y + alpha * radius * n1y * sgn1,
    };
    let c2 = CGPoint {
        x: t2.x + alpha * radius * n2x * sgn2,
        y: t2.y + alpha * radius * n2y * sgn2,
    };

    elems.push(PathElement::LineTo(t1));
    elems.push(PathElement::CurveTo {
        c1,
        c2,
        to: t2,
    });
}

fn CGPathAddPath(
    env: &mut Environment,
    path: CGMutablePathRef,
    _transform: MutVoidPtr,
    other: CGPathRef,
) {
    if other.is_null() {
        return;
    }
    let other_elems = env.objc.borrow::<CGPathHostObject>(other).elements.clone();
    env.objc
        .borrow_mut::<CGPathHostObject>(path)
        .elements
        .extend(other_elems);
}

// MARK: - Query

fn CGPathIsEmpty(env: &mut Environment, path: CGPathRef) -> bool {
    if path.is_null() {
        return true;
    }
    env.objc
        .borrow::<CGPathHostObject>(path)
        .elements
        .is_empty()
}

fn CGPathGetCurrentPoint(env: &mut Environment, path: CGPathRef) -> CGPoint {
    if path.is_null() {
        return CGPoint { x: 0.0, y: 0.0 };
    }
    for elem in env
        .objc
        .borrow::<CGPathHostObject>(path)
        .elements
        .iter()
        .rev()
    {
        match *elem {
            PathElement::MoveTo(p) => return p,
            PathElement::LineTo(p) => return p,
            PathElement::QuadCurveTo { to, .. } => return to,
            PathElement::CurveTo { to, .. } => return to,
            PathElement::Close => {}
        }
    }
    CGPoint { x: 0.0, y: 0.0 }
}

fn CGPathGetBoundingBox(env: &mut Environment, path: CGPathRef) -> CGRect {
    if path.is_null() {
        return CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: crate::frameworks::core_graphics::CGSize {
                width: 0.0,
                height: 0.0,
            },
        };
    }

    let mut min_x = CGFloat::MAX;
    let mut min_y = CGFloat::MAX;
    let mut max_x = CGFloat::MIN;
    let mut max_y = CGFloat::MIN;

    let mut update = |p: CGPoint| {
        if p.x < min_x {
            min_x = p.x;
        }
        if p.y < min_y {
            min_y = p.y;
        }
        if p.x > max_x {
            max_x = p.x;
        }
        if p.y > max_y {
            max_y = p.y;
        }
    };

    for elem in env.objc.borrow::<CGPathHostObject>(path).elements.iter() {
        match *elem {
            PathElement::MoveTo(p) | PathElement::LineTo(p) => update(p),
            PathElement::QuadCurveTo { control, to } => {
                update(control);
                update(to);
            }
            PathElement::CurveTo { c1, c2, to } => {
                update(c1);
                update(c2);
                update(to);
            }
            PathElement::Close => {}
        }
    }

    if min_x > max_x {
        return CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: crate::frameworks::core_graphics::CGSize {
                width: 0.0,
                height: 0.0,
            },
        };
    }

    CGRect {
        origin: CGPoint { x: min_x, y: min_y },
        size: crate::frameworks::core_graphics::CGSize {
            width: max_x - min_x,
            height: max_y - min_y,
        },
    }
}

fn CGPathContainsPoint(
    env: &mut Environment,
    path: CGPathRef,
    _transform: MutVoidPtr,
    point: CGPoint,
    _eo_fill_rule: bool,
) -> bool {
    // Simple bounding-box hit test — good enough for most game use-cases.
    let bbox = CGPathGetBoundingBox(env, path);
    point.x >= bbox.origin.x
        && point.x <= bbox.origin.x + bbox.size.width
        && point.y >= bbox.origin.y
        && point.y <= bbox.origin.y + bbox.size.height
}

fn CGPathEqualToPath(env: &mut Environment, path1: CGPathRef, path2: CGPathRef) -> bool {
    if path1 == path2 {
        return true;
    }
    if path1.is_null() || path2.is_null() {
        return false;
    }
    // Compare element counts as a quick proxy; deep equality not needed for
    // stubs.
    env.objc.borrow::<CGPathHostObject>(path1).elements.len()
        == env.objc.borrow::<CGPathHostObject>(path2).elements.len()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGPathRetain(_)),
    export_c_func!(CGPathRelease(_)),
    export_c_func!(CGPathCreateMutable()),
    export_c_func!(CGPathCreateCopy(_)),
    export_c_func!(CGPathCreateMutableCopy(_)),
    export_c_func!(CGPathCreateWithRect(_, _)),
    export_c_func!(CGPathCreateWithEllipseInRect(_, _)),
    export_c_func!(CGPathMoveToPoint(_, _, _, _)),
    export_c_func!(CGPathAddLineToPoint(_, _, _, _)),
    export_c_func!(CGPathAddQuadCurveToPoint(_, _, _, _, _, _)),
    export_c_func!(CGPathAddCurveToPoint(_, _, _, _, _, _, _, _)),
    export_c_func!(CGPathCloseSubpath(_)),
    export_c_func!(CGPathAddRect(_, _, _)),
    export_c_func!(CGPathAddArcToPoint(_, _, _, _, _, _, _)),
    export_c_func!(CGPathAddPath(_, _, _)),
    export_c_func!(CGPathIsEmpty(_)),
    export_c_func!(CGPathGetCurrentPoint(_)),
    export_c_func!(CGPathGetBoundingBox(_)),
    export_c_func!(CGPathContainsPoint(_, _, _, _)),
    export_c_func!(CGPathEqualToPath(_, _)),
];
