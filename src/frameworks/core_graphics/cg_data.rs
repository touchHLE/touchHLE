/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */


use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::mem::{MutVoidPtr, ConstVoidPtr, GuestUSize};
use crate::objc::{objc_classes, ClassExports, HostObject};
use crate::Environment;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// CGDataProvider seems to be a CFType-based type, but in our implementation
// those are just Objective-C types, so we need a class for it, but its name is
// not visible anywhere.
@implementation _touchHLE_CGDataProvider: NSObject
@end

};

pub(super) struct CGDataProviderHostObject {
    pub(super) data: ConstVoidPtr,
    pub(super) size: GuestUSize,
}
impl HostObject for CGDataProviderHostObject {}

pub type CGDataProviderRef = CFTypeRef;

pub fn CGDataProviderCreateWithData(env: &mut Environment, info: MutVoidPtr, data: ConstVoidPtr, size: GuestUSize, release: ConstVoidPtr) -> CGDataProviderRef {
    assert!(info.is_null());
    assert!(release.is_null());
    let isa = env
    .objc
    .get_known_class("_touchHLE_CGDataProvider", &mut env.mem);
    log!("CGDataProviderCreateWithData: info: {:?}, data: {:?}, size: {:?}, release: {:?}",
        info, data, size, release);
    env.objc.alloc_object(
        isa,
        Box::new(CGDataProviderHostObject{
            data: data,
            size: size,
        }),
        &mut env.mem,
    )
}

pub fn CGDataProviderRelease(env: &mut Environment, cs: CGDataProviderRef) {
    if !cs.is_null() {
        CFRelease(env, cs);
    }
}
pub fn CGDataProviderRetain(env: &mut Environment, cs: CGDataProviderRef) -> CGDataProviderRef {
    if !cs.is_null() {
        CFRetain(env, cs)
    } else {
        cs
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGDataProviderCreateWithData(_, _, _, _)),
    export_c_func!(CGDataProviderRelease(_)),
    export_c_func!(CGDataProviderRetain(_)),
];
