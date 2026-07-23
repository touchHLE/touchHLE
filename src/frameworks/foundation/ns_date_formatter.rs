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
use crate::libc::time::{calendar_date_to_timestamp, tm};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, ClassExports, HostObject, NSZonePtr,
};

fn take_digits(input: &str, offset: &mut usize, min: usize, max: usize) -> Option<u32> {
    let bytes = input.as_bytes();
    let start = *offset;
    while *offset < bytes.len() && *offset - start < max && bytes[*offset].is_ascii_digit() {
        *offset += 1;
    }
    if *offset - start < min {
        return None;
    }
    input[start..*offset].parse().ok()
}

/// Parse the common Unicode/NSDateFormatter calendar fields used by early
/// iPhone applications. Returns `None`, like Cocoa, when the string does not
/// match the configured format.
fn parse_date(format: &str, input: &str) -> Option<(u16, u8, u8, u8, u8, u8)> {
    let mut format_offset = 0;
    let mut input_offset = 0;
    let mut year = 1970u16;
    let mut month = 1u8;
    let mut day = 1u8;
    let mut hour = 0u8;
    let mut minute = 0u8;
    let mut second = 0u8;

    while format_offset < format.len() {
        let remaining = &format[format_offset..];
        if remaining.starts_with("yyyy") || remaining.starts_with("YYYY") {
            year = take_digits(input, &mut input_offset, 4, 4)?
                .try_into()
                .ok()?;
            format_offset += 4;
        } else if remaining.starts_with("yy") || remaining.starts_with("YY") {
            let short_year = take_digits(input, &mut input_offset, 2, 2)? as u16;
            year = if short_year >= 70 {
                1900 + short_year
            } else {
                2000 + short_year
            };
            format_offset += 2;
        } else if remaining.starts_with("MMMM") || remaining.starts_with("MMM") {
            const MONTH_NAMES: [(&str, &str); 12] = [
                ("Jan", "January"),
                ("Feb", "February"),
                ("Mar", "March"),
                ("Apr", "April"),
                ("May", "May"),
                ("Jun", "June"),
                ("Jul", "July"),
                ("Aug", "August"),
                ("Sep", "September"),
                ("Oct", "October"),
                ("Nov", "November"),
                ("Dec", "December"),
            ];
            let long = remaining.starts_with("MMMM");
            let input_remaining = &input[input_offset..];
            let (month_idx, matched_len) =
                MONTH_NAMES
                    .iter()
                    .enumerate()
                    .find_map(|(idx, (short, full))| {
                        let candidate = if long { *full } else { *short };
                        input_remaining
                            .get(..candidate.len())
                            .filter(|value| value.eq_ignore_ascii_case(candidate))
                            .map(|_| (idx, candidate.len()))
                    })?;
            month = (month_idx + 1) as u8;
            input_offset += matched_len;
            format_offset += if long { 4 } else { 3 };
        } else if remaining.starts_with("MM") {
            month = take_digits(input, &mut input_offset, 2, 2)?
                .try_into()
                .ok()?;
            format_offset += 2;
        } else if remaining.starts_with('M') {
            month = take_digits(input, &mut input_offset, 1, 2)?
                .try_into()
                .ok()?;
            format_offset += 1;
        } else if remaining.starts_with("dd") {
            day = take_digits(input, &mut input_offset, 2, 2)?
                .try_into()
                .ok()?;
            format_offset += 2;
        } else if remaining.starts_with('d') {
            day = take_digits(input, &mut input_offset, 1, 2)?
                .try_into()
                .ok()?;
            format_offset += 1;
        } else if remaining.starts_with("HH") {
            hour = take_digits(input, &mut input_offset, 2, 2)?
                .try_into()
                .ok()?;
            format_offset += 2;
        } else if remaining.starts_with('H') {
            hour = take_digits(input, &mut input_offset, 1, 2)?
                .try_into()
                .ok()?;
            format_offset += 1;
        } else if remaining.starts_with("mm") {
            minute = take_digits(input, &mut input_offset, 2, 2)?
                .try_into()
                .ok()?;
            format_offset += 2;
        } else if remaining.starts_with('m') {
            minute = take_digits(input, &mut input_offset, 1, 2)?
                .try_into()
                .ok()?;
            format_offset += 1;
        } else if remaining.starts_with("ss") {
            second = take_digits(input, &mut input_offset, 2, 2)?
                .try_into()
                .ok()?;
            format_offset += 2;
        } else if remaining.starts_with('s') {
            second = take_digits(input, &mut input_offset, 1, 2)?
                .try_into()
                .ok()?;
            format_offset += 1;
        } else {
            let literal = remaining.chars().next()?;
            let actual = input[input_offset..].chars().next()?;
            if literal != actual {
                return None;
            }
            format_offset += literal.len_utf8();
            input_offset += actual.len_utf8();
        }
    }

    if input_offset != input.len()
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    Some((year, month, day, hour, minute, second))
}

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

- (id)dateFromString:(id)string { // NSString *
    let date_format = env
        .objc
        .borrow::<NSDateFormatterHostObject>(this)
        .date_format
        .expect("NSDateFormatter dateFromString: called without a date format");
    let format = ns_string::to_rust_string(env, date_format).to_owned();
    let string = ns_string::to_rust_string(env, string).to_owned();
    let Some((year, month, day, hour, minute, second)) = parse_date(&format, &string) else {
        log_dbg!("NSDateFormatter could not parse {:?} with {:?}", string, format);
        return nil;
    };
    let unix_timestamp =
        calendar_date_to_timestamp(tm::from(year, month, day, hour, minute, second));
    log_dbg!(
        "NSDateFormatter parsed {:?} with {:?} => {}",
        string,
        format,
        unix_timestamp
    );
    let unix_timestamp = unix_timestamp as NSTimeInterval;
    msg_class![env; NSDate dateWithTimeIntervalSince1970:unix_timestamp]
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

    format = format.replace("yyyy", format!("{year:04}").as_str());
    format = format.replace("YYYY", format!("{year:04}").as_str());
    format = format.replace("MM", format!("{month:02}").as_str());
    format = format.replace("dd", format!("{day:02}").as_str());
    format = format.replace("HH", format!("{hour:02}").as_str());
    format = format.replace("mm", format!("{minute:02}").as_str());
    format = format.replace("ss", format!("{second:02}").as_str());

    for c in format.chars() {
        if let pattern @ ('A'..='Z' | 'a'..='z') = c {
            unimplemented!("date string contains unsubstituted format pattern: {pattern}");
        }
    }
    log_dbg!("date_format after: {:?}", format);

    let res = ns_string::from_rust_string(env, format);
    autorelease(env, res)
}

@end

};
