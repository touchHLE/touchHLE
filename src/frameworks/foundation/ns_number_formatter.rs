/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSNumberFormatter`.
//!
//! Resources:
//! - Apple's [NSNumberFormatter Class Reference](https://developer.apple.com/documentation/foundation/nsnumberformatter)

use crate::frameworks::foundation::{ns_string, NSUInteger};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};

/// `NSNumberFormatterStyle` enum values
type NSNumberFormatterStyle = NSUInteger;
const NSNumberFormatterNoStyle: NSNumberFormatterStyle = 0;
const NSNumberFormatterDecimalStyle: NSNumberFormatterStyle = 1;
const NSNumberFormatterCurrencyStyle: NSNumberFormatterStyle = 2;
const NSNumberFormatterPercentStyle: NSNumberFormatterStyle = 3;
const NSNumberFormatterScientificStyle: NSNumberFormatterStyle = 4;
const NSNumberFormatterSpellOutStyle: NSNumberFormatterStyle = 5;

/// `NSNumberFormatterRoundingMode` enum values
type NSNumberFormatterRoundingMode = NSUInteger;
#[allow(dead_code)]
const NSNumberFormatterRoundCeiling: NSNumberFormatterRoundingMode = 0;
#[allow(dead_code)]
const NSNumberFormatterRoundFloor: NSNumberFormatterRoundingMode = 1;
#[allow(dead_code)]
const NSNumberFormatterRoundDown: NSNumberFormatterRoundingMode = 2;
#[allow(dead_code)]
const NSNumberFormatterRoundUp: NSNumberFormatterRoundingMode = 3;
#[allow(dead_code)]
const NSNumberFormatterRoundHalfEven: NSNumberFormatterRoundingMode = 4;
#[allow(dead_code)]
const NSNumberFormatterRoundHalfDown: NSNumberFormatterRoundingMode = 5;
const NSNumberFormatterRoundHalfUp: NSNumberFormatterRoundingMode = 6;

struct NSNumberFormatterHostObject {
    number_style: NSNumberFormatterStyle,
    /// `NSLocale *`
    locale: id,
    /// `NSString *` (grouping separator)
    grouping_separator: id,
    uses_grouping_separator: bool,
    /// `NSString *` (decimal separator)
    decimal_separator: id,
    /// `NSString *` (currency symbol)
    currency_symbol: id,
    /// `NSString *` (positive prefix)
    positive_prefix: id,
    /// `NSString *` (negative prefix)
    negative_prefix: id,
    /// `NSString *` (positive suffix)
    positive_suffix: id,
    /// `NSString *` (negative suffix)
    negative_suffix: id,
    minimum_fraction_digits: NSUInteger,
    maximum_fraction_digits: NSUInteger,
    minimum_integer_digits: NSUInteger,
    rounding_mode: NSNumberFormatterRoundingMode,
    /// `NSNumber *` (multiplier)
    multiplier: id,
    generates_decimal_numbers: bool,
}
impl HostObject for NSNumberFormatterHostObject {}

/// Format a floating-point number with the given fraction digit counts.
fn format_double(
    value: f64,
    min_fraction: usize,
    max_fraction: usize,
    min_integer: usize,
    decimal_sep: &str,
    group_sep: &str,
    uses_grouping: bool,
) -> String {
    // Format integer part with grouping separators
    let is_negative = value < 0.0;
    let abs_value = value.abs();

    // Round to max_fraction decimal places
    let factor = 10f64.powi(max_fraction as i32);
    let rounded = (abs_value * factor).round() / factor;

    // Split integer and fractional parts
    let integer_part = rounded.floor() as u64;
    let frac_part_raw = rounded - integer_part as f64;

    // Build integer string with leading zeros
    let int_str = format!("{}", integer_part);
    let int_str = if int_str.len() < min_integer {
        format!("{:0>width$}", int_str, width = min_integer)
    } else {
        int_str
    };

    // Apply grouping separators if needed
    let int_with_groups = if uses_grouping && !group_sep.is_empty() {
        let chars: Vec<char> = int_str.chars().collect();
        let mut result = String::new();
        let len = chars.len();
        for (i, c) in chars.iter().enumerate() {
            if i != 0 && (len - i) % 3 == 0 {
                result.push_str(group_sep);
            }
            result.push(*c);
        }
        result
    } else {
        int_str
    };

    // Build fraction string
    let frac_str = if max_fraction == 0 {
        String::new()
    } else {
        // Get exactly max_fraction digits
        let frac_digits = format!("{:.prec$}", frac_part_raw, prec = max_fraction);
        // Remove leading "0." prefix
        let digits = &frac_digits[2..]; // skip "0."
        // Trim trailing zeros down to min_fraction
        let trimmed = digits.trim_end_matches('0');
        let effective_len = trimmed.len().max(min_fraction);
        let digits = &digits[..effective_len.min(digits.len())];
        if digits.is_empty() && min_fraction == 0 {
            String::new()
        } else {
            format!("{}{}", decimal_sep, digits)
        }
    };

    let sign = if is_negative { "-" } else { "" };
    format!("{}{}{}", sign, int_with_groups, frac_str)
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSNumberFormatter: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    // Defaults mirror NSNumberFormatterNoStyle on iOS
    let host_object = Box::new(NSNumberFormatterHostObject {
        number_style: NSNumberFormatterNoStyle,
        locale: nil,
        grouping_separator: nil,
        uses_grouping_separator: false,
        decimal_separator: nil,
        currency_symbol: nil,
        positive_prefix: nil,
        negative_prefix: nil,
        positive_suffix: nil,
        negative_suffix: nil,
        minimum_fraction_digits: 0,
        maximum_fraction_digits: 6,
        minimum_integer_digits: 0,
        rounding_mode: NSNumberFormatterRoundHalfUp,
        multiplier: nil,
        generates_decimal_numbers: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// Convenience class method
+ (id)localizedStringFromNumber:(id)number // NSNumber *
                    numberStyle:(NSNumberFormatterStyle)style {
    let formatter: id = msg![env; this new];
    let formatter: id = msg![env; formatter autorelease];
    () = msg![env; formatter setNumberStyle:style];
    msg![env; formatter stringFromNumber:number]
}

- (id)init {
    this
}

- (())dealloc {
    let host = env.objc.borrow::<NSNumberFormatterHostObject>(this);
    let locale = host.locale;
    let grouping_separator = host.grouping_separator;
    let decimal_separator = host.decimal_separator;
    let currency_symbol = host.currency_symbol;
    let positive_prefix = host.positive_prefix;
    let negative_prefix = host.negative_prefix;
    let positive_suffix = host.positive_suffix;
    let negative_suffix = host.negative_suffix;
    let multiplier = host.multiplier;
    release(env, locale);
    release(env, grouping_separator);
    release(env, decimal_separator);
    release(env, currency_symbol);
    release(env, positive_prefix);
    release(env, negative_prefix);
    release(env, positive_suffix);
    release(env, negative_suffix);
    release(env, multiplier);
    env.objc.dealloc_object(this, &mut env.mem)
}

// NSCopying
- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

// ============================================================
// Style
// ============================================================

- (NSNumberFormatterStyle)numberStyle {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).number_style
}
- (())setNumberStyle:(NSNumberFormatterStyle)style {
    let host = env.objc.borrow_mut::<NSNumberFormatterHostObject>(this);
    host.number_style = style;
    // Apply defaults based on style
    match style {
        NSNumberFormatterDecimalStyle => {
            host.uses_grouping_separator = true;
            host.minimum_fraction_digits = 0;
            host.maximum_fraction_digits = 6;
        }
        NSNumberFormatterCurrencyStyle => {
            host.uses_grouping_separator = true;
            host.minimum_fraction_digits = 2;
            host.maximum_fraction_digits = 2;
        }
        NSNumberFormatterPercentStyle => {
            host.uses_grouping_separator = true;
            host.minimum_fraction_digits = 0;
            host.maximum_fraction_digits = 0;
        }
        NSNumberFormatterScientificStyle => {
            host.minimum_fraction_digits = 0;
            host.maximum_fraction_digits = 6;
        }
        _ => {}
    }
}

// ============================================================
// Locale
// ============================================================

- (id)locale {
    let locale = env.objc.borrow::<NSNumberFormatterHostObject>(this).locale;
    if locale == nil {
        msg_class![env; NSLocale currentLocale]
    } else {
        locale
    }
}
- (())setLocale:(id)locale { // NSLocale *
    let old = env.objc.borrow::<NSNumberFormatterHostObject>(this).locale;
    release(env, old);
    retain(env, locale);
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).locale = locale;
}

// ============================================================
// Separators
// ============================================================

- (id)decimalSeparator {
    let sep = env.objc.borrow::<NSNumberFormatterHostObject>(this).decimal_separator;
    if sep == nil {
        ns_string::get_static_str(env, ".")
    } else {
        sep
    }
}
- (())setDecimalSeparator:(id)sep { // NSString *
    let old = env.objc.borrow::<NSNumberFormatterHostObject>(this).decimal_separator;
    release(env, old);
    let new_sep: id = msg![env; sep copy];
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).decimal_separator = new_sep;
}

- (id)groupingSeparator {
    let sep = env.objc.borrow::<NSNumberFormatterHostObject>(this).grouping_separator;
    if sep == nil {
        ns_string::get_static_str(env, ",")
    } else {
        sep
    }
}
- (())setGroupingSeparator:(id)sep { // NSString *
    let old = env.objc.borrow::<NSNumberFormatterHostObject>(this).grouping_separator;
    release(env, old);
    let new_sep: id = msg![env; sep copy];
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).grouping_separator = new_sep;
}

- (bool)usesGroupingSeparator {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).uses_grouping_separator
}
- (())setUsesGroupingSeparator:(bool)flag {
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).uses_grouping_separator = flag;
}

// ============================================================
// Currency symbol
// ============================================================

- (id)currencySymbol {
    let sym = env.objc.borrow::<NSNumberFormatterHostObject>(this).currency_symbol;
    if sym == nil {
        ns_string::get_static_str(env, "$")
    } else {
        sym
    }
}
- (())setCurrencySymbol:(id)sym { // NSString *
    let old = env.objc.borrow::<NSNumberFormatterHostObject>(this).currency_symbol;
    release(env, old);
    let new_sym: id = msg![env; sym copy];
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).currency_symbol = new_sym;
}

// ============================================================
// Prefixes / Suffixes
// ============================================================

- (id)positivePrefix {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).positive_prefix
}
- (())setPositivePrefix:(id)prefix {
    let old = env.objc.borrow::<NSNumberFormatterHostObject>(this).positive_prefix;
    release(env, old);
    let new_v: id = msg![env; prefix copy];
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).positive_prefix = new_v;
}

- (id)negativePrefix {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).negative_prefix
}
- (())setNegativePrefix:(id)prefix {
    let old = env.objc.borrow::<NSNumberFormatterHostObject>(this).negative_prefix;
    release(env, old);
    let new_v: id = msg![env; prefix copy];
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).negative_prefix = new_v;
}

- (id)positiveSuffix {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).positive_suffix
}
- (())setPositiveSuffix:(id)suffix {
    let old = env.objc.borrow::<NSNumberFormatterHostObject>(this).positive_suffix;
    release(env, old);
    let new_v: id = msg![env; suffix copy];
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).positive_suffix = new_v;
}

- (id)negativeSuffix {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).negative_suffix
}
- (())setNegativeSuffix:(id)suffix {
    let old = env.objc.borrow::<NSNumberFormatterHostObject>(this).negative_suffix;
    release(env, old);
    let new_v: id = msg![env; suffix copy];
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).negative_suffix = new_v;
}

// ============================================================
// Fraction / Integer digit counts
// ============================================================

- (NSUInteger)minimumFractionDigits {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).minimum_fraction_digits
}
- (())setMinimumFractionDigits:(NSUInteger)n {
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).minimum_fraction_digits = n;
}

- (NSUInteger)maximumFractionDigits {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).maximum_fraction_digits
}
- (())setMaximumFractionDigits:(NSUInteger)n {
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).maximum_fraction_digits = n;
}

- (NSUInteger)minimumIntegerDigits {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).minimum_integer_digits
}
- (())setMinimumIntegerDigits:(NSUInteger)n {
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).minimum_integer_digits = n;
}

// ============================================================
// Rounding
// ============================================================

- (NSNumberFormatterRoundingMode)roundingMode {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).rounding_mode
}
- (())setRoundingMode:(NSNumberFormatterRoundingMode)mode {
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).rounding_mode = mode;
}

// ============================================================
// Multiplier
// ============================================================

- (id)multiplier { // NSNumber *
    env.objc.borrow::<NSNumberFormatterHostObject>(this).multiplier
}
- (())setMultiplier:(id)multiplier { // NSNumber *
    let old = env.objc.borrow::<NSNumberFormatterHostObject>(this).multiplier;
    release(env, old);
    retain(env, multiplier);
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).multiplier = multiplier;
}

// ============================================================
// Generates decimal numbers
// ============================================================

- (bool)generatesDecimalNumbers {
    env.objc.borrow::<NSNumberFormatterHostObject>(this).generates_decimal_numbers
}
- (())setGeneratesDecimalNumbers:(bool)flag {
    env.objc.borrow_mut::<NSNumberFormatterHostObject>(this).generates_decimal_numbers = flag;
}

// ============================================================
// Number <-> String conversion
// ============================================================

- (id)stringFromNumber:(id)number { // NSNumber *
    if number == nil {
        return nil;
    }

    let host = env.objc.borrow::<NSNumberFormatterHostObject>(this);
    let style = host.number_style;
    let min_frac = host.minimum_fraction_digits as usize;
    let max_frac = host.maximum_fraction_digits as usize;
    let min_int = host.minimum_integer_digits as usize;
    let uses_grouping = host.uses_grouping_separator;
    let multiplier = host.multiplier;
    let positive_prefix = host.positive_prefix;
    let negative_prefix = host.negative_prefix;
    let positive_suffix = host.positive_suffix;
    let negative_suffix = host.negative_suffix;
    let decimal_sep_obj = host.decimal_separator;
    let grouping_sep_obj = host.grouping_separator;
    let currency_sym_obj = host.currency_symbol;
    let _ = host;

    // Get decimal value
    let mut value: f64 = msg![env; number doubleValue];

    // Apply multiplier if set
    if multiplier != nil {
        let mult: f64 = msg![env; multiplier doubleValue];
        value *= mult;
    }

    let decimal_sep = if decimal_sep_obj == nil {
        ".".to_string()
    } else {
        ns_string::to_rust_string(env, decimal_sep_obj).to_string()
    };
    let grouping_sep = if grouping_sep_obj == nil {
        ",".to_string()
    } else {
        ns_string::to_rust_string(env, grouping_sep_obj).to_string()
    };
    let currency_sym = if currency_sym_obj == nil {
        "$".to_string()
    } else {
        ns_string::to_rust_string(env, currency_sym_obj).to_string()
    };

    let formatted = match style {
        NSNumberFormatterNoStyle => {
            // No style: format as integer
            format!("{}", value as i64)
        }
        NSNumberFormatterDecimalStyle => {
            format_double(value, min_frac, max_frac, min_int, &decimal_sep, &grouping_sep, uses_grouping)
        }
        NSNumberFormatterCurrencyStyle => {
            let num_str = format_double(value.abs(), min_frac, max_frac, min_int, &decimal_sep, &grouping_sep, uses_grouping);
            if value < 0.0 {
                format!("-{}{}", currency_sym, num_str)
            } else {
                format!("{}{}", currency_sym, num_str)
            }
        }
        NSNumberFormatterPercentStyle => {
            // Multiply by 100 if no multiplier was set
            let pct_val = if multiplier == nil { value * 100.0 } else { value };
            let num_str = format_double(pct_val, min_frac, max_frac, min_int, &decimal_sep, &grouping_sep, uses_grouping);
            format!("{}%", num_str)
        }
        NSNumberFormatterScientificStyle => {
            format!("{:e}", value)
        }
        NSNumberFormatterSpellOutStyle => {
            // Very basic spell-out: just use decimal representation
            log!("Warning: NSNumberFormatterSpellOutStyle not fully implemented, falling back to decimal");
            format_double(value, min_frac, max_frac, min_int, &decimal_sep, &grouping_sep, uses_grouping)
        }
        _ => {
            log!("Warning: Unknown NSNumberFormatterStyle {}, using decimal fallback", style);
            format_double(value, min_frac, max_frac, min_int, &decimal_sep, &grouping_sep, uses_grouping)
        }
    };

    // Apply custom prefixes/suffixes if set
    let formatted = if positive_prefix != nil || negative_prefix != nil || positive_suffix != nil || negative_suffix != nil {
        let prefix = if value < 0.0 {
            if negative_prefix != nil { ns_string::to_rust_string(env, negative_prefix).to_string() } else { String::new() }
        } else {
            if positive_prefix != nil { ns_string::to_rust_string(env, positive_prefix).to_string() } else { String::new() }
        };
        let suffix = if value < 0.0 {
            if negative_suffix != nil { ns_string::to_rust_string(env, negative_suffix).to_string() } else { String::new() }
        } else {
            if positive_suffix != nil { ns_string::to_rust_string(env, positive_suffix).to_string() } else { String::new() }
        };
        format!("{}{}{}", prefix, formatted, suffix)
    } else {
        formatted
    };

    log_dbg!("NSNumberFormatter stringFromNumber: {} => \"{}\"", value, formatted);
    let result = ns_string::from_rust_string(env, formatted);
    autorelease(env, result)
}

- (id)numberFromString:(id)string { // NSString *
    if string == nil {
        return nil;
    }
    let s = ns_string::to_rust_string(env, string);
    let s = s.trim();

    // Strip currency symbol, percent sign, and whitespace for parsing
    let s = s.trim_start_matches('$').trim_end_matches('%').trim();
    // Replace grouping separators
    let s = s.replace(',', "");

    if let Ok(val) = s.parse::<i64>() {
        let result: id = msg_class![env; NSNumber numberWithLongLong:val];
        return result;
    }
    if let Ok(val) = s.parse::<f64>() {
        let result: id = msg_class![env; NSNumber numberWithDouble:val];
        return result;
    }

    log!("Warning: NSNumberFormatter numberFromString: could not parse {:?}", string);
    nil
}

@end

};
