/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Basic decimal `NSNumberFormatter` support.
//! TODO: locale-sensitive formatting and styles other than decimal and none.

use crate::frameworks::foundation::ns_string::{from_rust_string, to_rust_string};
use crate::frameworks::foundation::NSUInteger;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, ClassExports, HostObject,
    NSZonePtr,
};

type NSNumberFormatterStyle = NSUInteger;
const NSNumberFormatterNoStyle: NSNumberFormatterStyle = 0;
const NSNumberFormatterDecimalStyle: NSNumberFormatterStyle = 1;

struct NSNumberFormatterHostObject {
    style: NSNumberFormatterStyle,
    grouping: bool,
    separator: String,
    minimum_fraction_digits: usize,
    maximum_fraction_digits: usize,
    /// NSLocale*, copied. Formatting currently uses English conventions.
    locale: id,
}
impl HostObject for NSNumberFormatterHostObject {}
impl Default for NSNumberFormatterHostObject {
    fn default() -> Self {
        Self {
            style: NSNumberFormatterNoStyle,
            grouping: false,
            separator: ",".to_string(),
            minimum_fraction_digits: 0,
            maximum_fraction_digits: 0,
            locale: nil,
        }
    }
}

fn format_decimal(value: f64, minimum: usize, maximum: usize, separator: &str) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let mut text = format!("{value:.maximum$}");
    if maximum > 0 {
        while text.ends_with('0') && text.len() - text.find('.').unwrap() - 1 > minimum {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    group_decimal(text, separator)
}

fn group_decimal(mut text: String, separator: &str) -> String {
    if !separator.is_empty() {
        let end = text.find('.').unwrap_or(text.len());
        let start = usize::from(text.starts_with('-'));
        let mut pos = end;
        while pos > start + 3 {
            pos -= 3;
            text.insert_str(pos, separator);
        }
    }
    text
}

fn format_integer(mut text: String, minimum: usize, separator: &str) -> String {
    if minimum > 0 {
        text.push('.');
        text.extend(std::iter::repeat_n('0', minimum));
    }
    group_decimal(text, separator)
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSFormatter: NSObject
@end

@implementation NSNumberFormatter: NSFormatter

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<NSNumberFormatterHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)locale {
    let locale = env.objc.borrow::<NSNumberFormatterHostObject>(this).locale;
    if locale == nil {
        msg_class![env; NSLocale currentLocale]
    } else {
        locale
    }
}
- (())setLocale:(id)locale {
    let locale: id = msg![env; locale copy];
    let host_object = env.objc.borrow_mut::<NSNumberFormatterHostObject>(this);
    let old_locale = std::mem::replace(&mut host_object.locale, locale);
    release(env, old_locale);
}

- (())dealloc {
    let locale = env.objc.borrow::<NSNumberFormatterHostObject>(this).locale;
    release(env, locale);
    env.objc.dealloc_object(this, &mut env.mem);
}

- (NSNumberFormatterStyle)numberStyle {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).style
}
- (())setNumberStyle:(NSNumberFormatterStyle)style {
    assert!(matches!(style, NSNumberFormatterNoStyle | NSNumberFormatterDecimalStyle),
        "Unsupported NSNumberFormatter style {style}");
    let obj = env.objc.borrow_mut::<NSNumberFormatterHostObject>(this);
    obj.style = style;
    obj.grouping = style == NSNumberFormatterDecimalStyle;
    let maximum = if style == NSNumberFormatterDecimalStyle { 3 } else { 0 };
    obj.maximum_fraction_digits = maximum.max(obj.minimum_fraction_digits);
}

- (bool)usesGroupingSeparator {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).grouping
}
- (())setUsesGroupingSeparator:(bool)grouping {
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).grouping = grouping;
}

- (id)groupingSeparator {
    let separator = env.objc.borrow::<NSNumberFormatterHostObject>(this).separator.clone();
    let result = from_rust_string(env, separator);
    autorelease(env, result)
}
- (())setGroupingSeparator:(id)separator {
    let separator = to_rust_string(env, separator).into_owned();
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).separator = separator;
}

- (NSUInteger)minimumFractionDigits {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).minimum_fraction_digits as NSUInteger
}
- (())setMinimumFractionDigits:(NSUInteger)digits {
    let obj = env.objc.borrow_mut::<NSNumberFormatterHostObject>(this);
    obj.minimum_fraction_digits = digits as usize;
    obj.maximum_fraction_digits = obj.maximum_fraction_digits.max(digits as usize);
}

- (NSUInteger)maximumFractionDigits {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).maximum_fraction_digits as NSUInteger
}
- (())setMaximumFractionDigits:(NSUInteger)digits {
    let obj = env.objc.borrow_mut::<NSNumberFormatterHostObject>(this);
    obj.maximum_fraction_digits = digits as usize;
    obj.minimum_fraction_digits = obj.minimum_fraction_digits.min(digits as usize);
}

- (id)stringFromNumber:(id)number {
    if number == nil {
        return nil;
    }
    let string: id = msg![env; number stringValue];
    let string = to_rust_string(env, string).into_owned();
    let obj = env.objc.borrow::<NSNumberFormatterHostObject>(this);
    let separator = if obj.grouping { &obj.separator } else { "" };
    // Preserve all digits of 64-bit integers instead of converting to f64.
    let text = if string.parse::<i128>().is_ok() {
        format_integer(string, obj.minimum_fraction_digits, separator)
    } else {
        let value: f64 = msg![env; number doubleValue];
        let obj = env.objc.borrow::<NSNumberFormatterHostObject>(this);
        format_decimal(
            value, obj.minimum_fraction_digits, obj.maximum_fraction_digits,
            if obj.grouping { &obj.separator } else { "" },
        )
    };
    let result = from_rust_string(env, text);
    autorelease(env, result)
}

@end

};

#[cfg(test)]
mod tests {
    use super::{format_decimal, format_integer, group_decimal};

    #[test]
    fn decimal_grouping_and_rounding() {
        assert_eq!(format_decimal(1234567.5, 0, 3, ","), "1,234,567.5");
        assert_eq!(format_decimal(-1234.5, 2, 2, ","), "-1,234.50");
        assert_eq!(format_decimal(999.999, 0, 2, ","), "1,000");
        assert_eq!(format_decimal(12.0, 0, 0, ""), "12");
        assert_eq!(
            format_decimal(1234567.0, 0, 0, "\u{202f}"),
            "1\u{202f}234\u{202f}567"
        );
    }

    #[test]
    fn integer_grouping_preserves_precision() {
        assert_eq!(
            group_decimal(i64::MAX.to_string(), ","),
            "9,223,372,036,854,775,807"
        );
        assert_eq!(
            group_decimal(i64::MIN.to_string(), ","),
            "-9,223,372,036,854,775,808"
        );
        assert_eq!(
            group_decimal(u64::MAX.to_string(), ","),
            "18,446,744,073,709,551,615"
        );
        assert_eq!(
            format_integer(i64::MAX.to_string(), 2, ","),
            "9,223,372,036,854,775,807.00"
        );
    }
}
