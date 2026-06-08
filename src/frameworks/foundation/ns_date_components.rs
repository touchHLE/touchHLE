/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSDateComponents`.

use crate::frameworks::foundation::{NSInteger, NSNotFound};
use crate::objc::{id, objc_classes, ClassExports, HostObject, NSZonePtr};

struct NSDateComponentsHostObject {
    era: NSInteger,
    year: NSInteger,
    month: NSInteger,
    day: NSInteger,
    hour: NSInteger,
    minute: NSInteger,
    second: NSInteger,
    weekday: NSInteger,
    weekday_ordinal: NSInteger,
    quarter: NSInteger,
    week_of_month: NSInteger,
    week_of_year: NSInteger,
    year_for_week_of_year: NSInteger,
}

impl HostObject for NSDateComponentsHostObject {}

impl Default for NSDateComponentsHostObject {
    fn default() -> Self {
        Self {
            era: NSNotFound,
            year: NSNotFound,
            month: NSNotFound,
            day: NSNotFound,
            hour: NSNotFound,
            minute: NSNotFound,
            second: NSNotFound,
            weekday: NSNotFound,
            weekday_ordinal: NSNotFound,
            quarter: NSNotFound,
            week_of_month: NSNotFound,
            week_of_year: NSNotFound,
            year_for_week_of_year: NSNotFound,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSDateComponents: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<NSDateComponentsHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    // Already initialized with NSNotFound in allocWithZone
    this
}

- (NSInteger)era { env.objc.borrow::<NSDateComponentsHostObject>(this).era }
- (())setEra:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).era = v; }

- (NSInteger)year { env.objc.borrow::<NSDateComponentsHostObject>(this).year }
- (())setYear:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).year = v; }

- (NSInteger)month { env.objc.borrow::<NSDateComponentsHostObject>(this).month }
- (())setMonth:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).month = v; }

- (NSInteger)day { env.objc.borrow::<NSDateComponentsHostObject>(this).day }
- (())setDay:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).day = v; }

- (NSInteger)hour { env.objc.borrow::<NSDateComponentsHostObject>(this).hour }
- (())setHour:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).hour = v; }

- (NSInteger)minute { env.objc.borrow::<NSDateComponentsHostObject>(this).minute }
- (())setMinute:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).minute = v; }

- (NSInteger)second { env.objc.borrow::<NSDateComponentsHostObject>(this).second }
- (())setSecond:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).second = v; }

- (NSInteger)weekday { env.objc.borrow::<NSDateComponentsHostObject>(this).weekday }
- (())setWeekday:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).weekday = v; }

- (NSInteger)weekdayOrdinal { env.objc.borrow::<NSDateComponentsHostObject>(this).weekday_ordinal }
- (())setWeekdayOrdinal:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).weekday_ordinal = v; }

- (NSInteger)quarter { env.objc.borrow::<NSDateComponentsHostObject>(this).quarter }
- (())setQuarter:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).quarter = v; }

- (NSInteger)weekOfMonth { env.objc.borrow::<NSDateComponentsHostObject>(this).week_of_month }
- (())setWeekOfMonth:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).week_of_month = v; }

- (NSInteger)weekOfYear { env.objc.borrow::<NSDateComponentsHostObject>(this).week_of_year }
- (())setWeekOfYear:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).week_of_year = v; }

- (NSInteger)yearForWeekOfYear { env.objc.borrow::<NSDateComponentsHostObject>(this).year_for_week_of_year }
- (())setYearForWeekOfYear:(NSInteger)v { env.objc.borrow_mut::<NSDateComponentsHostObject>(this).year_for_week_of_year = v; }

@end

};
