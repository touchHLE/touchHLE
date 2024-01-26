/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSDateFormatter`.
//!
//! Resources:
//! - Apple's [Introduction to Data Formatting Programming Guide For Cocoa](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/DataFormatting/DataFormatting.html)
//! - [Unicode Technical Standard #35](https://unicode.org/reports/tr35/tr35-10.html#Date_Format_Patterns)

use crate::frameworks::core_foundation::time::CFAbsoluteTimeGetGregorianDate;
use crate::frameworks::foundation::{ns_string, NSTimeInterval};
use crate::objc::{id, msg, nil, objc_classes, ClassExports, HostObject, NSZonePtr};

struct NSDateFormatterHostObject {
    date_format: Option<id>,
}
impl HostObject for NSDateFormatterHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSDateFormatter: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSDateFormatterHostObject {
        date_format: None,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())setDateFormat:(id)format { // NSString *
    let date_format: id = msg![env; format copy];
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).date_format = Some(date_format);
}

- (id)stringFromDate:(id)date {
    let &NSDateFormatterHostObject {
        date_format
    } = env.objc.borrow(this);
    let mut format = ns_string::to_rust_string(env, date_format.unwrap()).to_string().clone();
    log_dbg!("date_format before: {:?}", format);

    let ti: NSTimeInterval = msg![env; date timeIntervalSinceReferenceDate];
    let greg_date = CFAbsoluteTimeGetGregorianDate(env, ti, nil);
    let year = greg_date.year;
    let month = greg_date.month;
    let day = greg_date.day;
    let hour = greg_date.hours;
    let minute = greg_date.minutes;
    let second = greg_date.seconds;

    format = format.replace("yyyy", format!("{:04}", year).as_str());
    format = format.replace("MM", format!("{:02}", month).as_str());
    format = format.replace("dd", format!("{}", day).as_str());
    format = format.replace("HH", format!("{}", hour).as_str());
    format = format.replace("mm", format!("{:02}", minute).as_str());
    format = format.replace("ss", format!("{:02}", second).as_str());

    for c in format.chars() {
        match c {
            'A'..='Z' | 'a'..='z' => unimplemented!("date string contains unsubstituted format patterns"),
            _ => {}
        }
    }
    log_dbg!("date_format after: {:?}", format);

    ns_string::from_rust_string(env, format)
}

@end

};
