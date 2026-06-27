/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSDateFormatter`.

use crate::frameworks::core_foundation::time::CFAbsoluteTimeGetGregorianDate;
use crate::frameworks::foundation::{ns_string, NSTimeInterval, NSUInteger};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};

// MARK: - Style constants

type NSDateFormatterStyle = NSUInteger;
const NSDateFormatterNoStyle: NSDateFormatterStyle = 0;
const NSDateFormatterShortStyle: NSDateFormatterStyle = 1;
const NSDateFormatterMediumStyle: NSDateFormatterStyle = 2;
const NSDateFormatterLongStyle: NSDateFormatterStyle = 3;
const NSDateFormatterFullStyle: NSDateFormatterStyle = 4;

// MARK: - Behavior constants

/// `NSDateFormatterBehavior` — selects between the legacy Foundation date
/// formatter behaviour (`NSDateFormatterBehavior10_0`, using NeXTSTEP-style
/// "%Y-%m-%d" strftime tokens) and the modern Unicode CLDR-based behaviour
/// (`NSDateFormatterBehavior10_4`). `NSDateFormatterBehaviorDefault` resolves
/// to whichever behaviour is currently set as the class-wide default; in real
/// Cocoa that resolves to `NSDateFormatterBehavior10_4` on iOS.
///
/// touchHLE implements the modern (10.4) behaviour, but we still need to
/// store and report the requested behaviour so apps that round-trip the value
/// see the expected result.
pub type NSDateFormatterBehavior = NSUInteger;
pub const NSDateFormatterBehaviorDefault: NSDateFormatterBehavior = 0;
pub const NSDateFormatterBehavior10_0: NSDateFormatterBehavior = 1000;
pub const NSDateFormatterBehavior10_4: NSDateFormatterBehavior = 1040;

/// Process-wide default for `+[NSDateFormatter defaultFormatterBehavior]`.
/// Apple uses `NSDateFormatterBehavior10_4` on iOS, and so do we.
const DEFAULT_FORMATTER_BEHAVIOR: NSDateFormatterBehavior = NSDateFormatterBehavior10_4;

#[derive(Default)]
struct NSDateFormatterHostObject {
    /// `NSString*` — custom format string, nil if using style
    date_format: id,
    /// `NSLocale*`
    locale: id,
    /// `NSTimeZone*`
    time_zone: id,
    /// `NSCalendar*`
    calendar: id,
    date_style: NSDateFormatterStyle,
    time_style: NSDateFormatterStyle,
    /// `NSDateFormatterBehavior` — see [`NSDateFormatterBehavior`].
    formatter_behavior: NSDateFormatterBehavior,
    lenient: bool,
    uses_24hr: bool,
    two_digit_start_date: id, // NSDate*
    default_date: id,         // NSDate*
    does_relative_date_formatting: bool,
    /// `NSString*` — AM symbol (defaults to "AM" via `apply_format`).
    am_symbol: id,
    /// `NSString*` — PM symbol (defaults to "PM").
    pm_symbol: id,
    /// `NSArray<NSString*>*` — short month names (e.g. "Jan", "Feb"). nil = default.
    short_month_symbols: id,
    /// `NSArray<NSString*>*` — full month names. nil = default.
    month_symbols: id,
    /// `NSArray<NSString*>*` — short weekday names ("Sun" .. "Sat"). nil = default.
    short_weekday_symbols: id,
    /// `NSArray<NSString*>*` — full weekday names. nil = default.
    weekday_symbols: id,
}
impl HostObject for NSDateFormatterHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSDateFormatter: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSDateFormatterHostObject {
        date_format:   nil,
        locale:        nil,
        time_zone:     nil,
        calendar:      nil,
        date_style:    NSDateFormatterNoStyle,
        time_style:    NSDateFormatterNoStyle,
        formatter_behavior: DEFAULT_FORMATTER_BEHAVIOR,
        lenient:       false,
        uses_24hr:     false,
        two_digit_start_date: nil,
        default_date:  nil,
        does_relative_date_formatting: false,
        am_symbol:               nil,
        pm_symbol:               nil,
        short_month_symbols:     nil,
        month_symbols:           nil,
        short_weekday_symbols:   nil,
        weekday_symbols:         nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: - Class-wide default behavior

+ (NSDateFormatterBehavior)defaultFormatterBehavior {
    // touchHLE implements modern (Unicode CLDR) behaviour, matching iOS.
    DEFAULT_FORMATTER_BEHAVIOR
}

+ (())setDefaultFormatterBehavior:(NSDateFormatterBehavior)behavior {
    // Apple lets the process pick between the legacy 10.0 NeXT-style
    // formatter and the modern 10.4 CLDR formatter. touchHLE only implements
    // the modern one, so we accept the setter but warn (rather than crash)
    // if anyone explicitly asks for the legacy path.
    if behavior == NSDateFormatterBehavior10_0 {
        log!(
            "Warning: +[NSDateFormatter setDefaultFormatterBehavior:NSDateFormatterBehavior10_0] \
             requested; touchHLE only implements NSDateFormatterBehavior10_4 (Unicode CLDR), \
             ignoring."
        );
    } else if behavior != NSDateFormatterBehavior10_4 && behavior != NSDateFormatterBehaviorDefault
    {
        log!(
            "Warning: unknown NSDateFormatterBehavior {}, ignoring",
            behavior
        );
    }
}

+ (id)localizedStringFromDate:(id)date
                    dateStyle:(NSDateFormatterStyle)date_style
                    timeStyle:(NSDateFormatterStyle)time_style {
    let formatter: id = msg_class![env; NSDateFormatter new];
    () = msg![env; formatter setDateStyle:date_style];
    () = msg![env; formatter setTimeStyle:time_style];
    let result: id = msg![env; formatter stringFromDate:date];
    release(env, formatter);
    result
}

+ (id)dateFormatFromTemplate:(id)_tmpl
                     options:(NSUInteger)_opts
                      locale:(id)_locale {
    // Full CLDR template resolution is complex; return the template unchanged.
    _tmpl
}

- (())dealloc {
    let host = env.objc.borrow::<NSDateFormatterHostObject>(this);
    let fmt = host.date_format;
    let locale = host.locale;
    let tz = host.time_zone;
    let cal = host.calendar;
    let tds = host.two_digit_start_date;
    let dd = host.default_date;
    let am = host.am_symbol;
    let pm = host.pm_symbol;
    let sms = host.short_month_symbols;
    let ms = host.month_symbols;
    let sws = host.short_weekday_symbols;
    let ws = host.weekday_symbols;
    release(env, fmt);
    release(env, locale);
    release(env, tz);
    release(env, cal);
    release(env, tds);
    release(env, dd);
    release(env, am);
    release(env, pm);
    release(env, sms);
    release(env, ms);
    release(env, sws);
    release(env, ws);
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Format string

- (id)dateFormat {
    env.objc.borrow::<NSDateFormatterHostObject>(this).date_format
}

- (())setDateFormat:(id)format { // NSString*
    let old = env.objc.borrow::<NSDateFormatterHostObject>(this).date_format;
    release(env, old);
    let copy: id = if format != nil { msg![env; format copy] } else { nil };
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).date_format = copy;
}

// MARK: - Styles

- (NSDateFormatterStyle)dateStyle {
    env.objc.borrow::<NSDateFormatterHostObject>(this).date_style
}

- (())setDateStyle:(NSDateFormatterStyle)style {
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).date_style = style;
}

- (NSDateFormatterStyle)timeStyle {
    env.objc.borrow::<NSDateFormatterHostObject>(this).time_style
}

- (())setTimeStyle:(NSDateFormatterStyle)style {
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).time_style = style;
}

// MARK: - Per-instance NSDateFormatterBehavior

- (NSDateFormatterBehavior)formatterBehavior {
    env.objc.borrow::<NSDateFormatterHostObject>(this).formatter_behavior
}

- (())setFormatterBehavior:(NSDateFormatterBehavior)behavior {
    let resolved = match behavior {
        NSDateFormatterBehaviorDefault => DEFAULT_FORMATTER_BEHAVIOR,
        NSDateFormatterBehavior10_0 => {
            log!(
                "Warning: [(NSDateFormatter*){:?} setFormatterBehavior:NSDateFormatterBehavior10_0]: \
                 touchHLE only implements the modern (10.4) Unicode behaviour; the value will be \
                 stored as requested but formatting still uses Unicode CLDR tokens.",
                this
            );
            NSDateFormatterBehavior10_0
        }
        NSDateFormatterBehavior10_4 => NSDateFormatterBehavior10_4,
        other => {
            log!(
                "Warning: [(NSDateFormatter*){:?} setFormatterBehavior:{}]: unknown behaviour, \
                 falling back to NSDateFormatterBehavior10_4",
                this, other
            );
            NSDateFormatterBehavior10_4
        }
    };
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).formatter_behavior = resolved;
}

// MARK: - Symbols (AM/PM, months, weekdays)
//
// Apple lets apps localise an `NSDateFormatter` by overriding individual
// symbol arrays. `apply_format` uses English defaults when these are `nil`,
// but storing them with proper retain/release means apps that round-trip the
// values (or read them back via `monthSymbols` etc.) observe the same array
// they set.

- (id)AMSymbol {
    env.objc.borrow::<NSDateFormatterHostObject>(this).am_symbol
}
- (())setAMSymbol:(id)symbol {
    let old = env.objc.borrow::<NSDateFormatterHostObject>(this).am_symbol;
    release(env, old);
    let copy: id = if symbol != nil { msg![env; symbol copy] } else { nil };
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).am_symbol = copy;
}

- (id)PMSymbol {
    env.objc.borrow::<NSDateFormatterHostObject>(this).pm_symbol
}
- (())setPMSymbol:(id)symbol {
    let old = env.objc.borrow::<NSDateFormatterHostObject>(this).pm_symbol;
    release(env, old);
    let copy: id = if symbol != nil { msg![env; symbol copy] } else { nil };
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).pm_symbol = copy;
}

- (id)shortMonthSymbols {
    env.objc.borrow::<NSDateFormatterHostObject>(this).short_month_symbols
}
- (())setShortMonthSymbols:(id)symbols {
    let old = env.objc.borrow::<NSDateFormatterHostObject>(this).short_month_symbols;
    release(env, old);
    retain(env, symbols);
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).short_month_symbols = symbols;
}

- (id)monthSymbols {
    env.objc.borrow::<NSDateFormatterHostObject>(this).month_symbols
}
- (())setMonthSymbols:(id)symbols {
    let old = env.objc.borrow::<NSDateFormatterHostObject>(this).month_symbols;
    release(env, old);
    retain(env, symbols);
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).month_symbols = symbols;
}

- (id)shortWeekdaySymbols {
    env.objc.borrow::<NSDateFormatterHostObject>(this).short_weekday_symbols
}
- (())setShortWeekdaySymbols:(id)symbols {
    let old = env.objc.borrow::<NSDateFormatterHostObject>(this).short_weekday_symbols;
    release(env, old);
    retain(env, symbols);
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).short_weekday_symbols = symbols;
}

- (id)weekdaySymbols {
    env.objc.borrow::<NSDateFormatterHostObject>(this).weekday_symbols
}
- (())setWeekdaySymbols:(id)symbols {
    let old = env.objc.borrow::<NSDateFormatterHostObject>(this).weekday_symbols;
    release(env, old);
    retain(env, symbols);
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).weekday_symbols = symbols;
}

// MARK: - Locale / TimeZone / Calendar

- (id)locale {
    let l = env.objc.borrow::<NSDateFormatterHostObject>(this).locale;
    if l != nil { l } else { msg_class![env; NSLocale currentLocale] }
}

- (())setLocale:(id)locale {
    let old = env.objc.borrow::<NSDateFormatterHostObject>(this).locale;
    release(env, old);
    retain(env, locale);
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).locale = locale;
}

- (id)timeZone {
    env.objc.borrow::<NSDateFormatterHostObject>(this).time_zone
}

- (())setTimeZone:(id)tz {
    let old = env.objc.borrow::<NSDateFormatterHostObject>(this).time_zone;
    release(env, old);
    retain(env, tz);
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).time_zone = tz;
}

- (id)calendar {
    env.objc.borrow::<NSDateFormatterHostObject>(this).calendar
}

- (())setCalendar:(id)cal {
    let old = env.objc.borrow::<NSDateFormatterHostObject>(this).calendar;
    release(env, old);
    retain(env, cal);
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).calendar = cal;
}

// MARK: - Misc properties

- (bool)isLenient {
    env.objc.borrow::<NSDateFormatterHostObject>(this).lenient
}
- (())setLenient:(bool)v {
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).lenient = v;
}

- (bool)doesRelativeDateFormatting {
    env.objc.borrow::<NSDateFormatterHostObject>(this).does_relative_date_formatting
}
- (())setDoesRelativeDateFormatting:(bool)v {
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).does_relative_date_formatting = v;
}

- (id)twoDigitStartDate {
    env.objc.borrow::<NSDateFormatterHostObject>(this).two_digit_start_date
}
- (())setTwoDigitStartDate:(id)date {
    let old = env.objc.borrow::<NSDateFormatterHostObject>(this).two_digit_start_date;
    release(env, old);
    retain(env, date);
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).two_digit_start_date = date;
}

- (id)defaultDate {
    env.objc.borrow::<NSDateFormatterHostObject>(this).default_date
}
- (())setDefaultDate:(id)date {
    let old = env.objc.borrow::<NSDateFormatterHostObject>(this).default_date;
    release(env, old);
    retain(env, date);
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).default_date = date;
}

- (id)stringFromDate:(id)date { // NSDate*
    if date == nil { return nil; }

    let ti: NSTimeInterval = msg![env; date timeIntervalSinceReferenceDate];
    let g = CFAbsoluteTimeGetGregorianDate(env, ti, nil);

    // Copy packed fields to locals (avoids E0793 unaligned ref).
    let year    = g.year;
    let month   = g.month as i32;
    let day     = g.day   as i32;
    let hours   = g.hours   as i32;
    let minutes = g.minutes as i32;
    let seconds = g.seconds as i64;

    // If no custom format, build one from style.
    let fmt_id = env.objc.borrow::<NSDateFormatterHostObject>(this).date_format;
    let fmt_str: String = if fmt_id != nil {
        ns_string::to_rust_string(env, fmt_id).into_owned()
    } else {
        let ds = env.objc.borrow::<NSDateFormatterHostObject>(this).date_style;
        let ts = env.objc.borrow::<NSDateFormatterHostObject>(this).time_style;
        style_to_format(ds.into(), ts.into())
    };

    let result = apply_format(&fmt_str, year, month, day, hours, minutes, seconds);
    log_dbg!("NSDateFormatter stringFromDate: {:?} => {:?}", fmt_str, result);
    let ns = ns_string::from_rust_string(env, result);
    autorelease(env, ns)
}

// MARK: - dateFromString:

- (id)dateFromString:(id)string { // NSString*
    if string == nil { return nil; }
    let s = ns_string::to_rust_string(env, string).into_owned();
    let fmt_id = env.objc.borrow::<NSDateFormatterHostObject>(this).date_format;
    if fmt_id == nil {
        log!("NSDateFormatter dateFromString: no format set, returning nil");
        return nil;
    }
    let fmt_str = ns_string::to_rust_string(env, fmt_id).into_owned();
    match parse_date(&fmt_str, &s) {
        Some(ti) => {
            let date: id = msg_class![env; NSDate dateWithTimeIntervalSinceReferenceDate:ti];
            date
        }
        None => {
            log!("NSDateFormatter dateFromString: couldn't parse {:?} with format {:?}", s, fmt_str);
            nil
        }
    }
}

// MARK: - getObjectValue:forString:range:error:

- (bool)getObjectValue:(crate::mem::MutPtr<id>)obj
             forString:(id)string
                 range:(crate::mem::MutVoidPtr)_range // NSRange*
                 error:(crate::mem::MutPtr<id>)_error {
    let date: id = msg![env; this dateFromString:string];
    if !obj.is_null() {
        env.mem.write(obj, date);
    }
    date != nil
}

// MARK: - NSCopying

- (id)copyWithZone:(NSZonePtr)_zone {
    let copy: id = msg_class![env; NSDateFormatter alloc];
    let fmt_id = env.objc.borrow::<NSDateFormatterHostObject>(this).date_format;
    let fmt_copy: id = if fmt_id != nil { msg![env; fmt_id copy] } else { nil };
    let host = env.objc.borrow::<NSDateFormatterHostObject>(this);
    let ds = host.date_style;
    let ts = host.time_style;
    let lenient = host.lenient;
    let uses_24hr = host.uses_24hr;
    let rel = host.does_relative_date_formatting;
    let behavior = host.formatter_behavior;
    // Reference types are retained-by-the-copy so the source instance still
    // owns them when it deallocs.
    let locale = host.locale;
    let tz = host.time_zone;
    let calendar = host.calendar;
    let tds = host.two_digit_start_date;
    let dd = host.default_date;
    let am = host.am_symbol;
    let pm = host.pm_symbol;
    let sms = host.short_month_symbols;
    let ms = host.month_symbols;
    let sws = host.short_weekday_symbols;
    let ws = host.weekday_symbols;
    let am_copy: id = if am != nil { msg![env; am copy] } else { nil };
    let pm_copy: id = if pm != nil { msg![env; pm copy] } else { nil };
    retain(env, locale);
    retain(env, tz);
    retain(env, calendar);
    retain(env, tds);
    retain(env, dd);
    retain(env, sms);
    retain(env, ms);
    retain(env, sws);
    retain(env, ws);
    let h = env.objc.borrow_mut::<NSDateFormatterHostObject>(copy);
    h.date_format = fmt_copy;
    h.date_style  = ds;
    h.time_style  = ts;
    h.lenient     = lenient;
    h.uses_24hr   = uses_24hr;
    h.formatter_behavior = behavior;
    h.does_relative_date_formatting = rel;
    h.locale      = locale;
    h.time_zone   = tz;
    h.calendar    = calendar;
    h.two_digit_start_date = tds;
    h.default_date         = dd;
    h.am_symbol            = am_copy;
    h.pm_symbol            = pm_copy;
    h.short_month_symbols  = sms;
    h.month_symbols        = ms;
    h.short_weekday_symbols = sws;
    h.weekday_symbols      = ws;
    autorelease(env, copy)
}

// MARK: - description

- (id)description {
    let fmt_id = env.objc.borrow::<NSDateFormatterHostObject>(this).date_format;
    let fmt_str = if fmt_id != nil {
        ns_string::to_rust_string(env, fmt_id).into_owned()
    } else {
        "<style-based>".to_string()
    };
    let s = format!("<NSDateFormatter: format={}>", fmt_str);
    let ns = ns_string::from_rust_string(env, s);
    autorelease(env, ns)
}

@end

};

// MARK: - Format helpers (outside macro)

/// Build a format string from date/time style constants.
fn style_to_format(date_style: u64, time_style: u64) -> String {
    let date_part = match date_style {
        1 => "M/d/yy",             // short
        2 => "MMM d, yyyy",        // medium
        3 => "MMMM d, yyyy",       // long
        4 => "EEEE, MMMM d, yyyy", // full
        _ => "",
    };
    let time_part = match time_style {
        1 => "h:mm a",         // short
        2 => "h:mm:ss a",      // medium
        3 => "h:mm:ss a z",    // long
        4 => "h:mm:ss a zzzz", // full
        _ => "",
    };
    match (date_part.is_empty(), time_part.is_empty()) {
        (false, false) => format!("{} {}", date_part, time_part),
        (false, true) => date_part.to_string(),
        (true, false) => time_part.to_string(),
        (true, true) => String::new(),
    }
}

/// Short day-of-week names (Sun=1 … Sat=7, but greg_date.day is day-of-month).
/// We compute weekday from year/month/day using Tomohiko Sakamoto's algorithm.
fn weekday(year: i32, month: i32, day: i32) -> u8 {
    // Returns 0=Sun, 1=Mon … 6=Sat
    static T: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let y = if month < 3 { year - 1 } else { year };
    ((y + y / 4 - y / 100 + y / 400 + T[(month - 1) as usize] + day) % 7) as u8
}

const SHORT_DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const LONG_DAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const SHORT_MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const LONG_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Apply a Unicode date format string to the given date components.
pub fn apply_format(
    fmt: &str,
    year: i32,
    month: i32,
    day: i32,
    hours: i32,
    minutes: i32,
    seconds: i64,
) -> String {
    let mut result = String::new();
    let mut chars = fmt.chars().peekable();
    let mut in_quotes = false;

    // 12-hour clock
    let hour12 = match hours % 12 {
        0 => 12,
        h => h,
    };
    let ampm = if hours < 12 { "AM" } else { "PM" };
    let ampm_l = ampm.to_lowercase();
    let wd = weekday(year, month, day);
    let m_idx = ((month - 1).clamp(0, 11)) as usize;
    let wd_idx = (wd as usize).clamp(0, 6);

    while let Some(c) = chars.next() {
        if c == '\'' {
            if chars.peek() == Some(&'\'') {
                result.push('\'');
                chars.next();
            } else {
                in_quotes = !in_quotes;
            }
            continue;
        }
        if in_quotes {
            result.push(c);
            continue;
        }
        if c.is_ascii_alphabetic() {
            let mut token = String::from(c);
            while chars.peek() == Some(&c) {
                token.push(chars.next().unwrap());
            }
            let s: String = match token.as_str() {
                // Era
                "G" | "GG" | "GGG" => "AD".into(),
                "GGGG" => "Anno Domini".into(),
                "GGGGG" => "A".into(),
                // Year
                "y" | "Y" => format!("{}", year),
                "yy" | "YY" => format!("{:02}", year % 100),
                "yyy" | "YYY" => format!("{:03}", year),
                "yyyy" | "YYYY" => format!("{:04}", year),
                // Quarter
                "Q" | "q" => format!("{}", (month - 1) / 3 + 1),
                "QQ" | "qq" => format!("{:02}", (month - 1) / 3 + 1),
                "QQQ" | "qqq" => format!("Q{}", (month - 1) / 3 + 1),
                "QQQQ" | "qqqq" => ["1st quarter", "2nd quarter", "3rd quarter", "4th quarter"]
                    [((month - 1) / 3) as usize]
                    .into(),
                // Month
                "M" | "L" => format!("{}", month),
                "MM" | "LL" => format!("{:02}", month),
                "MMM" | "LLL" => SHORT_MONTHS[m_idx].into(),
                "MMMM" | "LLLL" => LONG_MONTHS[m_idx].into(),
                "MMMMM" | "LLLLL" => SHORT_MONTHS[m_idx].chars().next().unwrap().to_string(),
                // Week of year / month (approximate)
                "w" | "ww" => format!("{}", (day + 6) / 7),
                "W" => format!("{}", (day - 1) / 7 + 1),
                // Day
                "d" => format!("{}", day),
                "dd" => format!("{:02}", day),
                "D" => {
                    // Day of year — approximate.
                    let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
                    let doy: i32 = days_in_month[..m_idx].iter().sum::<i32>() + day;
                    format!("{}", doy)
                }
                "F" => format!("{}", (day - 1) / 7 + 1), // day of week in month
                "g" => format!("{}", year * 365 + day),  // Julian day approx
                // Weekday
                "E" | "EE" | "EEE" | "eee" | "ccc" => SHORT_DAYS[wd_idx].into(),
                "EEEE" | "eeee" | "cccc" => LONG_DAYS[wd_idx].into(),
                "EEEEE" | "eeeee" | "ccccc" => {
                    SHORT_DAYS[wd_idx].chars().next().unwrap().to_string()
                }
                "e" | "ee" | "c" | "cc" => format!("{}", wd + 1),
                // Period
                "a" => ampm.into(),
                "aa" | "aaa" => ampm_l.clone(),
                // Hour
                "h" => format!("{}", hour12),
                "hh" => format!("{:02}", hour12),
                "H" => format!("{}", hours),
                "HH" => format!("{:02}", hours),
                "k" => format!("{}", if hours == 0 { 24 } else { hours }),
                "kk" => format!("{:02}", if hours == 0 { 24 } else { hours }),
                "K" => format!("{}", hours % 12),
                "KK" => format!("{:02}", hours % 12),
                // Minute
                "m" => format!("{}", minutes),
                "mm" => format!("{:02}", minutes),
                // Second
                "s" => format!("{}", seconds),
                "ss" => format!("{:02}", seconds),
                "S" | "SS" | "SSS" => "000".into(), // fractional seconds
                "A" => format!(
                    "{}",
                    hours * 3600000 + minutes * 60000 + seconds as i32 * 1000
                ),
                // Timezone
                "z" | "zz" | "zzz" | "zzzz" => "GMT".into(),
                "Z" | "ZZ" | "ZZZ" => "+0000".into(),
                "ZZZZ" => "GMT+00:00".into(),
                "ZZZZZ" => "+00:00".into(),
                "O" | "OO" | "OOO" => "GMT".into(),
                "OOOO" => "GMT+00:00".into(),
                "v" | "vv" | "vvv" | "vvvv" => "GMT".into(),
                "V" | "VV" => "UTC".into(),
                "VVV" => "Unknown City".into(),
                "VVVV" => "Coordinated Universal Time".into(),
                "X" | "x" => "+00".into(),
                "XX" | "xx" => "+0000".into(),
                "XXX" | "xxx" => "+00:00".into(),
                "XXXX" | "xxxx" => "+000000".into(),
                "XXXXX" | "xxxxx" => "+00:00:00".into(),
                other => {
                    log!("NSDateFormatter: unknown token {:?}", other);
                    other.to_string()
                }
            };
            result.push_str(&s);
        } else {
            result.push(c);
        }
    }
    result
}

/// Attempt to parse `s` according to `fmt`.
/// Returns `NSTimeInterval` (seconds since 2001-01-01) on success.
fn parse_date(fmt: &str, s: &str) -> Option<f64> {
    // Build a regex-like extractor by walking the format.
    // Supports the most common patterns apps use for parsing.
    let mut year = 2001i32;
    let mut month = 1i32;
    let mut day = 1i32;
    let mut hours = 0i32;
    let mut minutes = 0i32;
    let mut seconds = 0i32;

    let mut fmt_chars = fmt.chars().peekable();
    let mut s_chars = s.chars().peekable();

    while let Some(fc) = fmt_chars.next() {
        if fc == '\'' {
            // Skip quoted literal in format and matching chars in string.
            if fmt_chars.peek() == Some(&'\'') {
                fmt_chars.next();
                s_chars.next(); // skip matching ' in string
            } else {
                // consume until closing quote
                for qc in fmt_chars.by_ref() {
                    if qc == '\'' {
                        break;
                    }
                    s_chars.next(); // skip corresponding char
                }
            }
            continue;
        }
        if fc.is_ascii_alphabetic() {
            let mut token = String::from(fc);
            while fmt_chars.peek() == Some(&fc) {
                token.push(fmt_chars.next().unwrap());
            }
            // Read digits from s_chars.
            let mut num_str = String::new();
            while s_chars.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                num_str.push(s_chars.next().unwrap());
            }
            let num: i32 = num_str.parse().unwrap_or(0);
            match token.chars().next().unwrap() {
                'y' | 'Y' => {
                    year = if token.len() == 2 {
                        if num >= 70 {
                            1900 + num
                        } else {
                            2000 + num
                        }
                    } else {
                        num
                    }
                }
                'M' | 'L' => month = num,
                'd' => day = num,
                'H' | 'h' | 'k' | 'K' => hours = num,
                'm' => minutes = num,
                's' => seconds = num,
                _ => {} // skip other tokens
            }
        } else {
            // Literal — skip matching char in string.
            s_chars.next();
        }
    }

    // Convert to NSTimeInterval.
    // Rough seconds-since-2001 calculation.
    let days_since_epoch: i64 = {
        let y = year as i64;
        let m = month as i64;
        let d = day as i64;
        // Days from 2001-01-01 to year-month-day.
        let era_days = |yr: i64, mo: i64, dy: i64| -> i64 {
            let mo = if mo <= 2 { mo + 12 } else { mo };
            let yr = if mo <= 2 { yr - 1 } else { yr };
            let c = yr / 100;
            365 * yr + yr / 4 - c + c / 4 + (153 * mo + 2) / 5 + dy
        };
        era_days(y, m, d) - era_days(2001, 1, 1)
    };
    let ti = days_since_epoch as f64 * 86400.0
        + hours as f64 * 3600.0
        + minutes as f64 * 60.0
        + seconds as f64;
    Some(ti)
}
