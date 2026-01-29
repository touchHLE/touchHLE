/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSDateComponents`.

use super::NSInteger;
use crate::objc::{id, objc_classes, ClassExports, HostObject, NSZonePtr};

pub(super) struct NSDateComponentsHostObject {
    pub(super) year: Option<NSInteger>,
    pub(super) month: Option<NSInteger>,
    pub(super) day: Option<NSInteger>,
    pub(super) hour: Option<NSInteger>,
    pub(super) minute: Option<NSInteger>,
    pub(super) second: Option<NSInteger>,
    pub(super) weekday: Option<NSInteger>,
}
impl HostObject for NSDateComponentsHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSDateComponents: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSDateComponentsHostObject {
        year: None,
        month: None,
        day: None,
        hour: None,
        minute: None,
        second: None,
        weekday: None,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (NSInteger)year {
    env.objc.borrow::<NSDateComponentsHostObject>(this).year.unwrap_or(0)
}

- (NSInteger)month {
    env.objc.borrow::<NSDateComponentsHostObject>(this).month.unwrap_or(0)
}

- (NSInteger)day {
    env.objc.borrow::<NSDateComponentsHostObject>(this).day.unwrap_or(0)
}

- (NSInteger)hour {
    env.objc.borrow::<NSDateComponentsHostObject>(this).hour.unwrap_or(0)
}

- (NSInteger)minute {
    env.objc.borrow::<NSDateComponentsHostObject>(this).minute.unwrap_or(0)
}

- (NSInteger)second {
    env.objc.borrow::<NSDateComponentsHostObject>(this).second.unwrap_or(0)
}

- (NSInteger)weekday {
    env.objc.borrow::<NSDateComponentsHostObject>(this).weekday.unwrap_or(0)
}


@end

};
