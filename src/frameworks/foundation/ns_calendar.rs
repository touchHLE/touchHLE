/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSCalendar`.
use crate::frameworks::foundation::ns_date_components::NSDateComponentsHostObject;
use crate::frameworks::foundation::{NSInteger, NSTimeInterval, NSUInteger};
use crate::objc::{autorelease, id, msg_class, nil, objc_classes, ClassExports, HostObject, NSZonePtr};
use crate::{msg, Environment};

/// Calendar unit for weekday (used by [NSCalendar components:fromDate:]).
pub type NSCalendarUnit = NSUInteger;
pub const NSWeekdayCalendarUnit: NSUInteger = 1 << 6;

#[derive(Default)]
pub struct State {
    current_calendar: Option<id>,
}
impl State {
    fn get(env: &mut Environment) -> &mut State {
        &mut env.framework_state.foundation.ns_calendar
    }
}

#[derive(Default)]
struct NSCalendarHostObject {
    first_weekday: NSInteger,
}
impl HostObject for NSCalendarHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSCalendar: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<NSCalendarHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)currentCalendar {
    if let Some(calendar) = State::get(env).current_calendar {
        calendar
    } else {
        let new_calendar: id = msg![env; this alloc];
        let new_calendar: id = msg![env; new_calendar init];
        State::get(env).current_calendar = Some(new_calendar);
        new_calendar
    }
}

- (id)init {
    this
}

- (NSInteger)firstWeekday {
    env.objc.borrow::<NSCalendarHostObject>(this).first_weekday
}

- (())setFirstWeekday:(NSInteger)weekday {
    env.objc.borrow_mut::<NSCalendarHostObject>(this).first_weekday = weekday;
}

- (id)components:(NSCalendarUnit)unit_flags fromDate:(id)date { // NSDate *
    if date == nil {
        return nil;
    }
    let time_interval: NSTimeInterval = msg![env; date timeIntervalSinceReferenceDate];
    // Weekday: Apple epoch (Jan 1, 2001 00:00:00 GMT) was a Monday (2 in 1=Sun..7=Sat).
    let days_since_epoch = (time_interval / 86400.0).floor() as i64;
    let weekday = ((2 + days_since_epoch - 1).rem_euclid(7) + 1) as NSInteger;

    let components: id = msg_class![env; NSDateComponents new];
    if (unit_flags & NSWeekdayCalendarUnit) != 0 {
        env.objc.borrow_mut::<NSDateComponentsHostObject>(components).weekday = weekday;
    }
    autorelease(env, components)
}

@end

};
