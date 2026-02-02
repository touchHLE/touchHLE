/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSDateComponents`.
use crate::frameworks::foundation::NSInteger;
use crate::objc::{id, objc_classes, ClassExports, HostObject, NSZonePtr};

/// Host object for NSDateComponents.
#[derive(Default)]
pub struct NSDateComponentsHostObject {
    /// Only the `weekday` component is implemented so far.
    pub weekday: NSInteger,
}
impl HostObject for NSDateComponentsHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSDateComponents: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<NSDateComponentsHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())setWeekday:(NSInteger)weekday {
    env.objc.borrow_mut::<NSDateComponentsHostObject>(this).weekday = weekday;
}
- (NSInteger)weekday {
    env.objc.borrow::<NSDateComponentsHostObject>(this).weekday
}

@end

};
