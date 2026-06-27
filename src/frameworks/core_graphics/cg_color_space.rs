/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGColorSpace.h`

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::core_foundation::cf_string::CFStringRef;
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::foundation::{ns_string, NSUInteger};
use crate::objc::{nil, objc_classes, ClassExports, HostObject};
use crate::Environment;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_CGColorSpace: NSObject
@end

};

pub type CGColorSpaceModel = i32;
#[allow(dead_code)]
pub const kCGColorSpaceModelUnknown: CGColorSpaceModel = -1;
pub const kCGColorSpaceModelMonochrome: CGColorSpaceModel = 0;
pub const kCGColorSpaceModelRGB: CGColorSpaceModel = 1;
#[allow(dead_code)]
pub const kCGColorSpaceModelCMYK: CGColorSpaceModel = 2;
#[allow(dead_code)]
pub const kCGColorSpaceModelLab: CGColorSpaceModel = 3;
#[allow(dead_code)]
pub const kCGColorSpaceModelDeviceN: CGColorSpaceModel = 4;
#[allow(dead_code)]
pub const kCGColorSpaceModelIndexed: CGColorSpaceModel = 5;
#[allow(dead_code)]
pub const kCGColorSpaceModelPattern: CGColorSpaceModel = 6;

#[derive(Default)]
pub(super) struct CGColorSpaceHostObject {
    pub(super) name: &'static str,
}
impl HostObject for CGColorSpaceHostObject {}

pub type CGColorSpaceRef = CFTypeRef;

// MARK: - Internal alloc helper

fn alloc_color_space(env: &mut Environment, name: &'static str) -> CGColorSpaceRef {
    let isa = env
        .objc
        .get_known_class("_touchHLE_CGColorSpace", &mut env.mem);
    env.objc
        .alloc_object(isa, Box::new(CGColorSpaceHostObject { name }), &mut env.mem)
}

// MARK: - Constructors

pub fn CGColorSpaceCreateWithName(env: &mut Environment, name: CFStringRef) -> CGColorSpaceRef {
    if name == nil {
        log!(
            "Warning: CGColorSpaceCreateWithName(NULL): NULL name is invalid per \
             Apple's CGColorSpace.h, returning NULL."
        );
        return nil;
    }

    // Compare by Rust string so we don't need a static CFString lookup per call
    // and can also use the value as a dedup-key for the warning.
    let name_str = ns_string::to_rust_string(env, name).into_owned();

    let canonical: &'static str = match name_str.as_str() {
        // RGB-family — all map to our GenericRGB pipeline. touchHLE does not
        // model wide-gamut / linear / Adobe RGB / Rec. 709 / 2020 separately;
        // the closest match in our renderer is the standard sRGB-ish path.
        s if s == kCGColorSpaceGenericRGB
            || s == kCGColorSpaceSRGB
            || s == kCGColorSpaceLinearSRGB
            || s == kCGColorSpaceGenericRGBLinear
            || s == kCGColorSpaceDisplayP3
            || s == kCGColorSpaceAdobeRGB1998
            || s == kCGColorSpaceITUR_709
            || s == kCGColorSpaceITUR_2020 =>
        {
            kCGColorSpaceGenericRGB
        }
        // Gray-family.
        s if s == kCGColorSpaceGenericGray
            || s == kCGColorSpaceLinearGray
            || s == kCGColorSpaceGenericGrayGamma2_2 =>
        {
            kCGColorSpaceGenericGray
        }
        s if s == kCGColorSpaceGenericCMYK => kCGColorSpaceGenericCMYK,
        _ => {
            warn_unknown_color_space_once(&name_str);
            kCGColorSpaceGenericRGB
        }
    };

    alloc_color_space(env, canonical)
}

/// Warn at most once per unique color-space name. The same unknown name can be
/// passed to CGColorSpaceCreateWithName thousands of times per frame by some
/// apps; without dedup the log fills with hundreds of thousands of copies of
/// the same line and drowns out everything else useful.
fn warn_unknown_color_space_once(name: &str) {
    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let mut guard = seen.lock().unwrap();
    if guard.insert(name.to_string()) {
        log!(
            "Warning: CGColorSpaceCreateWithName: unknown color space {:?}, \
             falling back to GenericRGB (further occurrences of this name \
             will be silenced)",
            name
        );
    }
}

pub fn CGColorSpaceCreateDeviceRGB(env: &mut Environment) -> CGColorSpaceRef {
    alloc_color_space(env, kCGColorSpaceGenericRGB)
}

pub fn CGColorSpaceCreateDeviceGray(env: &mut Environment) -> CGColorSpaceRef {
    alloc_color_space(env, kCGColorSpaceGenericGray)
}

fn CGColorSpaceCreateDeviceCMYK(env: &mut Environment) -> CGColorSpaceRef {
    alloc_color_space(env, kCGColorSpaceGenericCMYK)
}

fn CGColorSpaceCreateWithICCProfile(
    env: &mut Environment,
    _data: crate::objc::id, // CFDataRef
) -> CGColorSpaceRef {
    log!(
        "Warning: CGColorSpaceCreateWithICCProfile: ICC profiles not supported, \
          falling back to GenericRGB"
    );
    alloc_color_space(env, kCGColorSpaceGenericRGB)
}

fn CGColorSpaceCreateICCBased(
    env: &mut Environment,
    n_components: NSUInteger,
    _range: crate::mem::ConstPtr<crate::frameworks::core_graphics::CGFloat>,
    _profile: CFTypeRef, // CGDataProviderRef
    _alternate: CGColorSpaceRef,
) -> CGColorSpaceRef {
    log!(
        "Warning: CGColorSpaceCreateICCBased: ICC profiles not supported \
         (nComponents={}), falling back",
        n_components
    );
    match n_components {
        1 => alloc_color_space(env, kCGColorSpaceGenericGray),
        4 => alloc_color_space(env, kCGColorSpaceGenericCMYK),
        _ => alloc_color_space(env, kCGColorSpaceGenericRGB),
    }
}

fn CGColorSpaceCreateIndexed(
    env: &mut Environment,
    _base_space: CGColorSpaceRef,
    _last_index: NSUInteger,
    _color_table: crate::mem::ConstPtr<u8>,
) -> CGColorSpaceRef {
    log!(
        "Warning: CGColorSpaceCreateIndexed: indexed color spaces not supported, \
          falling back to GenericRGB"
    );
    alloc_color_space(env, kCGColorSpaceGenericRGB)
}

fn CGColorSpaceCreatePattern(
    env: &mut Environment,
    _base_space: CGColorSpaceRef,
) -> CGColorSpaceRef {
    log!(
        "Warning: CGColorSpaceCreatePattern: pattern color spaces not supported, \
          falling back to GenericRGB"
    );
    alloc_color_space(env, kCGColorSpaceGenericRGB)
}

// MARK: - Retain / Release

pub fn CGColorSpaceRelease(env: &mut Environment, cs: CGColorSpaceRef) {
    if !cs.is_null() {
        CFRelease(env, cs);
    }
}

pub fn CGColorSpaceRetain(env: &mut Environment, cs: CGColorSpaceRef) -> CGColorSpaceRef {
    if !cs.is_null() {
        CFRetain(env, cs)
    } else {
        cs
    }
}

// MARK: - Accessors

pub fn CGColorSpaceGetModel(env: &mut Environment, cs: CGColorSpaceRef) -> CGColorSpaceModel {
    if cs.is_null() {
        return kCGColorSpaceModelUnknown;
    }
    match env.objc.borrow::<CGColorSpaceHostObject>(cs).name {
        kCGColorSpaceGenericGray => kCGColorSpaceModelMonochrome,
        kCGColorSpaceGenericRGB => kCGColorSpaceModelRGB,
        kCGColorSpaceGenericCMYK => kCGColorSpaceModelCMYK,
        _ => kCGColorSpaceModelUnknown,
    }
}

/// Returns the number of colour components *excluding* alpha.
pub fn CGColorSpaceGetNumberOfComponents(env: &mut Environment, cs: CGColorSpaceRef) -> NSUInteger {
    if cs.is_null() {
        return 0;
    }
    match env.objc.borrow::<CGColorSpaceHostObject>(cs).name {
        kCGColorSpaceGenericGray => 1,
        kCGColorSpaceGenericRGB => 3,
        kCGColorSpaceGenericCMYK => 4,
        _ => 3,
    }
}

fn CGColorSpaceCopyName(env: &mut Environment, cs: CGColorSpaceRef) -> CFStringRef {
    if cs.is_null() {
        return nil;
    }
    let name = env.objc.borrow::<CGColorSpaceHostObject>(cs).name;
    let ns = ns_string::from_rust_string(env, name.to_string());
    // CFStringRef is toll-free bridged with NSString; autorelease so caller
    // gets a +0 reference (matching real CG behaviour of "Copy" returning +1,
    // but apps usually don't release it — autorelease is the safe middle
    // ground).
    crate::objc::autorelease(env, ns)
}

fn CGColorSpaceIsWideGamutRGB(_env: &mut Environment, _cs: CGColorSpaceRef) -> bool {
    // touchHLE only models sRGB-equivalent spaces — never wide gamut.
    false
}

fn CGColorSpaceSupportsOutput(_env: &mut Environment, cs: CGColorSpaceRef) -> bool {
    !cs.is_null()
}

// MARK: - Constants

pub const kCGColorSpaceGenericRGB: &str = "kCGColorSpaceGenericRGB";
pub const kCGColorSpaceGenericGray: &str = "kCGColorSpaceGenericGray";
pub const kCGColorSpaceGenericCMYK: &str = "kCGColorSpaceGenericCMYK";
pub const kCGColorSpaceSRGB: &str = "kCGColorSpaceSRGB";
pub const kCGColorSpaceLinearSRGB: &str = "kCGColorSpaceLinearSRGB";
pub const kCGColorSpaceLinearGray: &str = "kCGColorSpaceLinearGray";
// Additional CGColorSpace name constants from <CoreGraphics/CGColorSpace.h>.
// These are passed by real-world apps but were not handled previously,
// triggering hundreds of thousands of "unknown color space" warnings per
// frame. We canonicalize them to the closest of our three internal models in
// CGColorSpaceCreateWithName above.
pub const kCGColorSpaceGenericRGBLinear: &str = "kCGColorSpaceGenericRGBLinear";
pub const kCGColorSpaceGenericGrayGamma2_2: &str = "kCGColorSpaceGenericGrayGamma2_2";
pub const kCGColorSpaceDisplayP3: &str = "kCGColorSpaceDisplayP3";
pub const kCGColorSpaceAdobeRGB1998: &str = "kCGColorSpaceAdobeRGB1998";
#[allow(non_upper_case_globals)]
pub const kCGColorSpaceITUR_709: &str = "kCGColorSpaceITUR_709";
#[allow(non_upper_case_globals)]
pub const kCGColorSpaceITUR_2020: &str = "kCGColorSpaceITUR_2020";

pub const CONSTANTS: ConstantExports = &[
    (
        "_kCGColorSpaceGenericRGB",
        HostConstant::NSString(kCGColorSpaceGenericRGB),
    ),
    (
        "_kCGColorSpaceGenericGray",
        HostConstant::NSString(kCGColorSpaceGenericGray),
    ),
    (
        "_kCGColorSpaceGenericCMYK",
        HostConstant::NSString(kCGColorSpaceGenericCMYK),
    ),
    (
        "_kCGColorSpaceSRGB",
        HostConstant::NSString(kCGColorSpaceSRGB),
    ),
    (
        "_kCGColorSpaceLinearSRGB",
        HostConstant::NSString(kCGColorSpaceLinearSRGB),
    ),
    (
        "_kCGColorSpaceLinearGray",
        HostConstant::NSString(kCGColorSpaceLinearGray),
    ),
];

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGColorSpaceCreateWithName(_)),
    export_c_func!(CGColorSpaceCreateDeviceRGB()),
    export_c_func!(CGColorSpaceCreateDeviceGray()),
    export_c_func!(CGColorSpaceCreateDeviceCMYK()),
    export_c_func!(CGColorSpaceCreateWithICCProfile(_)),
    export_c_func!(CGColorSpaceCreateICCBased(_, _, _, _)),
    export_c_func!(CGColorSpaceCreateIndexed(_, _, _)),
    export_c_func!(CGColorSpaceCreatePattern(_)),
    export_c_func!(CGColorSpaceRetain(_)),
    export_c_func!(CGColorSpaceRelease(_)),
    export_c_func!(CGColorSpaceGetModel(_)),
    export_c_func!(CGColorSpaceGetNumberOfComponents(_)),
    export_c_func!(CGColorSpaceCopyName(_)),
    export_c_func!(CGColorSpaceIsWideGamutRGB(_)),
    export_c_func!(CGColorSpaceSupportsOutput(_)),
];
