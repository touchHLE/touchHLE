/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSDate`.

use super::NSTimeInterval;
use crate::frameworks::core_foundation::time::apple_epoch;
use crate::objc::{autorelease, id, objc_classes, ClassExports, HostObject};

use std::time::SystemTime;

struct NSDateHostObject {
    instant: NSTimeInterval,
}
impl HostObject for NSDateHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSDate: NSObject

+ (id)date {
    // "Date objects are immutable, representing an invariant time interval relative to an
    // absolute reference date (00:00:00 UTC on 1 January 2001)."
    let instant = SystemTime::now()
        .duration_since(apple_epoch())
        .unwrap()
        .as_secs_f64();
    let host_object = Box::new(NSDateHostObject {
        instant
    });
    let new = env.objc.alloc_object(this, host_object, &mut env.mem);

    log_dbg!("[(NSDate*){:?} date]: New date {:?}", this, new);

    autorelease(env, new)
}

- (NSTimeInterval)timeIntervalSinceDate:(id)anotherDate {
    assert!(!anotherDate.is_null());
    let host_object = env.objc.borrow::<NSDateHostObject>(this);
    let another_date_host_object = env.objc.borrow::<NSDateHostObject>(anotherDate);
    let result = another_date_host_object.instant - host_object.instant;
    log_dbg!("[(NSDate*){:?} timeIntervalSinceDate:{:?}]: result {} seconds", this, anotherDate, result);
    result
}

- (NSTimeInterval)timeIntervalSinceReferenceDate {
    env.objc.borrow::<NSDateHostObject>(this).instant
}

@end

};
