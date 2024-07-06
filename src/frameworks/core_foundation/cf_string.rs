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
use crate::abi::{DotDotDot, VaList};
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::{kCFNotFound, CFIndex, CFOptionFlags, CFRange};
use crate::frameworks::foundation::{ns_string, NSNotFound, NSRange, NSUInteger};
use crate::mem::{ConstPtr, MutPtr};
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

fn CFStringConvertEncodingToNSStringEncoding(
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
        _ => unimplemented!("Unhandled: NSStringEncoding {:#x}", encoding),
    }
}

fn CFStringCreateMutable(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    max_length: CFIndex,
) -> CFMutableStringRef {
    assert_eq!(allocator, kCFAllocatorDefault); // unimplemented
    assert_eq!(max_length, 0);
    let str: id = msg_class![env; NSMutableString alloc];
    msg![env; str init]
}

fn CFStringCreateWithCString(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    c_string: ConstPtr<u8>,
    encoding: CFStringEncoding,
) -> CFStringRef {
    assert!(allocator == kCFAllocatorDefault); // unimplemented
    let encoding = CFStringConvertEncodingToNSStringEncoding(env, encoding);
    let ns_string: id = msg_class![env; NSString alloc];
    msg![env; ns_string initWithCString:c_string encoding:encoding]
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
    assert!(allocator == kCFAllocatorDefault); // unimplemented
    let res = ns_string::with_format(env, format, args);
    ns_string::from_rust_string(env, res)
}

pub type CFComparisonResult = CFIndex;
pub type CFStringCompareFlags = CFOptionFlags;

fn CFStringCompare(
    env: &mut Environment,
    a: CFStringRef,
    b: CFStringRef,
    flags: CFStringCompareFlags,
) -> CFComparisonResult {
    msg![env; a compare:b options:flags]
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

fn CFStringGetLength(env: &mut Environment, the_string: CFStringRef) -> CFIndex {
    let length: NSUInteger = msg![env; the_string length];
    length.try_into().unwrap()
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

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFStringAppendFormat(_, _, _, _)),
    export_c_func!(CFStringConvertEncodingToNSStringEncoding(_)),
    export_c_func!(CFStringConvertNSStringEncodingToEncoding(_)),
    export_c_func!(CFStringCreateMutable(_, _)),
    export_c_func!(CFStringCreateWithCString(_, _, _)),
    export_c_func!(CFStringCreateWithFormat(_, _, _, _)),
    export_c_func!(CFStringCreateWithFormatAndArguments(_, _, _, _)),
    export_c_func!(CFStringCompare(_, _, _)),
    export_c_func!(CFStringGetCStringPtr(_, _)),
    export_c_func!(CFStringGetCString(_, _, _, _)),
    export_c_func!(CFStringGetLength(_)),
    export_c_func!(CFStringFind(_, _, _)),
];
