/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use super::{ns_string, NSRange, NSUInteger};
use crate::mem::MutPtr;
use crate::msg;
use crate::objc::{
    autorelease, id, msg_class, nil, objc_classes, release, ClassExports, HostObject, NSZonePtr,
};
use regex::Regex;

use super::ns_text_checking_result;

/// Хост-объект для хранения скомпилированного регулярного выражения.
#[derive(Default)]
struct NSRegularExpressionHostObject {
    regex: Option<Regex>,
}
impl HostObject for NSRegularExpressionHostObject {}

pub const CLASSES: ClassExports = objc_classes! {
    (env, this, _cmd);

    @implementation NSRegularExpression: NSObject

    + (id)allocWithZone:(NSZonePtr)_zone {
        let host_object = Box::new(NSRegularExpressionHostObject { regex: None });
        env.objc.alloc_object(this, host_object, &mut env.mem)
    }

    // + (NSRegularExpression *)
    //     regularExpressionWithPattern:(NSString *)pattern
    //     options:(NSRegularExpressionOptions)options
    //     error:(NSError **)error
    + (id)regularExpressionWithPattern:(id)pattern
                               options:(u32)options
                                 error:(MutPtr<id>)error {
        let new: id = msg![env; this alloc];
        let new: id = msg![env; new initWithPattern:pattern
                                            options:options
                                              error:error];
        if new == nil {
            return nil;
        }
        autorelease(env, new)
    }

    // - (id)initWithPattern:(NSString *)pattern
    //              options:(NSRegularExpressionOptions)options
    //                error:(NSError **)error
    - (id)initWithPattern:(id)pattern
                  options:(u32)_options
                    error:(MutPtr<id>)_error {
        if pattern == nil {
            release(env, this);
            return nil;
        }
        let pattern_str = ns_string::to_rust_string(env, pattern).into_owned();
        match Regex::new(&pattern_str) {
            Ok(re) => {
                env.objc
                    .borrow_mut::<NSRegularExpressionHostObject>(this)
                    .regex = Some(re);
                this
            }
            Err(e) => {
                log!(
                    "NSRegularExpression: failed to compile pattern '{}': {}",
                    pattern_str,
                    e
                );
                // В полноценной реализации здесь нужно создавать NSError, но
                // пока возвращаем nil и освобождаем приёмник, как и положено
                // по соглашению Cocoa для неудавшегося -init.
                release(env, this);
                nil
            }
        }
    }

    // - (NSUInteger)numberOfMatchesInString:(NSString *)string
    //                              options:(NSMatchingOptions)options
    //                                range:(NSRange)range
    - (NSUInteger)numberOfMatchesInString:(id)string
                                  options:(u32)_options
                                    range:(NSRange)range {
        if string == nil {
            return 0;
        }
        let full_text = ns_string::to_rust_string(env, string);
        // `NSRange` is in UTF-16 code units, but `full_text` is UTF-8: a naïve
        // `&full_text[start..end]` panics for non-ASCII strings or for ranges
        // that fall outside the string. Convert the range to a byte slice
        // safely; bail out (returning 0 matches) on bad input rather than
        // taking down the whole emulator.
        let Some((start_byte, end_byte)) =
            utf16_range_to_utf8_byte_range(&full_text, range)
        else {
            return 0;
        };

        let target_text = &full_text[start_byte..end_byte];
        let host_obj = env.objc.borrow::<NSRegularExpressionHostObject>(this);
        if let Some(re) = &host_obj.regex {
            re.find_iter(target_text).count() as NSUInteger
        } else {
            0
        }
    }

    // - (NSTextCheckingResult *)firstMatchInString:(NSString *)string
    //                                     options:(NSMatchingOptions)options
    //                                       range:(NSRange)range
    - (id)firstMatchInString:(id)string
                     options:(u32)_options
                       range:(NSRange)range {
        if string == nil {
            return nil;
        }
        let full_text = ns_string::to_rust_string(env, string);
        let range_location = range.location;
        let Some((start_byte, end_byte)) =
            utf16_range_to_utf8_byte_range(&full_text, range)
        else {
            return nil;
        };
        let target_text = &full_text[start_byte..end_byte];
        // Clone the regex so we can release the borrow on env before calling
        // from_ranges (which needs &mut env).
        let re_clone = {
            let host_obj = env.objc.borrow::<NSRegularExpressionHostObject>(this);
            match &host_obj.regex {
                Some(re) => re.clone(),
                None => return nil,
            }
        };

        let Some(caps) = re_clone.captures(target_text) else {
            return nil;
        };

        let ranges = build_capture_ranges(&full_text, &caps, start_byte, range_location);
        ns_text_checking_result::from_ranges(env, ranges)
    }

    // - (NSArray *)matchesInString:(NSString *)string
    //                      options:(NSMatchingOptions)options
    //                        range:(NSRange)range
    - (id)matchesInString:(id)string
                  options:(u32)_options
                    range:(NSRange)range {
        if string == nil {
            let empty: id = msg_class![env; NSArray array];
            return empty;
        }
        let full_text = ns_string::to_rust_string(env, string);
        let range_location = range.location;
        let Some((start_byte, end_byte)) =
            utf16_range_to_utf8_byte_range(&full_text, range)
        else {
            let empty: id = msg_class![env; NSArray array];
            return empty;
        };
        let target_text = &full_text[start_byte..end_byte];
        // Clone regex to release the borrow before calling env-mutating functions.
        let re_clone = {
            let host_obj = env.objc.borrow::<NSRegularExpressionHostObject>(this);
            match &host_obj.regex {
                Some(re) => re.clone(),
                None => {
                    let empty: id = msg_class![env; NSArray array];
                    return empty;
                }
            }
        };

        // Collect all capture results into byte-level data first
        let all_ranges: Vec<Vec<NSRange>> = re_clone.captures_iter(target_text)
            .map(|caps| build_capture_ranges(&full_text, &caps, start_byte, range_location))
            .collect();

        let mut results: Vec<id> = Vec::new();
        for ranges in all_ranges {
            let result = ns_text_checking_result::from_ranges(env, ranges);
            crate::objc::retain(env, result);
            results.push(result);
        }

        let arr = crate::frameworks::foundation::ns_array::from_vec(env, results);
        autorelease(env, arr)
    }

    // - (NSString *)pattern
    - (id)pattern {
        let pat_string = {
            let host_obj = env.objc.borrow::<NSRegularExpressionHostObject>(this);
            if let Some(re) = &host_obj.regex {
                re.as_str().to_string()
            } else {
                String::new()
            }
        };
        let s = ns_string::from_rust_string(env, pat_string);
        autorelease(env, s)
    }

    // - (NSUInteger)numberOfCaptureGroups
    - (NSUInteger)numberOfCaptureGroups {
        let host_obj = env.objc.borrow::<NSRegularExpressionHostObject>(this);
        if let Some(re) = &host_obj.regex {
            re.captures_len().saturating_sub(1) as NSUInteger
        } else {
            0
        }
    }

    // - (NSString *)stringByReplacingMatchesInString:(NSString *)string
    //                                       options:(NSMatchingOptions)options
    //                                         range:(NSRange)range
    //                                  withTemplate:(NSString *)templ
    - (id)stringByReplacingMatchesInString:(id)string
                                   options:(u32)_options
                                     range:(NSRange)range
                              withTemplate:(id)templ {
        if string == nil {
            return nil;
        }
        let full_text = ns_string::to_rust_string(env, string).into_owned();
        let template_str = if templ != nil {
            ns_string::to_rust_string(env, templ).into_owned()
        } else {
            String::new()
        };
        let Some((start_byte, end_byte)) =
            utf16_range_to_utf8_byte_range(&full_text, range)
        else {
            let ns = ns_string::from_rust_string(env, full_text);
            return autorelease(env, ns);
        };
        // Clone regex to release borrow before using env for string operations
        let re_clone = {
            let host_obj = env.objc.borrow::<NSRegularExpressionHostObject>(this);
            match &host_obj.regex {
                Some(re) => re.clone(),
                None => {
                    let ns = ns_string::from_rust_string(env, full_text);
                    return autorelease(env, ns);
                }
            }
        };

        // Only replace within the specified range
        let prefix = &full_text[..start_byte];
        let target = &full_text[start_byte..end_byte];
        let suffix = &full_text[end_byte..];

        let replaced = re_clone.replace_all(target, template_str.as_str());
        let result = format!("{}{}{}", prefix, replaced, suffix);
        let ns = ns_string::from_rust_string(env, result);
        autorelease(env, ns)
    }

    @end
};

/// Convert an `NSRange` (UTF-16 code units) into a byte range inside a UTF-8
/// string. Returns `None` if the range falls outside the string.
fn utf16_range_to_utf8_byte_range(s: &str, range: NSRange) -> Option<(usize, usize)> {
    let start_units = range.location as usize;
    let end_units = (range.location as usize).checked_add(range.length as usize)?;

    let mut units_seen: usize = 0;
    let mut start_byte: Option<usize> = None;
    let mut end_byte: Option<usize> = None;

    for (byte_offset, c) in s.char_indices() {
        if units_seen == start_units && start_byte.is_none() {
            start_byte = Some(byte_offset);
        }
        if units_seen == end_units && end_byte.is_none() {
            end_byte = Some(byte_offset);
        }
        units_seen += c.len_utf16();
    }
    if units_seen == start_units && start_byte.is_none() {
        start_byte = Some(s.len());
    }
    if units_seen == end_units && end_byte.is_none() {
        end_byte = Some(s.len());
    }
    let start_byte = start_byte?;
    let end_byte = end_byte?;
    if start_byte > end_byte {
        return None;
    }
    Some((start_byte, end_byte))
}

/// Build an NSRange vector from regex captures. Converts byte offsets within
/// `target_text` (which starts at `target_start_byte` within the full string)
/// into UTF-16 code unit ranges relative to the start of the full string.
/// `search_location_utf16` is the NSRange.location passed to the search method.
fn build_capture_ranges(
    full_text: &str,
    caps: &regex::Captures<'_>,
    target_start_byte: usize,
    _search_location_utf16: u32,
) -> Vec<NSRange> {
    let mut ranges = Vec::with_capacity(caps.len());
    for i in 0..caps.len() {
        if let Some(m) = caps.get(i) {
            // m.start()/m.end() are byte offsets relative to target_text.
            // Convert to byte offsets in full_text:
            let abs_start_byte = target_start_byte + m.start();
            let abs_end_byte = target_start_byte + m.end();
            // Convert byte offsets to UTF-16 code unit offsets
            let utf16_start = byte_offset_to_utf16(full_text, abs_start_byte);
            let utf16_end = byte_offset_to_utf16(full_text, abs_end_byte);
            ranges.push(NSRange {
                location: utf16_start as u32,
                length: (utf16_end - utf16_start) as u32,
            });
        } else {
            // Capture group didn't participate — {NSNotFound, 0}
            ranges.push(NSRange {
                location: 0x7fffffff,
                length: 0,
            });
        }
    }
    ranges
}

/// Convert a byte offset in a UTF-8 string to a UTF-16 code unit offset.
fn byte_offset_to_utf16(s: &str, byte_offset: usize) -> usize {
    s[..byte_offset].chars().map(|c| c.len_utf16()).sum()
}

#[cfg(test)]
mod tests {
    use super::utf16_range_to_utf8_byte_range;
    use super::NSRange;

    #[test]
    fn ascii_range_matches_byte_range() {
        let s = "hello world";
        let range = NSRange {
            location: 6,
            length: 5,
        };
        assert_eq!(utf16_range_to_utf8_byte_range(s, range), Some((6, 11)));
    }

    #[test]
    fn non_ascii_does_not_panic() {
        // "café" — 'é' is one UTF-16 code unit but 2 UTF-8 bytes.
        let s = "café";
        let range = NSRange {
            location: 0,
            length: 4,
        };
        assert_eq!(utf16_range_to_utf8_byte_range(s, range), Some((0, 5)));
    }

    #[test]
    fn out_of_bounds_returns_none() {
        let s = "abc";
        let range = NSRange {
            location: 0,
            length: 100,
        };
        assert_eq!(utf16_range_to_utf8_byte_range(s, range), None);
    }

    #[test]
    fn surrogate_pair_counted_as_two_units() {
        // U+1F600 is a single char but two UTF-16 code units and 4 UTF-8 bytes.
        let s = "a\u{1F600}b";
        let range = NSRange {
            location: 0,
            length: 3,
        };
        assert_eq!(utf16_range_to_utf8_byte_range(s, range), Some((0, 5)));
        let just_the_emoji = NSRange {
            location: 1,
            length: 2,
        };
        assert_eq!(
            utf16_range_to_utf8_byte_range(s, just_the_emoji),
            Some((1, 5))
        );
    }
}
