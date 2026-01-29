/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSCalendar`.

use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::objc::{id, nil, objc_classes, ClassExports, HostObject, NSZonePtr};
use crate::{msg, Environment};

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

- (())setFirstWeekday:(NSInteger)weekday {
    env.objc.borrow_mut::<NSCalendarHostObject>(this).first_weekday = weekday;
}

- (id)components:(NSUInteger)_unit_flags fromDate:(id)_date { // NSDate *
    log!("TODO: implement NSDateComponents and proper date component extraction");
    nil
}

@end

};
