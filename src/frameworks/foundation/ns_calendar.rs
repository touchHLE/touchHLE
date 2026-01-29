/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSCalendar`.

use super::{
    ns_date_components::NSDateComponentsHostObject, NSInteger, NSCalendarUnit,
    NSYearCalendarUnit, NSMonthCalendarUnit, NSDayCalendarUnit, NSHourCalendarUnit,
    NSMinuteCalendarUnit, NSSecondCalendarUnit, NSWeekdayCalendarUnit, NSTimeInterval,
};
use crate::frameworks::core_foundation::time::CFAbsoluteTimeGetGregorianDate;
use crate::objc::{autorelease, id, msg, msg_class, objc_classes, ClassExports, HostObject, NSZonePtr};

struct NSCalendarHostObject {
    first_weekday: NSInteger,
}
impl HostObject for NSCalendarHostObject {}

#[derive(Default)]
pub struct State {
    current_calendar: Option<id>,
}
impl State {
    fn get(env: &mut crate::Environment) -> &mut State {
        &mut env.framework_state.foundation.ns_calendar
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSCalendar: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSCalendarHostObject {
        first_weekday: 1,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)currentCalendar {
    if let Some(existing) = State::get(env).current_calendar {
        existing
    } else {
        let new: id = msg![env; this new];
        State::get(env).current_calendar = Some(new);
        new
    }
}

- (NSInteger)firstWeekday {
    env.objc.borrow::<NSCalendarHostObject>(this).first_weekday
}

- (())setFirstWeekday:(NSInteger)weekday {
    env.objc.borrow_mut::<NSCalendarHostObject>(this).first_weekday = weekday;
}

- (id)components:(NSCalendarUnit)unitFlags
    fromDate:(id)date { // NSDate *
    let time_interval: NSTimeInterval = msg![env; date timeIntervalSinceReferenceDate];
    
    let greg_date = CFAbsoluteTimeGetGregorianDate(env, time_interval, crate::objc::nil);
    
    // Calculate weekday: Apple epoch (Jan 1, 2001 00:00:00 GMT) was a Monday
    // NSDateComponents weekday: 1 = Sunday, 2 = Monday, ..., 7 = Saturday
    let days_since_epoch = (time_interval / 86400.0).floor() as i64;
    // Calculate: (Monday(2) + days_since_epoch) % 7, but adjust for 1-based indexing
    let weekday = ((2 + days_since_epoch - 1) % 7) + 1;
    let weekday = if weekday <= 0 { weekday + 7 } else { weekday } as NSInteger;
    
    // Create NSDateComponents object
    let components: id = msg_class![env; NSDateComponents new];
    let comp_host = env.objc.borrow_mut::<NSDateComponentsHostObject>(components);
    
    // Set components based on requested unit flags
    if (unitFlags & NSYearCalendarUnit) != 0 {
        comp_host.year = Some(greg_date.year as NSInteger);
    }
    if (unitFlags & NSMonthCalendarUnit) != 0 {
        comp_host.month = Some(greg_date.month as NSInteger);
    }
    if (unitFlags & NSDayCalendarUnit) != 0 {
        comp_host.day = Some(greg_date.day as NSInteger);
    }
    if (unitFlags & NSHourCalendarUnit) != 0 {
        comp_host.hour = Some(greg_date.hours as NSInteger);
    }
    if (unitFlags & NSMinuteCalendarUnit) != 0 {
        comp_host.minute = Some(greg_date.minutes as NSInteger);
    }
    if (unitFlags & NSSecondCalendarUnit) != 0 {
        comp_host.second = Some(greg_date.seconds as NSInteger);
    }
    if (unitFlags & NSWeekdayCalendarUnit) != 0 {
        comp_host.weekday = Some(weekday);
    }
    
    autorelease(env, components)
}

@end

};
