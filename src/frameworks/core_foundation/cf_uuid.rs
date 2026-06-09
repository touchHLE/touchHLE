/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFUUID`.

use super::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use super::cf_string::CFStringRef;
use super::CFTypeRef;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::ns_string::from_rust_string;
use crate::objc::{objc_classes, ClassExports, HostObject};
use crate::Environment;
use uuid::Uuid;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// CFUUID doesn't have a corresponding NS type (at least, not up until iOS 6+
// and even that one is _not_ toll-free bridged, see NSUUID docs),
// but the callers of CFUUIDCreate() are expected to call CFRelease() on them.
@implementation _touchHLE_CFUUID: NSObject
@end

};

/// Note: Apple is using a pointer to an opaque struct instead
type CFUUIDRef = CFTypeRef;

struct CFUUIDHostObject {
    uuid: Uuid,
}
impl HostObject for CFUUIDHostObject {}

fn CFUUIDCreate(env: &mut Environment, allocator: CFAllocatorRef) -> CFUUIDRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default()); // unimplemented

    let host_obj = Box::new(CFUUIDHostObject {
        uuid: Uuid::new_v4(),
    });
    let class = env.objc.get_known_class("_touchHLE_CFUUID", &mut env.mem);
    env.objc.alloc_object(class, host_obj, &mut env.mem)
}

fn CFUUIDCreateString(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    uuid: CFUUIDRef,
) -> CFStringRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default()); // unimplemented

    let host_object = env.objc.borrow::<CFUUIDHostObject>(uuid);
    let uuid_str = host_object.uuid.hyphenated().to_string().to_uppercase();
    from_rust_string(env, uuid_str)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFUUIDCreate(_)),
    export_c_func!(CFUUIDCreateString(_, _)),
];
