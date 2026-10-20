/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Time things including `CFAbsoluteTime`.

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::CFTypeRef;
use crate::frameworks::foundation::NSTimeInterval;
use crate::libc::time::{time_t, timestamp_to_calendar_date};
use crate::mem::SafeRead;
use crate::objc::nil;
use crate::{impl_GuestRet_for_large_struct, Environment};
use std::ops::Add;
use std::time::{Duration, SystemTime};

/// Seconds between Unix and Apple's epochs
pub const SECS_FROM_UNIX_TO_APPLE_EPOCHS: u64 = 978_307_200;

/// The absolute reference date is 1 Jan 2001 00:00:00 GMT
pub fn apple_epoch() -> SystemTime {
    SystemTime::UNIX_EPOCH.add(Duration::from_secs(SECS_FROM_UNIX_TO_APPLE_EPOCHS))
}

pub type CFTimeInterval = NSTimeInterval;
pub type CFAbsoluteTime = CFTimeInterval;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C, packed)]
pub struct CFGregorianDate {
    pub year: i32,    // SInt32
    pub month: i8,    // SInt8
    pub day: i8,      // SInt8
    pub hours: i8,    // SInt8
    pub minutes: i8,  // SInt8
    pub seconds: f64, // double
}
unsafe impl SafeRead for CFGregorianDate {}
impl_GuestRet_for_large_struct!(CFGregorianDate);

/// Absolute time is measured in seconds relative to the absolute reference date
/// of Jan 1 2001 00:00:00 GMT.
fn CFAbsoluteTimeGetCurrent(_env: &mut Environment) -> CFAbsoluteTime {
    SystemTime::now()
        .duration_since(apple_epoch())
        .unwrap()
        .as_secs_f64()
}

type CFTimeZoneRef = CFTypeRef;

fn CFTimeZoneCopySystem(_env: &mut Environment) -> CFTimeZoneRef {
    // TODO: implement (nil seems to correspond to GMT)
    nil
}

pub fn CFAbsoluteTimeGetGregorianDate(
    _env: &mut Environment,
    at: CFAbsoluteTime,
    tz: CFTimeZoneRef,
) -> CFGregorianDate {
    assert!(tz.is_null());
    let time64 = apple_epoch()
        .add(Duration::from_secs_f64(at))
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let time = time64 as time_t;
    let tm = timestamp_to_calendar_date(time);
    CFGregorianDate {
        year: 1900 + tm.tm_year,
        month: (tm.tm_mon + 1) as i8,
        day: tm.tm_mday as i8,
        hours: tm.tm_hour as i8,
        minutes: tm.tm_min as i8,
        seconds: tm.tm_sec.into(),
    }
}

fn CFAbsoluteTimeGetDayOfWeek(env: &mut Environment, at: CFAbsoluteTime, tz: CFTimeZoneRef) -> i32 {
    assert!(tz.is_null());
    CFAbsoluteTimeGetGregorianDate(env, at, tz).day.into()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFAbsoluteTimeGetCurrent()),
    export_c_func!(CFTimeZoneCopySystem()),
    export_c_func!(CFAbsoluteTimeGetGregorianDate(_, _)),
    export_c_func!(CFAbsoluteTimeGetDayOfWeek(_, _)),
];
