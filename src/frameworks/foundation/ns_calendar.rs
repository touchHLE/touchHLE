/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSCalendar`.

use crate::frameworks::foundation::ns_string;
use crate::objc::{id, objc_classes, retain, ClassExports, HostObject, NSZonePtr};
use crate::{msg, msg_class};

struct NSCalendarHostObject {
    /// Calendar identifier, e.g. "NSGregorianCalendar"
    identifier: id,
}
impl HostObject for NSCalendarHostObject {}

#[derive(Default)]
pub struct State {
    current_calendar: Option<id>,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSCalendar: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSCalendarHostObject {
        identifier: crate::objc::nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)currentCalendar {
    if let Some(existing) = env.framework_state.foundation.ns_calendar.current_calendar {
        existing
    } else {
        let gregorian = ns_string::get_static_str(env, "NSGregorianCalendar");
        let host_object = Box::new(NSCalendarHostObject {
            identifier: gregorian,
        });
        let new = env.objc.alloc_static_object(
            this,
            host_object,
            &mut env.mem,
        );
        env.framework_state.foundation.ns_calendar.current_calendar = Some(new);
        new
    }
}

+ (id)autoupdatingCurrentCalendar {
    // For stub purposes, just return currentCalendar
    msg![env; this currentCalendar]
}

- (id)calendarIdentifier {
    env.objc.borrow::<NSCalendarHostObject>(this).identifier
}

- (id)dateFromComponents:(id)comps {
    log!("TODO: [NSCalendar dateFromComponents:{:?}] returning current date", comps);
    msg_class![env; NSDate date]
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

@end

};
