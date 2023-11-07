/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFURL`.
//!
//! This is toll-free bridged to `NSURL` in Apple's implementation. Here it is
//! the same type.

use super::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use super::CFIndex;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::ns_string::{to_rust_string, NSUTF8StringEncoding};
use crate::frameworks::foundation::NSUInteger;
use crate::mem::{ConstPtr, MutPtr};
use crate::objc::{id, msg, msg_class};
use crate::Environment;

pub type CFURLRef = super::CFTypeRef;

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
    msg![env; url initFileURLWithPath:string isDirectory:is_directory]
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFURLGetFileSystemRepresentation(_, _, _, _)),
    export_c_func!(CFURLCreateFromFileSystemRepresentation(_, _, _, _)),
];
