/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `time.h` (C) and `sys/time.h` (POSIX)

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::set_errno;
use crate::mem::{guest_size_of, ConstPtr, MutPtr, Ptr, SafeRead};
use crate::Environment;
use std::time::{Duration, Instant, SystemTime};

#[derive(Default)]
pub struct State {
    y2k38_warned: bool,
    /// Temporary static storage for the return value of `gmtime` or
    /// `localtime`. The standard allows calls to either to overwrite it.
    gmtime_tmp: Option<MutPtr<tm>>,
}

// time.h (C)

#[allow(non_camel_case_types)]
/// Time in seconds since UNIX epoch (1970-01-01 00:00:00)
pub type time_t = i32;

#[allow(non_camel_case_types)]
type clock_t = u64;

const CLOCKS_PER_SEC: clock_t = 1000000;

fn clock(env: &mut Environment) -> clock_t {
    Instant::now()
        .duration_since(env.startup_time)
        .as_secs()
        .wrapping_mul(CLOCKS_PER_SEC)
}

fn time(env: &mut Environment, out: MutPtr<time_t>) -> time_t {
    // TODO: handle errno properly
    set_errno(env, 0);

    let time64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let time = time64 as time_t;
    if !env.libc_state.time.y2k38_warned && time64 != time as u64 {
        env.libc_state.time.y2k38_warned = true;
        log!("Warning: system clock is beyond Y2K38 and might confuse the app");
    }
    if !out.is_null() {
        env.mem.write(out, time);
    }
    time
}

fn tzset(_env: &mut Environment) {
    log!("TODO: tzset()");
}

#[allow(non_camel_case_types)]
#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
/// `struct tm`, fields count from 0 unless marked otherwise
pub struct tm {
    /// second of the minute
    pub tm_sec: i32,
    /// minute of the hour
    pub tm_min: i32,
    /// hour of the day (24-hour)
    pub tm_hour: i32,
    /// day of the month (**from 1**)
    pub tm_mday: i32,
    /// month of the year
    pub tm_mon: i32,
    /// year with 1900 subtracted from it
    pub tm_year: i32,
    /// day of the week (where Sunday is the first day)
    tm_wday: i32,
    /// day of the year
    tm_yday: i32,
    /// 1 if daylight saving time is in effect
    tm_isdst: i32,
    /// timezone offset from UTC in seconds
    tm_gmtoff: i32,
    /// abbreviated timezone name (not `const` in C but why not?)
    tm_zone: ConstPtr<u8>,
}
unsafe impl SafeRead for tm {}

// Helpers for timestamp to calendar date conversion, all of these are our own
// original implementation details.
const fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}
/// Number of years in a Gregorian calendar cycle (leap year function cycle)
const CYCLE_YEARS: i32 = 400;
/// Lookup table where the index is the number of years since the first year in
/// a Gregorian calendar cycle (400 years), and the value is the number of days
/// between the first day in that year and the first day in the first year.
/// Intended for binary search.
const YEAR_TO_DAY: [i32; CYCLE_YEARS as usize] = calc_year_to_day().0;
/// Number of days in a Gregorian calendar cycle
const CYCLE_DAYS: i32 = calc_year_to_day().1;
const fn calc_year_to_day() -> ([i32; CYCLE_YEARS as usize], i32) {
    let mut table = [0i32; CYCLE_YEARS as usize];
    let mut day = 0;
    let mut year = 0;
    while year < CYCLE_YEARS {
        table[year as usize] = day;
        day += if is_leap_year(year) { 366 } else { 365 };
        year += 1;
    }
    (table, day)
}
/// Lookup table where the index is the number of months since the first month
/// of the year, and the value is the number of days in that month in a non-leap
/// year.
const DAYS_IN_MONTH: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
/// Lookup table where the index is the number of months since the first month
/// in a non-leap year, and the value is the number of days between that month's
/// first day and the first day of the year. Intended for binary search.
const MONTH_TO_DAY_NONLEAP: [i32; 12] = calc_month_to_day(false);
/// [MONTH_TO_DAY_NONLEAP] but for leap years.
const MONTH_TO_DAY_LEAP: [i32; 12] = calc_month_to_day(true);
const fn calc_month_to_day(leap_year: bool) -> [i32; 12] {
    let mut table = [0i32; 12];
    let mut day = 0;
    let mut month = 0;
    while month < 12 {
        table[month] = day;
        day += DAYS_IN_MONTH[month] + ((leap_year && month == 1) as i32);
        month += 1;
    }
    table
}
pub fn timestamp_to_calendar_date(timestamp: time_t) -> tm {
    let seconds_since_unix_epoch: i32 = timestamp;

    // The easy bit: seconds, minutes, hours and days don't vary in length in
    // UNIX time.

    let days_since_unix_epoch = seconds_since_unix_epoch.div_euclid(DAY_SECONDS);
    let second_in_day = seconds_since_unix_epoch.rem_euclid(DAY_SECONDS);

    const MINUTE_SECONDS: i32 = 60;
    const HOUR_SECONDS: i32 = MINUTE_SECONDS * 60;
    const DAY_SECONDS: i32 = HOUR_SECONDS * 24;
    let tm_sec = second_in_day % MINUTE_SECONDS;
    let tm_min = (second_in_day % HOUR_SECONDS) / MINUTE_SECONDS;
    let tm_hour = second_in_day / HOUR_SECONDS;

    // The hard bit: months and hence years vary in length.

    // UNIX time starts on 1970-01-01. The pattern of leap and non-leap years
    // in the Gregorian calendar resets when the year is a multiple of 400, e.g.
    // the year 2000, so let's adjust the epoch to make things easier.
    let days_since_y2k = days_since_unix_epoch - 10957;
    let cycles_since_y2k = days_since_y2k.div_euclid(CYCLE_DAYS);
    let day_in_cycle = days_since_y2k.rem_euclid(CYCLE_DAYS);

    let year_in_cycle: i32 = (YEAR_TO_DAY.partition_point(|&day| day <= day_in_cycle) - 1) as _;
    let year = 2000 + cycles_since_y2k * CYCLE_YEARS + year_in_cycle;
    let day_in_year = day_in_cycle - YEAR_TO_DAY[usize::try_from(year_in_cycle).unwrap()];
    let is_leap_year = is_leap_year(year_in_cycle);
    assert!(day_in_year < (365 + is_leap_year as i32));

    let month_to_day = if is_leap_year {
        &MONTH_TO_DAY_LEAP
    } else {
        &MONTH_TO_DAY_NONLEAP
    };
    let month_in_year: i32 = (month_to_day.partition_point(|&day| day <= day_in_year) - 1) as _;
    let day_in_month = day_in_year - month_to_day[usize::try_from(month_in_year).unwrap()];
    assert!(day_in_month < DAYS_IN_MONTH[month_in_year as usize] + is_leap_year as i32);

    // 0 = Sunday, 1970-01-01 was a Thursday
    let day_of_the_week = (4 + days_since_unix_epoch).rem_euclid(7);

    tm {
        tm_sec,
        tm_min,
        tm_hour,
        tm_mday: day_in_month + 1,
        tm_mon: month_in_year,
        tm_year: year - 1900,
        tm_wday: day_of_the_week,
        tm_yday: day_in_year,
        // This function always returns UTC
        tm_isdst: 0,
        tm_gmtoff: 0,
        // TODO: this probably shouldn't be NULL?
        tm_zone: Ptr::null(),
    }
}
#[cfg(test)]
#[test]
fn test_timestamp_to_calendar_date() {
    fn do_test(expected: &str, timestamp: time_t) {
        let tm {
            tm_year,
            tm_mon,
            tm_mday,
            tm_hour,
            tm_min,
            tm_sec,
            tm_wday,
            ..
        } = timestamp_to_calendar_date(timestamp);
        let wday = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][tm_wday as usize];
        assert_eq!(
            expected,
            &format!(
                "{}, {:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                wday,
                tm_year + 1900,
                tm_mon + 1,
                tm_mday,
                tm_hour,
                tm_min,
                tm_sec
            )
        );
    }
    // Random tests generated with this JavaScript:
    //
    //   for (i = 0; i < 12; i++) {
    //     let timestamp = (Math.random() * 2 ** 32) | 0;
    //     console.log(
    //       "do_test(\"" +
    //       (new Date(timestamp * 1000)).toUTCString().substr(0, 5) +
    //       (new Date(timestamp * 1000)).toISOString().substr(0, 19) +
    //       "\", " + timestamp + ");"
    //     );
    //   }
    do_test("Mon, 2006-02-20T01:27:52", 1140398872);
    do_test("Tue, 2036-12-16T06:40:54", 2113022454);
    do_test("Thu, 1922-03-02T06:22:31", -1509557849);
    do_test("Wed, 1990-07-25T13:02:43", 648910963);
    do_test("Wed, 1912-12-18T20:42:53", -1799896627);
    do_test("Wed, 1990-03-28T04:47:24", 638599644);
    do_test("Thu, 2034-03-30T18:44:51", 2027357091);
    do_test("Sun, 2022-01-09T21:41:51", 1641764511);
    do_test("Fri, 2018-04-13T17:03:50", 1523639030);
    do_test("Thu, 1973-08-30T10:11:33", 115553493);
    do_test("Fri, 2005-05-27T19:45:47", 1117223147);
    do_test("Sat, 1955-03-26T20:47:45", -466053135);
}

fn gmtime_r(env: &mut Environment, timestamp: ConstPtr<time_t>, res: MutPtr<tm>) -> MutPtr<tm> {
    let timestamp = env.mem.read(timestamp);
    let calendar_date = timestamp_to_calendar_date(timestamp);
    env.mem.write(res, calendar_date);
    res
}
fn gmtime(env: &mut Environment, timestamp: ConstPtr<time_t>) -> MutPtr<tm> {
    let tmp = *env
        .libc_state
        .time
        .gmtime_tmp
        .get_or_insert_with(|| env.mem.alloc(guest_size_of::<tm>()).cast());
    gmtime_r(env, timestamp, tmp)
}

fn localtime_r(env: &mut Environment, timestamp: ConstPtr<time_t>, res: MutPtr<tm>) -> MutPtr<tm> {
    // TODO: don't assume local time is UTC?
    gmtime_r(env, timestamp, res)
}
fn localtime(env: &mut Environment, timestamp: ConstPtr<time_t>) -> MutPtr<tm> {
    // TODO: don't assume local time is UTC?
    // This doesn't have to be a unique temporary, gmtime and localtime are
    // allowed to share it.
    gmtime(env, timestamp)
}

// sys/time.h (POSIX)

#[allow(non_camel_case_types)]
type suseconds_t = i32;

#[allow(non_camel_case_types)]
#[repr(C, packed)]
struct timeval {
    tv_sec: time_t,
    tv_usec: suseconds_t,
}
unsafe impl SafeRead for timeval {}

#[allow(non_camel_case_types)]
#[derive(Default)]
#[repr(C, packed)]
pub struct timespec {
    tv_sec: time_t,
    tv_nsec: i32,
}
unsafe impl SafeRead for timespec {}

#[allow(non_camel_case_types)]
#[repr(C, packed)]
struct timezone {
    tz_minuteswest: i32,
    tz_dsttime: i32,
}
unsafe impl SafeRead for timezone {}

fn gettimeofday(
    env: &mut Environment,
    timeval_ptr: MutPtr<timeval>,
    timezone_ptr: MutPtr<timezone>,
) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    if !timezone_ptr.is_null() {
        env.mem.write(
            timezone_ptr,
            timezone {
                tz_minuteswest: 0,
                tz_dsttime: 0,
            },
        );
    }

    if timeval_ptr.is_null() {
        return 0; // success
    }

    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let time_s_64: u64 = time.as_secs();
    let tv_sec = time_s_64 as time_t;
    if !env.libc_state.time.y2k38_warned && time_s_64 != tv_sec as u64 {
        env.libc_state.time.y2k38_warned = true;
        log!("Warning: system clock is beyond Y2K38 and might confuse the app");
    }
    let tv_usec: suseconds_t = time.subsec_micros().try_into().unwrap();

    env.mem.write(timeval_ptr, timeval { tv_sec, tv_usec });

    0 // success
}

fn nanosleep(env: &mut Environment, rqtp: ConstPtr<timespec>, _rmtp: MutPtr<timespec>) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let t = env.mem.read(rqtp);
    let tv_sec = t.tv_sec;
    let tv_nsec = t.tv_nsec;
    log_dbg!("nanosleep {} {}", tv_sec, tv_nsec);
    let total_sleep = Duration::from_secs(tv_sec.try_into().unwrap())
        + Duration::from_nanos(tv_nsec.try_into().unwrap());
    env.sleep(total_sleep, true);
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(clock()),
    export_c_func!(time(_)),
    export_c_func!(tzset()),
    export_c_func!(gmtime_r(_, _)),
    export_c_func!(gmtime(_)),
    export_c_func!(localtime_r(_, _)),
    export_c_func!(localtime(_)),
    export_c_func!(gettimeofday(_, _)),
    export_c_func!(nanosleep(_, _)),
];
