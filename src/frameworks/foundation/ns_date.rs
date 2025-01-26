/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSDate`.

use touchhle_macros::validate_class_exports;

use super::ns_string::from_rust_ordering;
use super::{NSComparisonResult, NSTimeInterval};
use crate::frameworks::core_foundation::time::{apple_epoch, SECS_FROM_UNIX_TO_APPLE_EPOCHS};
use crate::objc::{
    autorelease, id, msg, msg_class, objc_classes, release, ClassExports, HostObject, NSZonePtr,
};

use crate::frameworks::foundation::ns_keyed_unarchiver::decode_current_date;
use std::ops::{Add, Sub};
use std::time::{Duration, SystemTime};

#[derive(Default)]
struct NSDateHostObject {
    time_interval: NSTimeInterval,
}
impl HostObject for NSDateHostObject {}

#[validate_class_exports]
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSDate: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<NSDateHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (NSTimeInterval)timeIntervalSinceReferenceDate {
    SystemTime::now()
        .duration_since(apple_epoch())
        .unwrap()
        .as_secs_f64()
}

+ (id)date {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new init];
    log_dbg!("[NSDate date] => {:?} ({:?}s)", new, env.objc.borrow::<NSDateHostObject>(this).time_interval);
    autorelease(env, new)
}

+ (id)distantFuture {
    // As of 2024, this approximately corresponds to 20 years into the future.
    // While `distantFuture` docs are talking in terms of centuries,
    // this should be OK to use for our purposes.
    let time_interval = SystemTime::now()
        .duration_since(apple_epoch())
        .unwrap()
        .as_secs_f64() * 2.0;
    let host_object = Box::new(NSDateHostObject {
        time_interval
    });
    let new = env.objc.alloc_object(this, host_object, &mut env.mem);

    log_dbg!("[(NSDate*){:?} distantFuture]: date {:?} (time_interval: {})", this, new, time_interval);

    autorelease(env, new)
}

+ (id)distantPast {
    // This corresponds to the Unix epoch from Apple's reference date.
    // While `distantPast` docs are talking in terms of centuries,
    // for our purposes it is OK to use the Unix epoch as a distant past.
    let time_interval = -(SECS_FROM_UNIX_TO_APPLE_EPOCHS as f64);
    let host_object = Box::new(NSDateHostObject {
        time_interval
    });
    let new = env.objc.alloc_object(this, host_object, &mut env.mem);

    log_dbg!("[(NSDate*){:?} distantPast]: date {:?} (time_interval: {})", this, new, time_interval);

    autorelease(env, new)
}

+ (id)dateWithTimeIntervalSinceNow:(NSTimeInterval)secs {
    let now: id = msg_class![env; NSDate date];
    msg![env; now addTimeInterval:secs]
}

+ (id)dateWithTimeIntervalSince1970:(NSTimeInterval)secs {
    let time_interval = -(SECS_FROM_UNIX_TO_APPLE_EPOCHS as f64) + secs;
    let host_object = Box::new(NSDateHostObject {
        time_interval
    });
    let new = env.objc.alloc_object(this, host_object, &mut env.mem);
    autorelease(env, new)
}

+ (id)dateWithTimeInterval:(NSTimeInterval)secs
                 sinceDate:(id)date { // NSDate *
    let time_interval = env.objc.borrow_mut::<NSDateHostObject>(date).time_interval + secs;
    let host_object = Box::new(NSDateHostObject {
        time_interval
    });
    let new = env.objc.alloc_object(this, host_object, &mut env.mem);
    autorelease(env, new)
}

- (id)init {
    // "Date objects are immutable, representing an invariant time interval
    // relative to an absolute reference date (00:00:00 UTC on 1 January 2001)."
    let time_interval = SystemTime::now()
        .duration_since(apple_epoch())
        .unwrap()
        .as_secs_f64();
    env.objc.borrow_mut::<NSDateHostObject>(this).time_interval = time_interval;
    this
}

- (id)initWithTimeIntervalSinceNow:(NSTimeInterval)secs {
    let time_interval = SystemTime::now()
        .duration_since(apple_epoch())
        .unwrap()
        .as_secs_f64();
    env.objc.borrow_mut::<NSDateHostObject>(this).time_interval = time_interval + secs;
    this
}

- (id)initWithTimeIntervalSinceReferenceDate:(NSTimeInterval)secs {
    env.objc.borrow_mut::<NSDateHostObject>(this).time_interval = secs;
    this
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    release(env, this);
    // Note: Assuming NSKeyedUnarchiver as coder here
    decode_current_date(env, coder)
}

- (NSTimeInterval)timeIntervalSinceDate:(id)anotherDate {
    assert!(!anotherDate.is_null());
    let host_object = env.objc.borrow::<NSDateHostObject>(this);
    let another_date_host_object = env.objc.borrow::<NSDateHostObject>(anotherDate);
    let result =  host_object.time_interval-another_date_host_object.time_interval;
    log_dbg!("[(NSDate*){:?} ({:?}s) timeIntervalSinceDate:{:?} ({:?}s)] => {}", this, host_object.time_interval, anotherDate, another_date_host_object.time_interval, result);
    result
}

- (NSTimeInterval)timeIntervalSinceReferenceDate {
    env.objc.borrow::<NSDateHostObject>(this).time_interval
}

- (NSTimeInterval)timeIntervalSinceNow {
    let host_object = env.objc.borrow::<NSDateHostObject>(this);
    let time_interval = SystemTime::now()
        .duration_since(apple_epoch())
        .unwrap()
        .as_secs_f64();
    time_interval - host_object.time_interval
}

- (NSTimeInterval)timeIntervalSince1970 {
    let time_interval = env.objc.borrow::<NSDateHostObject>(this).time_interval;
    let new_time = if time_interval >= 0.0 {
        apple_epoch().add(Duration::from_secs_f64(time_interval))
    } else {
        apple_epoch().sub(Duration::from_secs_f64(-time_interval))
    };
    new_time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

- (id)addTimeInterval:(NSTimeInterval)seconds {
    let interval = env.objc.borrow::<NSDateHostObject>(this).time_interval + seconds;
    let date = msg_class![env; NSDate date];
    env.objc.borrow_mut::<NSDateHostObject>(date).time_interval = interval;
    date
}

- (NSComparisonResult)compare:(id)anotherDate { // NSDate *
    let host_object = env.objc.borrow::<NSDateHostObject>(this);
    let another_date_host_object = env.objc.borrow::<NSDateHostObject>(anotherDate);
    from_rust_ordering(host_object.time_interval.total_cmp(&another_date_host_object.time_interval))
}

@end

};
