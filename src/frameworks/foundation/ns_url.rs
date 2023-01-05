//! `NSURL`.

use super::ns_string::{to_rust_string, NSUTF8StringEncoding};
use super::NSUInteger;
use crate::mem::{MutPtr, MutVoidPtr};
use crate::objc::{id, msg, nil, objc_classes, release, retain, ClassExports, HostObject};

/// It seems like there's two kinds of NSURLs: ones for file paths, and others.
/// So far only the former is implemented (TODO).
enum NSURLHostObject {
    /// This is a file URL. The NSString is a system path (no `file:///`).
    ///
    /// This is a wrapper around NSString so that conversions between NSURL
    /// and NSString, which happen often, can be simple and efficient.
    FileURL { ns_string: id },
}
impl HostObject for NSURLHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSURL: NSObject

+ (id)allocWithZone:(MutVoidPtr)_zone {
    let host_object = NSURLHostObject::FileURL { ns_string: nil };
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

- (())dealloc {
    let &NSURLHostObject::FileURL { ns_string } = env.objc.borrow(this);
    release(env, ns_string);
    env.objc.dealloc_object(this, &mut env.mem)
}

// NSCopying implementation
- (id)copyWithZone:(MutVoidPtr)_zone {
    retain(env, this)
}

- (id)initFileURLWithPath:(id)path { // NSString*
    // FIXME: this does not resolve relative paths to be absolute!
    // TODO: this does not strip the file:/// prefix!
    assert!(!to_rust_string(env, path).starts_with("file:"));
    let path: id = msg![env; path copy];
    *env.objc.borrow_mut(this) = NSURLHostObject::FileURL { ns_string: path };
    this
}

- (id)path {
    let &NSURLHostObject::FileURL { ns_string } = env.objc.borrow(this);
    ns_string
}

- (bool)getFileSystemRepresentation:(MutPtr<u8>)buffer
                          maxLength:(NSUInteger)buffer_size {
    let &NSURLHostObject::FileURL { ns_string } = env.objc.borrow(this);
    msg![env; ns_string getCString:buffer
                         maxLength:buffer_size
                          encoding:NSUTF8StringEncoding]
}

// TODO: more constructors, more accessors

@end

};
