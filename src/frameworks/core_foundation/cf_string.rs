/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFString` and `CFMutableString`.
//!
//! This is toll-free bridged to `NSString` and `NSMutableString` in
//! Apple's implementation. Here it is the same type.

use super::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use super::cf_dictionary::CFDictionaryRef;
use super::cf_locale::CFLocaleRef;
use super::{kCFNotFound, CFComparisonResult, CFIndex, CFOptionFlags, CFRange};
use crate::abi::{DotDotDot, VaList};
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::{ns_string, unichar, NSNotFound, NSRange, NSUInteger};
use crate::libc::string::strlen;
use crate::mem::{ConstPtr, GuestUSize, MutPtr};
use crate::objc::{id, msg, msg_class};
use crate::Environment;

pub type CFStringRef = super::CFTypeRef;
pub type CFMutableStringRef = CFStringRef;

pub type CFStringEncoding = u32;
pub const kCFStringEncodingMacRoman: CFStringEncoding = 0;
pub const kCFStringEncodingASCII: CFStringEncoding = 0x600;
pub const kCFStringEncodingUTF8: CFStringEncoding = 0x8000100;
pub const kCFStringEncodingUnicode: CFStringEncoding = 0x100;
pub const kCFStringEncodingUTF16: CFStringEncoding = kCFStringEncodingUnicode;
pub const kCFStringEncodingUTF16BE: CFStringEncoding = 0x10000100;
pub const kCFStringEncodingUTF16LE: CFStringEncoding = 0x14000100;
pub const kCFStringEncodingISOLatin1: CFStringEncoding = 0x0201;

fn CFStringAppend(
    env: &mut Environment,
    the_string: CFMutableStringRef,
    appended_string: CFStringRef,
) {
    msg![env; the_string appendString:appended_string]
}

fn CFStringAppendCString(
    env: &mut Environment,
    string: CFMutableStringRef,
    c_string: ConstPtr<u8>,
    encoding: CFStringEncoding,
) {
    let encoding = CFStringConvertEncodingToNSStringEncoding(env, encoding);
    // TODO: avoid copying
    let to_append: id = msg_class![env; NSString stringWithCString:c_string encoding:encoding];
    msg![env; string appendString:to_append]
}

fn CFStringAppendFormat(
    env: &mut Environment,
    string: CFMutableStringRef,
    // Apple's own docs say these are unimplemented!
    _format_options: CFDictionaryRef,
    format: CFStringRef,
    dots: DotDotDot,
) {
    let res = ns_string::with_format(env, format, dots.start());
    let to_append: id = ns_string::from_rust_string(env, res);
    msg![env; string appendString:to_append]
}

pub fn CFStringConvertEncodingToNSStringEncoding(
    _env: &mut Environment,
    encoding: CFStringEncoding,
) -> ns_string::NSStringEncoding {
    match encoding {
        kCFStringEncodingMacRoman => ns_string::NSMacOSRomanStringEncoding,
        kCFStringEncodingASCII => ns_string::NSASCIIStringEncoding,
        kCFStringEncodingUTF8 => ns_string::NSUTF8StringEncoding,
        kCFStringEncodingUTF16 => ns_string::NSUTF16StringEncoding,
        kCFStringEncodingUTF16BE => ns_string::NSUTF16BigEndianStringEncoding,
        kCFStringEncodingUTF16LE => ns_string::NSUTF16LittleEndianStringEncoding,
        kCFStringEncodingISOLatin1 => ns_string::NSISOLatin1StringEncoding,
        _ => unimplemented!("Unhandled: CFStringEncoding {:#x}", encoding),
    }
}
fn CFStringConvertNSStringEncodingToEncoding(
    _env: &mut Environment,
    encoding: ns_string::NSStringEncoding,
) -> CFStringEncoding {
    match encoding {
        ns_string::NSMacOSRomanStringEncoding => kCFStringEncodingMacRoman,
        ns_string::NSASCIIStringEncoding => kCFStringEncodingASCII,
        ns_string::NSUTF8StringEncoding => kCFStringEncodingUTF8,
        ns_string::NSUTF16StringEncoding => kCFStringEncodingUTF16,
        ns_string::NSUTF16BigEndianStringEncoding => kCFStringEncodingUTF16BE,
        ns_string::NSUTF16LittleEndianStringEncoding => kCFStringEncodingUTF16LE,
        ns_string::NSISOLatin1StringEncoding => kCFStringEncodingISOLatin1,
        _ => unimplemented!("Unhandled: NSStringEncoding {:#x}", encoding),
    }
}

fn CFStringCreateCopy(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    the_string: CFStringRef,
) -> CFStringRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default()); // unimplemented
    msg![env; the_string copy]
}

fn CFStringCreateMutable(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    max_length: CFIndex,
) -> CFMutableStringRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default()); // unimplemented
    assert_eq!(max_length, 0);
    msg_class![env; NSMutableString new]
}

fn CFStringCreateMutableCopy(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    max_length: CFIndex,
    the_string: CFStringRef,
) -> CFMutableStringRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default()); // unimplemented
    assert_eq!(max_length, 0);
    msg![env; the_string mutableCopy]
}

fn CFStringCreateWithBytes(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    bytes: ConstPtr<u8>,
    num_bytes: CFIndex,
    encoding: CFStringEncoding,
    is_external: bool,
) -> CFStringRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default()); // unimplemented
    assert!(!is_external); // TODO
    let encoding = CFStringConvertEncodingToNSStringEncoding(env, encoding);
    let length: NSUInteger = num_bytes.try_into().unwrap();
    let ns_string: id = msg_class![env; NSString alloc];
    msg![env; ns_string initWithBytes:bytes length:length encoding:encoding]
}

fn CFStringCreateWithCString(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    c_string: ConstPtr<u8>,
    encoding: CFStringEncoding,
) -> CFStringRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default()); // unimplemented
    let encoding = CFStringConvertEncodingToNSStringEncoding(env, encoding);
    let ns_string: id = msg_class![env; NSString alloc];
    msg![env; ns_string initWithCString:c_string encoding:encoding]
}

fn CFStringCreateWithCStringNoCopy(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    c_string: ConstPtr<u8>,
    encoding: CFStringEncoding,
    deallocator: CFAllocatorRef,
) -> CFStringRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default()); // unimplemented
    assert!(env.mem.read(deallocator).is_null()); // unimplemented
    let encoding = CFStringConvertEncodingToNSStringEncoding(env, encoding);
    let c_len: GuestUSize = strlen(env, c_string);
    let ns_string: id = msg_class![env; NSString alloc];
    // Docs of CFStringCreateWithCStringNoCopy says caller should never assume
    // that the object is using the external buffer (it could be copied or even
    // dumped). So we can "safely" invoke a method which does copy!
    msg![env; ns_string initWithBytes:c_string length:c_len encoding:encoding]
}

fn CFStringCreateWithFormat(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    format_options: CFDictionaryRef,
    format: CFStringRef,
    args: DotDotDot,
) -> CFStringRef {
    CFStringCreateWithFormatAndArguments(env, allocator, format_options, format, args.start())
}

fn CFStringCreateWithFormatAndArguments(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    // Apple's own docs say these are unimplemented!
    _format_options: CFDictionaryRef,
    format: CFStringRef,
    args: VaList,
) -> CFStringRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default()); // unimplemented
    let res = ns_string::with_format(env, format, args);
    ns_string::from_rust_string(env, res)
}

fn CFStringCreateWithSubstring(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    the_string: CFStringRef,
    range: CFRange,
) -> CFStringRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default()); // unimplemented
    let range = NSRange {
        location: range.location.try_into().unwrap(),
        length: range.length.try_into().unwrap(),
    };
    let res: id = msg![env; the_string substringWithRange:range];
    msg![env; res copy]
}

pub type CFStringCompareFlags = CFOptionFlags;

fn CFStringCompare(
    env: &mut Environment,
    a: CFStringRef,
    b: CFStringRef,
    flags: CFStringCompareFlags,
) -> CFComparisonResult {
    msg![env; a compare:b options:flags]
}

fn CFStringCompareWithOptions(
    env: &mut Environment,
    a: CFStringRef,
    b: CFStringRef,
    range: CFRange,
    flags: CFStringCompareFlags,
) -> CFComparisonResult {
    let range = NSRange {
        location: range.location.try_into().unwrap(),
        length: range.length.try_into().unwrap(),
    };
    // TODO: avoid copying
    let a_sub: id = msg![env; a substringWithRange:range];
    msg![env; a_sub compare:b options:flags]
}

fn CFStringDelete(env: &mut Environment, string: CFMutableStringRef, range: CFRange) {
    let range = NSRange {
        location: range.location.try_into().unwrap(),
        length: range.length.try_into().unwrap(),
    };
    msg![env; string deleteCharactersInRange:range]
}

fn CFStringGetCharacterAtIndex(
    env: &mut Environment,
    the_string: CFStringRef,
    idx: CFIndex,
) -> unichar {
    // Both Sims 3 Ambitions and World Adventures use the same native text
    // renderer, but its return address differs between their iOS binaries.
    let wrapped_line_caller = match env.bundle.bundle_identifier() {
        "com.ea.sims3ambition.inc" => Some(0xc3cc4),
        _ => None,
    };
    if let Some(wrapped_line_caller) = wrapped_line_caller {
        let string_length = CFStringGetLength(env, the_string);
        if idx < 0 || idx >= string_length {
            let sp = env.cpu.regs()[crate::cpu::Cpu::SP];
            let caller = env.mem.read(ConstPtr::<u32>::from_bits(sp.wrapping_add(4)));
            if caller == wrapped_line_caller {
                // TextManager::drawWrappedStringLines sometimes requests a
                // character beyond the end of its source string while handling
                // wrapped line boundaries. It computed the segment length in
                // r4 before calling charAt(). Clear that length so it skips
                // the invalid segment instead of drawing out-of-range text.
                // Scope this to verified call sites from these two games only.
                env.cpu.regs_mut()[4] = 0;

                return 0;
            }
        }
    }

    let idx: NSUInteger = idx.try_into().unwrap();
    msg![env; the_string characterAtIndex:idx]
}

fn CFStringGetCharacters(
    env: &mut Environment,
    string: CFStringRef,
    range: CFRange,
    buffer: MutPtr<unichar>,
) {
    let range = NSRange {
        location: range.location.try_into().unwrap(),
        length: range.length.try_into().unwrap(),
    };
    msg![env; string getCharacters:buffer range:range]
}

fn CFStringGetCharactersPtr(_env: &mut Environment, _the_string: CFStringRef) -> ConstPtr<unichar> {
    // NULL is expected if the function cannot provide a buffer of Unicode
    // characters `efficiently`. Moreover, the same doc claims that the caller
    // should not `count on receiving a non-NULL result from this function
    // under any circumstances`. Win-win situation, if you ask me!
    ConstPtr::null()
}

fn CFStringGetCStringPtr(
    env: &mut Environment,
    the_string: CFStringRef,
    encoding: CFStringEncoding,
) -> ConstPtr<u8> {
    let encoding = CFStringConvertEncodingToNSStringEncoding(env, encoding);
    msg![env; the_string cStringUsingEncoding:encoding]
}

fn CFStringGetCString(
    env: &mut Environment,
    a: CFStringRef,
    buffer: MutPtr<u8>,
    buffer_size: CFIndex,
    encoding: CFStringEncoding,
) -> bool {
    let encoding = CFStringConvertEncodingToNSStringEncoding(env, encoding);
    let buffer_size = buffer_size as NSUInteger;
    msg![env; a getCString:buffer maxLength:buffer_size encoding:encoding]
}

fn CFStringGetBytes(
    env: &mut Environment,
    string: CFStringRef,
    range: CFRange,
    encoding: CFStringEncoding,
    loss_byte: u8,
    is_external: bool,
    buffer: MutPtr<u8>,
    max_buf_len: CFIndex,
    used_buf_len: MutPtr<CFIndex>,
) -> CFIndex {
    assert_eq!(loss_byte, 0);
    assert!(!is_external); // TODO

    let range_len = range.length;
    let range = NSRange {
        location: range.location.try_into().unwrap(),
        length: range_len.try_into().unwrap(),
    };
    // TODO: avoid copying
    let substring: id = msg![env; string substringWithRange:range];

    let encoding = CFStringConvertEncodingToNSStringEncoding(env, encoding);
    let buffer_size: NSUInteger = max_buf_len.try_into().unwrap();
    let success: bool =
        ns_string::get_bytes_buffer_inner(env, substring, buffer, buffer_size, encoding, false);
    assert!(success); // TODO
    let length: NSUInteger = msg![env; substring length];
    assert_eq!(length, range_len.try_into().unwrap());

    if !used_buf_len.is_null() {
        let result_bytes_length: NSUInteger =
            msg![env; substring lengthOfBytesUsingEncoding:encoding];
        env.mem
            .write(used_buf_len, result_bytes_length.try_into().unwrap());
    }

    length.try_into().unwrap()
}

fn CFStringGetFileSystemRepresentation(
    env: &mut Environment,
    string: CFStringRef,
    buffer: MutPtr<u8>,
    max_buflen: CFIndex,
) -> bool {
    let max_buflen: NSUInteger = max_buflen.try_into().unwrap();
    msg![env; string getFileSystemRepresentation:buffer maxLength:max_buflen]
}

fn CFStringGetLength(env: &mut Environment, the_string: CFStringRef) -> CFIndex {
    let length: NSUInteger = msg![env; the_string length];
    length.try_into().unwrap()
}

fn CFStringGetIntValue(env: &mut Environment, string: CFStringRef) -> i32 {
    // TODO: check for allowed characters
    msg![env; string intValue]
}

fn CFStringFind(
    env: &mut Environment,
    string: CFStringRef,
    to_find: CFStringRef,
    options: CFStringCompareFlags,
) -> CFRange {
    let range: NSRange = msg![env; string rangeOfString:to_find options:options];
    let location: CFIndex = if range.location == NSNotFound as NSUInteger {
        // NSNotFound and kCFNotFound are not the same!
        kCFNotFound
    } else {
        range.location.try_into().unwrap()
    };
    CFRange {
        location,
        length: range.length.try_into().unwrap(),
    }
}

fn CFStringHasSuffix(env: &mut Environment, the_string: CFStringRef, suffix: CFStringRef) -> bool {
    msg![env; the_string hasSuffix:suffix]
}

fn CFStringUppercase(env: &mut Environment, string: CFStringRef, _locale: CFLocaleRef) {
    // TODO: account for locale
    let uppercase: id = msg![env; string uppercaseString];
    msg![env; string setString:uppercase]
}

type ConstStr255Param = ConstPtr<u8>;
type StringPtr = MutPtr<u8>;

fn CFStringCreateWithPascalString(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    p_str: ConstStr255Param,
    encoding: CFStringEncoding,
) -> CFStringRef {
    let len: CFIndex = env.mem.read(p_str).into();
    let res = CFStringCreateWithBytes(env, allocator, p_str + 1, len, encoding, false);
    assert_eq!(len, CFStringGetLength(env, res));
    log_dbg!(
        "CFStringCreateWithPascalString('{}')",
        ns_string::to_rust_string(env, res)
    );
    res
}

fn CFStringGetPascalString(
    env: &mut Environment,
    the_string: CFStringRef,
    buffer: StringPtr,
    buffer_size: CFIndex,
    encoding: CFStringEncoding,
) -> bool {
    log_dbg!(
        "CFStringGetPascalString('{}')",
        ns_string::to_rust_string(env, the_string)
    );
    let len = CFStringGetLength(env, the_string);
    // first byte of Pascal string is length
    assert!((len + 1) <= buffer_size);
    let len_char: u8 = len.try_into().unwrap();
    env.mem.write(buffer, len_char);
    let encoding = CFStringConvertEncodingToNSStringEncoding(env, encoding);
    ns_string::get_bytes_buffer_inner(
        env,
        the_string,
        buffer + 1,
        len_char.into(),
        encoding,
        false,
    )
}

type CFStringNormalizationForm = CFIndex;

fn CFStringNormalize(
    env: &mut Environment,
    the_string: CFMutableStringRef,
    the_form: CFStringNormalizationForm,
) {
    let str = ns_string::to_rust_string(env, the_string);
    log!("TODO: CFStringNormalize('{}', {})", str, the_form);
    assert!(str.is_ascii()); // TODO
}

fn find_utf16_range(
    haystack: &[u16],
    needle: &[u16],
    options: CFStringCompareFlags,
) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }

    let backwards = options & 4 != 0;
    let anchored = options & 8 != 0;
    let case_insensitive = options & 1 != 0;
    let last = haystack.len() - needle.len();

    for offset in 0..=last {
        let position = if backwards { last - offset } else { offset };
        let found = haystack[position..position + needle.len()]
            .iter()
            .zip(needle)
            .all(|(&a, &b)| {
                if a == b {
                    return true;
                }
                if !case_insensitive {
                    return false;
                }
                let (Some(a), Some(b)) = (char::from_u32(a.into()), char::from_u32(b.into()))
                else {
                    return false;
                };
                a.to_lowercase().eq(b.to_lowercase())
            });
        if found {
            return Some(position);
        }
        if anchored {
            break;
        }
    }
    None
}

fn CFStringFindWithOptions(
    env: &mut Environment,
    string: CFStringRef,
    to_find: CFStringRef,
    search_range: CFRange,
    options: CFStringCompareFlags,
    result: MutPtr<CFRange>,
) -> bool {
    assert_eq!(
        options & !(1 | 4 | 8),
        0,
        "Unsupported CFStringFindWithOptions flags: {options:#x}"
    );

    let start: usize = search_range.location.try_into().unwrap();
    let length: usize = search_range.length.try_into().unwrap();
    let string_length: usize = CFStringGetLength(env, string).try_into().unwrap();
    assert!(start <= string_length && length <= string_length - start);

    let needle_length: usize = CFStringGetLength(env, to_find).try_into().unwrap();
    if needle_length == 0 || needle_length > length {
        return false;
    }

    let haystack: Vec<u16> = (start..start + length)
        .map(|i| CFStringGetCharacterAtIndex(env, string, i.try_into().unwrap()))
        .collect();
    let needle: Vec<u16> = (0..needle_length)
        .map(|i| CFStringGetCharacterAtIndex(env, to_find, i.try_into().unwrap()))
        .collect();

    let Some(position) = find_utf16_range(&haystack, &needle, options) else {
        return false;
    };

    if !result.is_null() {
        env.mem.write(
            result,
            CFRange {
                location: (start + position).try_into().unwrap(),
                length: needle_length.try_into().unwrap(),
            },
        );
    }
    true
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFStringFindWithOptions(_, _, _, _, _)),
    export_c_func!(CFStringAppend(_, _)),
    export_c_func!(CFStringAppendCString(_, _, _)),
    export_c_func!(CFStringAppendFormat(_, _, _, _)),
    export_c_func!(CFStringConvertEncodingToNSStringEncoding(_)),
    export_c_func!(CFStringConvertNSStringEncodingToEncoding(_)),
    export_c_func!(CFStringCreateCopy(_, _)),
    export_c_func!(CFStringCreateMutable(_, _)),
    export_c_func!(CFStringCreateMutableCopy(_, _, _)),
    export_c_func!(CFStringCreateWithBytes(_, _, _, _, _)),
    export_c_func!(CFStringCreateWithCString(_, _, _)),
    export_c_func!(CFStringCreateWithCStringNoCopy(_, _, _, _)),
    export_c_func!(CFStringCreateWithFormat(_, _, _, _)),
    export_c_func!(CFStringCreateWithFormatAndArguments(_, _, _, _)),
    export_c_func!(CFStringCreateWithSubstring(_, _, _)),
    export_c_func!(CFStringCompare(_, _, _)),
    export_c_func!(CFStringCompareWithOptions(_, _, _, _)),
    export_c_func!(CFStringDelete(_, _)),
    export_c_func!(CFStringGetCharacterAtIndex(_, _)),
    export_c_func!(CFStringGetCharacters(_, _, _)),
    export_c_func!(CFStringGetCharactersPtr(_)),
    export_c_func!(CFStringGetCStringPtr(_, _)),
    export_c_func!(CFStringGetCString(_, _, _, _)),
    export_c_func!(CFStringGetBytes(_, _, _, _, _, _, _, _)),
    export_c_func!(CFStringGetFileSystemRepresentation(_, _, _)),
    export_c_func!(CFStringGetIntValue(_)),
    export_c_func!(CFStringGetLength(_)),
    export_c_func!(CFStringFind(_, _, _)),
    export_c_func!(CFStringHasSuffix(_, _)),
    export_c_func!(CFStringUppercase(_, _)),
    export_c_func!(CFStringCreateWithPascalString(_, _, _)),
    export_c_func!(CFStringGetPascalString(_, _, _, _)),
    export_c_func!(CFStringNormalize(_, _)),
];

#[cfg(test)]
mod find_with_options_tests {
    use super::find_utf16_range;
    #[test]
    fn searches_forward_and_backward() {
        let source: Vec<u16> = "hello hello".encode_utf16().collect();
        let needle: Vec<u16> = "hello".encode_utf16().collect();
        assert_eq!(find_utf16_range(&source, &needle, 0), Some(0));
        assert_eq!(find_utf16_range(&source, &needle, 4), Some(6));
        assert_eq!(find_utf16_range(&source, &needle, 8), Some(0));
        assert_eq!(find_utf16_range(&source, &needle, 12), Some(6));
    }
    #[test]
    fn respects_case_and_missing_matches() {
        let source: Vec<u16> = "Game Data".encode_utf16().collect();
        let needle: Vec<u16> = "game".encode_utf16().collect();
        assert_eq!(find_utf16_range(&source, &needle, 0), None);
        assert_eq!(find_utf16_range(&source, &needle, 1), Some(0));
        assert_eq!(find_utf16_range(&source, &[], 0), None);
    }
    #[test]
    fn searches_utf16_code_units() {
        let source: Vec<u16> = "a😀b😀".encode_utf16().collect();
        let needle: Vec<u16> = "😀".encode_utf16().collect();
        assert_eq!(find_utf16_range(&source, &needle, 0), Some(1));
        assert_eq!(find_utf16_range(&source, &needle, 4), Some(4));
    }
}
