//! `UIGeometry.h`
//!
//! See also [crate::frameworks::core_graphics::cg_geometry].

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string;
use crate::mem::{ConstVoidPtr, MutVoidPtr, Ptr};
use crate::objc::{autorelease, id};
use crate::Environment;

/// Allocate a 16-byte block of zeros and return a pointer to it. This matches
/// the in-memory layout of `UIEdgeInsets { top, left, bottom, right }` where
/// each field is a 4-byte CGFloat. The block is leaked once at first lookup,
/// matching how Apple's UIKit exposes it.
fn ui_edge_insets_zero(env: &mut Environment) -> ConstVoidPtr {
    let ptr = env.mem.alloc(16);
    // Memory from `alloc` is uninitialized; explicitly zero it.
    for i in 0..16 {
        env.mem.write(ptr.cast::<u8>() + i, 0u8);
    }
    Ptr::from_bits(ptr.to_bits())
}

#[derive(Default)]
pub struct State {
    pub default_malloc_zone: Option<MutVoidPtr>,
}

// Apple's documentation says all of these return zeroes if the input is not
// well-formed.
pub fn CGPointFromString(env: &mut Environment, string: id) -> CGPoint {
    // TODO: avoid copy
    ns_string::to_rust_string(env, string)
        .parse()
        .unwrap_or_default()
}
pub fn CGSizeFromString(env: &mut Environment, string: id) -> CGSize {
    // TODO: avoid copy
    ns_string::to_rust_string(env, string)
        .parse()
        .unwrap_or_default()
}
pub fn CGRectFromString(env: &mut Environment, string: id) -> CGRect {
    // TODO: avoid copy
    ns_string::to_rust_string(env, string)
        .parse()
        .unwrap_or_default()
}

pub fn NSStringFromCGPoint(env: &mut Environment, point: CGPoint) -> id {
    let s = ns_string::from_rust_string(env, point.to_string());
    autorelease(env, s)
}
pub fn NSStringFromCGSize(env: &mut Environment, size: CGSize) -> id {
    let s = ns_string::from_rust_string(env, size.to_string());
    autorelease(env, s)
}
pub fn NSStringFromCGRect(env: &mut Environment, rect: CGRect) -> id {
    let s = ns_string::from_rust_string(env, rect.to_string());
    autorelease(env, s)
}

fn NSDefaultMallocZone(env: &mut Environment) -> MutVoidPtr {
    if let Some(ptr) = env.framework_state.uikit.ui_geometry.default_malloc_zone {
        return ptr;
    }
    let ptr = env.mem.alloc(4);
    env.framework_state.uikit.ui_geometry.default_malloc_zone = Some(ptr);
    log_dbg!("NSDefaultMallocZone: allocated dummy zone at {:?}", ptr);
    ptr
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGPointFromString(_)),
    export_c_func!(CGSizeFromString(_)),
    export_c_func!(CGRectFromString(_)),
    export_c_func!(NSStringFromCGPoint(_)),
    export_c_func!(NSStringFromCGSize(_)),
    export_c_func!(NSStringFromCGRect(_)),
    export_c_func!(NSDefaultMallocZone()),
];

pub const CONSTANTS: ConstantExports = &[(
    "_UIEdgeInsetsZero",
    HostConstant::Custom(ui_edge_insets_zero),
)];
