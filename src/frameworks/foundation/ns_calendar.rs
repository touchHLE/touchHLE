/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSCalendar` and `NSDateComponents`.

use crate::frameworks::core_foundation::time::CFAbsoluteTimeGetGregorianDate;
use crate::frameworks::foundation::{ns_date, NSInteger, NSUInteger};
use crate::objc::{id, msg_class, nil, objc_classes, Class, ClassExports, HostObject, NSZonePtr};

struct NSCalendarHostObject;
impl HostObject for NSCalendarHostObject {}

struct NSDateComponentsHostObject {
    year: NSInteger,
    month: NSInteger,
    day: NSInteger,
}
impl HostObject for NSDateComponentsHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSCalendar: NSObject

+ (id)currentCalendar {
    env.objc.alloc_static_object(
        this,
        Box::new(NSCalendarHostObject),
        &mut env.mem
    )
}

- (id)components:(NSUInteger)_unit_flags fromDate:(id)date {
    let time_interval = env.objc.borrow::<ns_date::NSDateHostObject>(date).time_interval;
    let greg_date = CFAbsoluteTimeGetGregorianDate(env, time_interval, nil);
    let components_class: Class = msg_class![env; NSDateComponents class];
    env.objc.alloc_object(
        components_class,
        Box::new(NSDateComponentsHostObject {
            year: greg_date.year,
            month: greg_date.month as NSInteger,
            day: greg_date.day as NSInteger,
        }),
        &mut env.mem
    )
}

@end

@implementation NSDateComponents: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(
        this,
        Box::new(NSDateComponentsHostObject {
            year: 0,
            month: 0,
            day: 0,
        }),
        &mut env.mem
    )
}

- (NSInteger)year {
    env.objc.borrow::<NSDateComponentsHostObject>(this).year
}

- (NSInteger)month {
    env.objc.borrow::<NSDateComponentsHostObject>(this).month
}

- (NSInteger)day {
    env.objc.borrow::<NSDateComponentsHostObject>(this).day
}

@end

};
