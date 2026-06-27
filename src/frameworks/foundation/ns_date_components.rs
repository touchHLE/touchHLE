/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//!
//! `NSDateComponents`.

use crate::frameworks::foundation::NSInteger;
use crate::objc::{id, msg, objc_classes, ClassExports, HostObject, NSZonePtr};

/// Host object for NSDateComponents.
#[derive(Default)]
pub struct NSDateComponentsHostObject {
    pub era: NSInteger,
    pub year: NSInteger,
    pub month: NSInteger,
    pub week: NSInteger,
    pub day: NSInteger,
    pub hour: NSInteger,
    pub minute: NSInteger,
    pub second: NSInteger,
    pub nanosecond: NSInteger,
    pub weekday: NSInteger,
    pub weekday_ordinal: NSInteger,
    pub week_of_month: NSInteger,
    pub week_of_year: NSInteger,
    pub year_for_week_of_year: NSInteger,
    pub quarter: NSInteger,
    /// `NSCalendar*` — weak (not retained since components are lightweight)
    pub calendar: id,
    /// `NSTimeZone*`
    pub time_zone: id,
}
impl HostObject for NSDateComponentsHostObject {}

/// Sentinel value returned by getters when a component has not been set.
/// Matches `NSDateComponentUndefined` on real iOS (= `NSIntegerMax`).
const NSDateComponentUndefined: NSInteger = NSInteger::MAX;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);
@implementation NSDateComponents: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSDateComponentsHostObject {
        era:        NSDateComponentUndefined,
        year:       NSDateComponentUndefined,
        month:      NSDateComponentUndefined,
        week:       NSDateComponentUndefined,
        day:        NSDateComponentUndefined,
        hour:       NSDateComponentUndefined,
        minute:     NSDateComponentUndefined,
        second:     NSDateComponentUndefined,
        nanosecond: NSDateComponentUndefined,
        weekday:    NSDateComponentUndefined,
        weekday_ordinal:       NSDateComponentUndefined,
        week_of_month:         NSDateComponentUndefined,
        week_of_year:          NSDateComponentUndefined,
        year_for_week_of_year: NSDateComponentUndefined,
        quarter:    NSDateComponentUndefined,
        calendar:   crate::objc::nil,
        time_zone:  crate::objc::nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Era / Year / Quarter

- (NSInteger)era { env.objc.borrow::<NSDateComponentsHostObject>(this).era }
- (())setEra:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).era = v; }

- (NSInteger)year { env.objc.borrow::<NSDateComponentsHostObject>(this).year }
- (())setYear:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).year = v; }

- (NSInteger)quarter { env.objc.borrow::<NSDateComponentsHostObject>(this).quarter }
- (())setQuarter:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).quarter = v; }

// MARK: - Month / Day

- (NSInteger)month { env.objc.borrow::<NSDateComponentsHostObject>(this).month }
- (())setMonth:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).month = v; }

- (NSInteger)week { env.objc.borrow::<NSDateComponentsHostObject>(this).week }
- (())setWeek:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).week = v; }

- (NSInteger)day { env.objc.borrow::<NSDateComponentsHostObject>(this).day }
- (())setDay:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).day = v; }

// MARK: - Hour / Minute / Second / Nanosecond

- (NSInteger)hour { env.objc.borrow::<NSDateComponentsHostObject>(this).hour }
- (())setHour:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).hour = v; }

- (NSInteger)minute { env.objc.borrow::<NSDateComponentsHostObject>(this).minute }
- (())setMinute:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).minute = v; }

- (NSInteger)second { env.objc.borrow::<NSDateComponentsHostObject>(this).second }
- (())setSecond:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).second = v; }

- (NSInteger)nanosecond { env.objc.borrow::<NSDateComponentsHostObject>(this).nanosecond }
- (())setNanosecond:(NSInteger)v {
    env.objc.borrow_mut::<NSDateComponentsHostObject>(this).nanosecond = v;
}

// MARK: - Weekday

- (NSInteger)weekday { env.objc.borrow::<NSDateComponentsHostObject>(this).weekday }
- (())setWeekday:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).weekday = v; }

- (NSInteger)weekdayOrdinal {
    env.objc.borrow::<NSDateComponentsHostObject>(this).weekday_ordinal
}
- (())setWeekdayOrdinal:(NSInteger)v {
    env.objc.borrow_mut::<NSDateComponentsHostObject>(this).weekday_ordinal = v;
}

- (NSInteger)weekOfMonth {
    env.objc.borrow::<NSDateComponentsHostObject>(this).week_of_month
}
- (())setWeekOfMonth:(NSInteger)v {
    env.objc.borrow_mut::<NSDateComponentsHostObject>(this).week_of_month = v;
}

- (NSInteger)weekOfYear {
    env.objc.borrow::<NSDateComponentsHostObject>(this).week_of_year
}
- (())setWeekOfYear:(NSInteger)v {
    env.objc.borrow_mut::<NSDateComponentsHostObject>(this).week_of_year = v;
}

- (NSInteger)yearForWeekOfYear {
    env.objc.borrow::<NSDateComponentsHostObject>(this).year_for_week_of_year
}
- (())setYearForWeekOfYear:(NSInteger)v {
    env.objc.borrow_mut::<NSDateComponentsHostObject>(this).year_for_week_of_year = v;
}

// MARK: - Calendar / TimeZone

- (id)calendar {
    env.objc.borrow::<NSDateComponentsHostObject>(this).calendar
}
- (())setCalendar:(id)calendar {
    // Weak — no retain.
    env.objc.borrow_mut::<NSDateComponentsHostObject>(this).calendar = calendar;
}

- (id)timeZone {
    env.objc.borrow::<NSDateComponentsHostObject>(this).time_zone
}
- (())setTimeZone:(id)tz {
    // Weak — no retain.
    env.objc.borrow_mut::<NSDateComponentsHostObject>(this).time_zone = tz;
}

// MARK: - Validity

- (bool)isValidDate {
    // Report valid if year/month/day are all set.
    let host = env.objc.borrow::<NSDateComponentsHostObject>(this);
    host.year  != NSDateComponentUndefined
        && host.month != NSDateComponentUndefined
        && host.day   != NSDateComponentUndefined
}

- (bool)isValidDateInCalendar:(id)_calendar {
    msg![env;
    this isValidDate]
}

// MARK: - Date conversion

- (id)date {
    // Delegate to the associated calendar if available, otherwise return nil.
    let calendar = env.objc.borrow::<NSDateComponentsHostObject>(this).calendar;
    if calendar != crate::objc::nil {
        msg![env;
        calendar dateFromComponents:this]
    } else {
        crate::objc::nil
    }
}

// MARK: - Description

- (id)description {
    let host = env.objc.borrow::<NSDateComponentsHostObject>(this);
    let s = format!(
        "NSDateComponents: era={} year={} month={} day={} \
         hour={} minute={} second={} nanosecond={} \
         weekday={} weekdayOrdinal={} weekOfMonth={} weekOfYear={} \
         yearForWeekOfYear={} quarter={}",
        host.era, host.year, host.month, host.day,
        host.hour, host.minute, host.second, host.nanosecond,
        host.weekday, host.weekday_ordinal, host.week_of_month, host.week_of_year,
        host.year_for_week_of_year, host.quarter,
    );
    let ns = crate::frameworks::foundation::ns_string::from_rust_string(env, s);
    crate::objc::autorelease(env, ns)
}

@end

};
