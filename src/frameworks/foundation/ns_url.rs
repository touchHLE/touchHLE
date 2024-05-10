/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSURL`.

use super::ns_string::{from_rust_string, to_rust_string, NSUTF8StringEncoding};
use super::NSUInteger;
use crate::fs::{GuestPath, GuestPathBuf};
use crate::mem::MutPtr;
use crate::objc::{
    autorelease, id, msg, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};
use crate::Environment;
use std::borrow::Cow;

/// It seems like there's two kinds of NSURLs: ones for file paths, and others.
/// So far only the former is implemented (TODO).
enum NSURLHostObject {
    /// This is a file URL. The NSString is a system path (no `file:///`).
    ///
    /// This is a wrapper around NSString so that conversions between NSURL
    /// and NSString, which happen often, can be simple and efficient.
    FileURL {
        ns_string: id,
        // Relative file URL save the working directory at the time of creation
        // At the moment, used in the description selector.
        working_directory: GuestPathBuf,
    },
    /// Non-file URL.
    OtherURL { ns_string: id },
}
impl HostObject for NSURLHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSURL: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = NSURLHostObject::FileURL { ns_string: nil, working_directory: env.fs.working_directory().into() };
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

+ (id)URLWithString:(id)url { // NSString*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithString:url];
    autorelease(env, new)
}

+ (id)fileURLWithPath:(id)path { // NSString*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initFileURLWithPath:path];
    autorelease(env, new)
}

+ (id)fileURLWithPath:(id)path // NSString*
          isDirectory:(bool)is_dir {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initFileURLWithPath:path isDirectory:is_dir];
    autorelease(env, new)
}

- (())dealloc {
    match *env.objc.borrow(this) {
        NSURLHostObject::FileURL { ns_string, .. } => release(env, ns_string),
        NSURLHostObject::OtherURL { ns_string } => release(env, ns_string),
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

- (id)initFileURLWithPath:(id)path { // NSString*
    // FIXME: this should guess whether the path is a directory
    msg![env; this initFileURLWithPath:path isDirectory:false]
}

- (id)initFileURLWithPath:(id)path // NSString*
              isDirectory:(bool)_is_dir {
    // FIXME: this does not resolve relative paths to be absolute!
    // TODO: this does not strip the file:/// prefix!
    assert!(!to_rust_string(env, path).starts_with("file:"));
    let path: id = msg![env; path copy];
    *env.objc.borrow_mut(this) = NSURLHostObject::FileURL { ns_string: path, working_directory: env.fs.working_directory().into() };
    this
}

- (id)initWithString:(id)url { // NSString*
    if url == nil {
        return nil;
    }

    // FIXME: this should parse the URL
    assert!(!to_rust_string(env, url).starts_with("file:")); // TODO
    let url: id = msg![env; url copy];
    *env.objc.borrow_mut(this) = NSURLHostObject::OtherURL { ns_string: url };
    this
}

- (id)description {
    match env.objc.borrow(this) {
        NSURLHostObject::FileURL { ns_string, working_directory } => {
            let working_directory = working_directory.as_str().to_string();
            let mut description = to_rust_string(env, *ns_string).to_string().clone();
            if !description.starts_with('/') {
                description = format!("{} -- file://localhost{}", description.trim_start_matches("./"), working_directory );
            }
            let desc = from_rust_string(env, description);
            autorelease(env, desc)
        },
        NSURLHostObject::OtherURL { ns_string } => *ns_string,
    }
}

- (id)path {
    match *env.objc.borrow(this) {
        NSURLHostObject::FileURL { ns_string, .. } => ns_string,
        NSURLHostObject::OtherURL { ns_string } => {
            // TODO: Support full URLs, not only ones that are just a path.
            // FIXME: This should do unescaping.
            // TODO: Avoid copy.
            assert!(to_rust_string(env, ns_string).starts_with('/'));
            ns_string
        },
    }
}

- (id)absoluteString {
    match *env.objc.borrow(this) {
        // FIXME: don't assume URL is already absolute
        NSURLHostObject::FileURL { ns_string, .. } => ns_string,
        NSURLHostObject::OtherURL { ns_string } => {
            // TODO: full RFC 1808 resolution
            assert!(to_rust_string(env, ns_string).starts_with("http"));
            ns_string
        },
    }
}

- (id)absoluteURL {
    // FIXME: don't assume URL is already absolute
    let &NSURLHostObject::OtherURL { .. } = env.objc.borrow(this) else {
        unimplemented!(); // TODO
    };
    this
}

- (bool)getFileSystemRepresentation:(MutPtr<u8>)buffer
                          maxLength:(NSUInteger)buffer_size {
    let &NSURLHostObject::FileURL { ns_string, .. } = env.objc.borrow(this) else {
        unimplemented!(); // TODO
    };
    msg![env; ns_string getCString:buffer
                         maxLength:buffer_size
                          encoding:NSUTF8StringEncoding]
}

// TODO: more constructors, more accessors

@end

};

/// Shortcut for host code, provides a view of a URL as a path.
/// TODO: Try to avoid allocating a new GuestPathBuf in more cases.
pub fn to_rust_path(env: &mut Environment, url: id) -> Cow<'static, GuestPath> {
    let path_string: id = msg![env; url path];

    match to_rust_string(env, path_string) {
        Cow::Borrowed(path) => Cow::Borrowed(path.as_ref()),
        Cow::Owned(path_buf) => Cow::Owned(path_buf.into()),
    }
}
