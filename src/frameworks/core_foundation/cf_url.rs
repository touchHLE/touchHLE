/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFURL`.
//!
//! This is toll-free bridged to `NSURL` in Apple's implementation. Here it is
//! the same type.

use std::collections::HashSet;

use super::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use super::CFIndex;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_foundation::cf_string::{
    kCFStringEncodingASCII, kCFStringEncodingUTF8, CFStringConvertEncodingToNSStringEncoding,
    CFStringEncoding, CFStringRef,
};
use crate::frameworks::core_foundation::CFRetain;
use crate::frameworks::foundation::ns_string::{
    from_rust_string, get_static_str, to_rust_string, NSUTF8StringEncoding,
};
use crate::frameworks::foundation::NSUInteger;
use crate::mem::{ConstPtr, MutPtr, Ptr};
use crate::objc::{id, msg, msg_class, release};
use crate::Environment;

pub type CFURLRef = super::CFTypeRef;

type CFURLPathStyle = CFIndex;
const kCFURLPOSIXPathStyle: CFURLPathStyle = 0;
#[allow(dead_code)]
const kCFURLHFSPathStyle: CFURLPathStyle = 1;
#[allow(dead_code)]
const kCFURLWindowsPathStyle: CFURLPathStyle = 2;

const LEGAL_URI_CHARACTERS: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
const RESERVED_URI_CHARACTERS: &str = ":/?#[]@!$&'()*+,;=";

pub fn CFURLGetFileSystemRepresentation(
    env: &mut Environment,
    url: CFURLRef,
    resolve_against_base: bool,
    buffer: MutPtr<u8>,
    buffer_size: CFIndex,
) -> bool {
    if resolve_against_base {
        // this function usually called to resolve resources from the main
        // bundle
        // thus, the url should already be an absolute path name
        // TODO: use absoluteURL instead once implemented
        let path = msg![env; url path];
        // TODO: avoid copy
        assert!(to_rust_string(env, path).starts_with('/'));
    }
    let buffer_size: NSUInteger = buffer_size.try_into().unwrap();

    msg![env; url getFileSystemRepresentation:buffer
                                    maxLength:buffer_size]
}

pub fn CFURLCreateFromFileSystemRepresentation(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    buffer: ConstPtr<u8>,
    buffer_size: CFIndex,
    is_directory: bool,
) -> CFURLRef {
    assert!(allocator == kCFAllocatorDefault); // unimplemented

    let buffer_size: NSUInteger = buffer_size.try_into().unwrap();

    let string: id = msg_class![env; NSString alloc];
    let string: id = msg![env; string initWithBytes:buffer
                                             length:buffer_size
                                           encoding:NSUTF8StringEncoding];

    let url: id = msg_class![env; NSURL alloc];
    let res = msg![env; url initFileURLWithPath:string isDirectory:is_directory];
    release(env, string);
    res
}

fn CFURLCreateWithBytes(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    url_bytes: ConstPtr<u8>,
    length: CFIndex,
    encoding: CFStringEncoding,
    base_url: CFURLRef,
) -> CFURLRef {
    assert_eq!(allocator, kCFAllocatorDefault); // unimplemented
    assert_eq!(encoding, kCFStringEncodingASCII); // TODO
    assert!(base_url.is_null()); // TODO

    // TODO: interpret percent escape sequences using encoding as well
    let encoding = CFStringConvertEncodingToNSStringEncoding(env, encoding);
    let length: NSUInteger = length.try_into().unwrap();

    if length == 0 {
        return Ptr::null();
    }

    let string: id = msg_class![env; NSString alloc];
    let string: id = msg![env; string initWithBytes:url_bytes
                                             length:length
                                           encoding:encoding];

    assert!(!to_rust_string(env, string).contains("://")); // TODO

    // Assume file URL case here
    let url: id = msg_class![env; NSURL alloc];
    let res = msg![env; url initFileURLWithPath:string];
    release(env, string);
    res
}

fn CFURLCreateWithFileSystemPath(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    file_path: CFStringRef,
    style: CFURLPathStyle,
    is_directory: bool,
) -> CFURLRef {
    assert!(allocator == kCFAllocatorDefault); // unimplemented
    assert_eq!(style, kCFURLPOSIXPathStyle);
    let url: id = msg_class![env; NSURL alloc];
    msg![env; url initFileURLWithPath:file_path isDirectory:is_directory]
}

fn CFURLCreateStringByAddingPercentEscapes(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    original_string_ref: CFStringRef,
    characters_to_leave_unescaped: CFStringRef,
    legal_url_characters_to_be_escaped: CFStringRef,
    encoding: CFStringEncoding,
) -> CFStringRef {
    assert!(allocator.is_null() || allocator == kCFAllocatorDefault); // unimplemented
    assert_eq!(encoding, kCFStringEncodingUTF8); // TODO
    let original_string = to_rust_string(env, original_string_ref);
    assert!(!original_string.contains("%")); // TODO: Handle escaped chars?
    let characters_to_leave_unescaped = if characters_to_leave_unescaped.is_null() {
        ""
    } else {
        &to_rust_string(env, characters_to_leave_unescaped)
    };
    let legal_url_characters_to_be_escaped = if legal_url_characters_to_be_escaped.is_null() {
        ""
    } else {
        &to_rust_string(env, legal_url_characters_to_be_escaped)
    };
    let mut legal_characters = LEGAL_URI_CHARACTERS
        .chars()
        .chain(RESERVED_URI_CHARACTERS.chars())
        .chain(characters_to_leave_unescaped.chars())
        .collect::<HashSet<char>>();
    for value in legal_url_characters_to_be_escaped.chars() {
        legal_characters.remove(&value);
    }
    let escaped = original_string
        .chars()
        .map(|c| {
            if legal_characters.contains(&c) {
                c.to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect::<String>();
    if escaped == original_string {
        // From the docs:
        // If it does not need to be modified (no percent escape sequences are
        // missing), this function may merely return originalString with its
        // reference count incremented.
        CFRetain(env, original_string_ref)
    } else {
        // TODO: Create NSURL directly if possible, to avoid the extra copy
        let escaped: id = from_rust_string(env, escaped);
        msg_class![env; NSURL URLWithString:escaped]
    }
}

pub fn CFURLCopyPathExtension(env: &mut Environment, url: CFURLRef) -> CFStringRef {
    let path = msg![env; url path];
    let ext = msg![env; path pathExtension];
    msg![env; ext copy]
}

fn CFURLCopyFileSystemPath(
    env: &mut Environment,
    url: CFURLRef,
    style: CFURLPathStyle,
) -> CFStringRef {
    assert_eq!(style, kCFURLPOSIXPathStyle);
    let path: CFStringRef = msg![env; url path];
    msg![env; path copy]
}

fn CFURLCreateCopyAppendingPathComponent(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    url: CFURLRef,
    path_component: CFStringRef,
    is_directory: bool,
) -> CFURLRef {
    assert!(allocator.is_null());
    let new_url =
        msg![env; url URLByAppendingPathComponent:path_component isDirectory:is_directory];
    msg![env; new_url copy]
}

fn CFURLCreateCopyDeletingLastPathComponent(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    url: CFURLRef,
) -> CFURLRef {
    assert!(allocator.is_null());
    let new_url = msg![env; url URLByDeletingLastPathComponent];
    msg![env; new_url copy]
}

fn CFURLHasDirectoryPath(env: &mut Environment, url: CFURLRef) -> bool {
    assert!(!url.is_null());

    let path = msg![env; url path];
    if msg![env; path isEqual:(get_static_str(env, "//"))] {
        // Special case
        return false;
    }
    // Note: cannot use `lastPathComponent` here!
    let components: id = msg![env; path pathComponents];
    let count: NSUInteger = msg![env; components count];
    if count == 0 {
        return false;
    }
    let last: id = msg![env; components objectAtIndex:(count - 1)];
    msg![env; last isEqual:(get_static_str(env, "/"))]
        || msg![env; last isEqual:(get_static_str(env, "."))]
        || msg![env; last isEqual:(get_static_str(env, ".."))]
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFURLGetFileSystemRepresentation(_, _, _, _)),
    export_c_func!(CFURLCreateFromFileSystemRepresentation(_, _, _, _)),
    export_c_func!(CFURLCreateWithBytes(_, _, _, _, _)),
    export_c_func!(CFURLCreateWithFileSystemPath(_, _, _, _)),
    export_c_func!(CFURLCreateStringByAddingPercentEscapes(_, _, _, _, _)),
    export_c_func!(CFURLCopyPathExtension(_)),
    export_c_func!(CFURLCopyFileSystemPath(_, _)),
    export_c_func!(CFURLCreateCopyAppendingPathComponent(_, _, _, _)),
    export_c_func!(CFURLCreateCopyDeletingLastPathComponent(_, _)),
    export_c_func!(CFURLHasDirectoryPath(_)),
];
