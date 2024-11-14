/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSURL`.

// TODO: The url crate is not quite perfect for what we want - it doesn't
// support relative URLs, and it's also based on newer url standards.
// While the latter is probably fine, the former has to be worked around by
// basing relative URLs on another url.
// Note that hyper::http::uri is a worse choice here - even though it supports
// relative URLs, it doesn't support joining relative URLs to absolute URLs,
// which is needed for [NSURL path], among other apis.
use url::Url;

use super::ns_string::{from_rust_string, to_rust_string};
use super::NSUInteger;
use crate::frameworks::foundation::unichar;
use crate::fs::GuestPath;
use crate::mem::MutPtr;
use crate::objc::{
    autorelease, id, msg, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};
use crate::Environment;
use std::borrow::Cow;

// A base url for urls that are relative but should have no base.
// This is to work around limitations in the url crate, see above
static relative_base: std::sync::LazyLock<Url> =
    std::sync::LazyLock::new(|| Url::parse("file://localhost").unwrap());

#[derive(Clone)]
enum NSURLHostObject {
    /// An absolute URL.
    AbsoluteURL {
        url_string: id,
        url: Url,
    },
    /// A relative URL without a defined base.
    RelativeURL {
        /// Note that this is already an absolute URL based on relative_base!
        url_string: id,
        url: Url,
    },
    /// A relative URL with a defined base.
    RelativeURLWithBase {
        url_string: id,
        /// Note that this is already an absolute URL based on base_url!
        url: Url,
        base_string: id,
        base_url: Url,
    },
    Uninit,
}
impl HostObject for NSURLHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSURL: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = NSURLHostObject::Uninit;
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
        NSURLHostObject::AbsoluteURL { url_string, .. }
        | NSURLHostObject::RelativeURL { url_string, .. } => {
            release(env, url_string);
        }
        NSURLHostObject::RelativeURLWithBase {
            url_string,
            base_string,
            ..
        } => {
            release(env, url_string);
            release(env, base_string);
        }
        NSURLHostObject::Uninit => {}
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
              isDirectory:(bool)is_dir {
    // FIXME: this does not resolve relative paths to be absolute!
    // TODO: this does not strip the file:/// prefix!
    let mut path_str = to_rust_string(env, path).to_string();
    // HACK: This should block against url
    assert!(!path_str.contains(":") && !path_str.starts_with("//"));
    if is_dir && !path_str.ends_with("/") {
        path_str += "/";
    } else if !is_dir && path_str.ends_with("/") {
        path_str.truncate(path_str.len() - 1)
    }
    let path: id = msg![env; path copy];
    let (base_url, base_string) = working_directory_url_helper(env);
    let url = match base_url.join(path_str.as_ref()) {
        Ok(url) => url,
        Err(err) => {
            log!("initFileURLWithPath:({}) isDirectory:({}) failed to make url (error: {}), returning nil!", path_str, is_dir, err);
            return nil;
        }
    };
    *env.objc.borrow_mut(this) = NSURLHostObject::RelativeURLWithBase { url_string: path, url, base_string, base_url };

    this
}

- (id)initWithString:(id)url { // NSString*
    if url == nil {
        return nil;
    }

    let url_string: id = msg![env; url copy];
    let url_str = to_rust_string(env, url_string);
    match Url::parse(&url_str) {
        Ok(url) => {
            *env.objc.borrow_mut(this) = NSURLHostObject::AbsoluteURL { url_string, url };
        },
        Err(first_err) => {
            // Check if this is an unbased relative url.
            match relative_base.join(&url_str) {
                Ok(url) => {
                    *env.objc.borrow_mut(this) = NSURLHostObject::RelativeURL { url_string, url };
                },
                Err(second_err) => {
                    log!("[NSURL initWithString:{}] failed, errors ({}), ({})", url_str, first_err, second_err);
                    return nil;
                },
            }
        },
    }
    this
}

- (id)description {
    let host_obj = env.objc.borrow::<NSURLHostObject>(this).clone();
    match host_obj {
        NSURLHostObject::AbsoluteURL { url_string, .. }
        | NSURLHostObject::RelativeURL { url_string, .. } => url_string,
        NSURLHostObject::RelativeURLWithBase {
            url_string,
            base_string,
            ..
        } => {
            let url_str = to_rust_string(env, url_string);
            let base_str = to_rust_string(env, base_string);
            from_rust_string(env, format!("{} -- {}", url_str, base_str))
        }
        NSURLHostObject::Uninit => panic!("Use of uninitialized NSURL {:?}!", this),
    }
}

- (id)path {
    let host_obj = env.objc.borrow::<NSURLHostObject>(this).clone();
    match host_obj {
        NSURLHostObject::AbsoluteURL { url, .. } |
        NSURLHostObject::RelativeURLWithBase { url, .. } => {
            from_rust_string(env, url.path().to_string())
        },
        NSURLHostObject::RelativeURL { url_string, url }=> {
            // url::URL always adds a leading '/' to the path - we only want to
            // keep it if the original string starts with a '/' (0x002F).
            let leading_char: unichar = msg![env; url_string characterAtIndex:0];
            if leading_char == 0x002F as unichar {
                from_rust_string(env, url.path().to_string())
            } else {
                from_rust_string(env, url.path()[1..].to_string())
            }
        },
        NSURLHostObject::Uninit => panic!("Use of uninitialized NSURL {:?}!", this),
    }
}

- (id)absoluteString {
    todo!()
}

- (id)absoluteURL {
    todo!();
}

- (bool)getFileSystemRepresentation:(MutPtr<u8>)buffer
                          maxLength:(NSUInteger)buffer_size {
    // FIXME: Should canonicalize file names
    let host_obj = env.objc.borrow::<NSURLHostObject>(this).clone();
    match host_obj {
        NSURLHostObject::AbsoluteURL { url, .. } => {
            if url.scheme() != "file" {
                log!("[NSURL({}) getFileSystemRepresentation:getFileSystemRepresentation:] called for non-file URL, returning nil.", url.as_str());
                false
            } else {
                let path = url.path().as_bytes();
                if path.len() > buffer_size as usize {
                    false
                } else {
                    // Does not write null terminator (tested on simulator 5.1)
                    let buf = env.mem.bytes_at_mut(buffer, path.len().try_into().unwrap());
                    buf.copy_from_slice(path);
                    true
                }
            }
        }
        NSURLHostObject::RelativeURLWithBase { .. } => {
            todo!()
        }
        NSURLHostObject::RelativeURL { .. } => {
            todo!()
        }
        NSURLHostObject::Uninit => panic!("Use of uninitialized NSURL!"),
    }
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

/// Gets a file:// url pointing to the working directory.
pub fn working_directory_url_helper(env: &mut Environment) -> (Url, id) {
    let working_directory = env.fs.working_directory().as_str();
    // WIP TODO: this should percent escape characters, borrow from the nsstring impl
    let url_string = format!("file://localhost/{}", working_directory);
    let url = Url::parse(&url_string).unwrap();
    let url_ns_string = from_rust_string(env, url_string);
    (url, url_ns_string)
}
