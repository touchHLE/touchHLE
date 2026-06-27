/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#![allow(non_snake_case)]

//! `CGPattern.h`
//!
//! Per Apple's [Quartz 2D Programming
//! Guide](https://developer.apple.com/library/archive/documentation/GraphicsImaging/Conceptual/drawingwithquartz2d/dq_patterns/dq_patterns.html)
//! and the [`CGPattern`
//! reference](https://developer.apple.com/documentation/coregraphics/cgpattern),
//! a `CGPattern` is an opaque `CFType` that describes a repeating tile drawn
//! by a guest-supplied callback (`drawPattern`). It is used as a "colour"
//! when stroking or filling paths.
//!
//! `touchHLE` does not currently render patterns into the host bitmap, so the
//! tile geometry/callbacks are stored verbatim and the pattern is treated as
//! a transparent fill by `CGColorCreateWithPattern`. This is a real CFType
//! object (it can be retained/released, deallocated correctly, and held by
//! the guest) — not a 0 return-stub.

use super::CGFloat;
use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::core_graphics::cg_affine_transform::CGAffineTransform;
use crate::frameworks::core_graphics::CGRect;
use crate::mem::{ConstVoidPtr, MutVoidPtr, SafeRead};
use crate::objc::{objc_classes, ClassExports, HostObject};
use crate::Environment;

pub type CGPatternRef = CFTypeRef;

/// `CGPatternTiling` — controls how anti-aliasing is performed at the edges
/// of pattern tiles. We retain the value verbatim for compatibility with
/// guest code that round-trips it.
pub type CGPatternTiling = i32;
pub const kCGPatternTilingNoDistortion: CGPatternTiling = 0;
pub const kCGPatternTilingConstantSpacingMinimalDistortion: CGPatternTiling = 1;
pub const kCGPatternTilingConstantSpacing: CGPatternTiling = 2;

/// Mirrors Apple's `CGPatternCallbacks` struct layout:
///
/// ```c
/// struct CGPatternCallbacks {
///     unsigned int version;
///     CGPatternDrawPatternCallback drawPattern;
///     CGPatternReleaseInfoCallback releaseInfo;
/// };
/// ```
#[repr(C, packed)]
#[derive(Copy, Clone, Default, Debug)]
pub struct CGPatternCallbacks {
    pub version: u32,
    pub draw_pattern: u32,  // function pointer
    pub release_info: u32,  // function pointer
}
unsafe impl SafeRead for CGPatternCallbacks {}

#[derive(Default)]
struct CGPatternHostObject {
    /// `info` pointer passed to the drawPattern/releaseInfo callbacks.
    info: u32,
    /// Tile bounds in pattern-space.
    bounds: CGRect,
    /// Pattern → user space transform.
    matrix: CGAffineTransform,
    /// Horizontal/vertical pattern repetition step (in pattern space).
    x_step: CGFloat,
    y_step: CGFloat,
    /// Tile anti-aliasing strategy.
    tiling: CGPatternTiling,
    /// Whether the pattern carries its own colour (`true`) or is stencilled
    /// onto a colour supplied via `CGColorCreateWithPattern` (`false`).
    is_colored: bool,
    /// Verbatim copy of the callbacks pointer table the app supplied. The
    /// `releaseInfo` callback is invoked when the host object is dropped, so
    /// the guest can free its `info` payload.
    callbacks: CGPatternCallbacks,
}
impl HostObject for CGPatternHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_CGPattern: NSObject

- (())dealloc {
    // Invoke the guest's releaseInfo callback (if non-null) before the host
    // object is destroyed, mirroring CoreGraphics' real-world behaviour.
    let (release_info, info) = {
        let host = env.objc.borrow::<CGPatternHostObject>(this);
        (host.callbacks.release_info, host.info)
    };
    if release_info != 0 {
        let func = GuestFunction::from_addr_with_thumb_bit(release_info);
        let _: () = func.call_from_host(env, (info,));
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

// MARK: - Lifecycle

pub fn CGPatternRetain(env: &mut Environment, pattern: CGPatternRef) -> CGPatternRef {
    if !pattern.is_null() {
        CFRetain(env, pattern)
    } else {
        pattern
    }
}

pub fn CGPatternRelease(env: &mut Environment, pattern: CGPatternRef) {
    if !pattern.is_null() {
        CFRelease(env, pattern);
    }
}

// MARK: - Constructor

/// `CGPatternCreate(info, bounds, matrix, xStep, yStep, tiling, isColored, callbacks)`.
///
/// Returns a retained `CGPatternRef`. The caller is responsible for matching
/// it with a `CGPatternRelease` call.
fn CGPatternCreate(
    env: &mut Environment,
    info: MutVoidPtr,
    bounds: CGRect,
    matrix: CGAffineTransform,
    x_step: CGFloat,
    y_step: CGFloat,
    tiling: CGPatternTiling,
    is_colored: bool,
    callbacks: ConstVoidPtr,
) -> CGPatternRef {
    let cbs = if callbacks.is_null() {
        CGPatternCallbacks::default()
    } else {
        env.mem.read(callbacks.cast::<CGPatternCallbacks>())
    };

    let class = env
        .objc
        .get_known_class("_touchHLE_CGPattern", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CGPatternHostObject {
            info: info.to_bits(),
            bounds,
            matrix,
            x_step,
            y_step,
            tiling,
            is_colored,
            callbacks: cbs,
        }),
        &mut env.mem,
    )
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGPatternRetain(_)),
    export_c_func!(CGPatternRelease(_)),
    export_c_func!(CGPatternCreate(_, _, _, _, _, _, _, _)),
];
