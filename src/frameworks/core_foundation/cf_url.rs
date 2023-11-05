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
pub type CFURLPathStyle = CFIndex;

pub const kCFURLPOSIXPathStyle: CFURLPathStyle = 0;
pub const kCFURLHFSPathStyle: CFURLPathStyle = 1;
pub const kCFURLWindowsPathStyle: CFURLPathStyle = 2;

pub fn CFURLGetFileSystemRepresentation(
    env: &mut Environment,
    url: CFURLRef,
    resolve_against_base: bool,
    buffer: MutPtr<u8>,
    buffer_size: CFIndex,
) -> bool {
    if resolve_against_base {
        // this function usually called to resolve resources from the main bundle
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

pub fn CFURLCopyFileSystemPath(
    env: &mut Environment,
    _anURL: CFURLRef,
    _pathStyle : CFURLPathStyle
) -> CFURLRef {

    /*
        The URL's path in the format specified by pathStyle. Ownership follows the create rule. See The Create Rule.

        ------------------------------------------------------------------------------------------------------------------------------------------------------

        The Create Rule

        Core Foundation functions have names that indicate when you own a returned object:

        Object-creation functions that have “Create” embedded in the name;
        Object-duplication functions that have “Copy” embedded in the name.
        If you own an object, it is your responsibility to relinquish ownership (using CFRelease) when you have finished with it.

        Consider the following examples. The first example shows two create functions associated with CFTimeZone and one associated with CFBundle.
    */

    let url: id = msg_class![env; NSURL alloc];

    msg![env; url init]
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFURLGetFileSystemRepresentation(_, _, _, _)),
    export_c_func!(CFURLCreateFromFileSystemRepresentation(_, _, _, _)),
    export_c_func!(CFURLCopyFileSystemPath(_, _))
];
