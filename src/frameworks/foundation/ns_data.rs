/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSData` and `NSMutableData`.

use super::ns_string::to_rust_string;
use super::{NSRange, NSUInteger};
use crate::fs::GuestPath;
use crate::mem::{ConstVoidPtr, MutPtr, MutVoidPtr, Ptr};
use crate::objc::{
    autorelease, id, msg, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};
use crate::{msg_class, Environment};

struct NSDataHostObject {
    bytes: MutVoidPtr,
    length: NSUInteger,
}
impl HostObject for NSDataHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// NSData doesn't seem to be an abstract class?
@implementation NSData: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSDataHostObject {
        bytes: Ptr::null(),
        length: 0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)dataWithBytesNoCopy:(MutVoidPtr)bytes
                   length:(NSUInteger)length {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithBytesNoCopy:bytes length:length];
    autorelease(env, new)
}

+ (id)dataWithBytes:(MutVoidPtr)bytes
             length:(NSUInteger)length {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithBytes:bytes length:length];
    autorelease(env, new)
}

+ (id)dataWithContentsOfFile:(id)path {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithContentsOfFile:path];
    autorelease(env, new)
}

// Calling the standard `init` is also allowed, in which case we just get data
// of size 0.

- (id)initWithBytesNoCopy:(MutVoidPtr)bytes
                   length:(NSUInteger)length {
    let host_object = env.objc.borrow_mut::<NSDataHostObject>(this);
    assert!(host_object.bytes.is_null() && host_object.length == 0);
    host_object.bytes = bytes;
    host_object.length = length;
    this
}

- (id)initWithBytes:(MutVoidPtr)bytes
              length:(NSUInteger)length {
    let host_object = env.objc.borrow_mut::<NSDataHostObject>(this);
    assert!(host_object.bytes.is_null() && host_object.length == 0);
    let alloc = env.mem.alloc(length);
    env.mem.memmove(alloc, bytes.cast_const(), length);
    host_object.bytes = alloc;
    host_object.length = length;
    this
}

- (id)initWithContentsOfURL:(id)url { // NSURL *
    let path: id = msg![env; url absoluteString];
    let path = to_rust_string(env, path);
    // TODO: file URL case
    assert!(path.starts_with("http"));
    log!("TODO: ignoring [(NSData*){:?} initWithContentsOfURL:{:?}]", this, path);
    // TODO: actually load data once we have proper network support
    nil
}

- (id)initWithContentsOfFile:(id)path {
    let path = to_rust_string(env, path);
    log_dbg!("[(NSData*){:?} initWithContentsOfFile:{:?}]", this, path);
    let Ok(bytes) = env.fs.read(GuestPath::new(&path)) else {
        release(env, this);
        return nil;
    };
    let size = bytes.len().try_into().unwrap();
    let alloc = env.mem.alloc(size);
    let slice = env.mem.bytes_at_mut(alloc.cast(), size);
    slice.copy_from_slice(&bytes);

    let host_object = env.objc.borrow_mut::<NSDataHostObject>(this);
    host_object.bytes = alloc;
    host_object.length = size;
    this
}

// FIXME: writes should be atomic
- (bool)writeToFile:(id)path // NSString*
         atomically:(bool)_use_aux_file {
    let file = to_rust_string(env, path);
    log_dbg!("[(NSData*){:?} writeToFile:{:?} atomically:_]", this, file);
    let host_object = env.objc.borrow::<NSDataHostObject>(this);
    // Mem::bytes_at() panics when the pointer is NULL, but NSData's pointer can
    // be NULL if the length is 0.
    let slice = if host_object.length == 0 {
        &[]
    } else {
        env.mem.bytes_at(host_object.bytes.cast(), host_object.length)
    };
    env.fs.write(GuestPath::new(&file), slice).is_ok()
}

- (())dealloc {
    let &NSDataHostObject { bytes, .. } = env.objc.borrow(this);
    if !bytes.is_null() {
        env.mem.free(bytes);
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

- (id)mutableCopyWithZone:(NSZonePtr)_zone {
    let bytes: ConstVoidPtr = msg![env; this bytes];
    let length: NSUInteger = msg![env; this length];
    let new = msg_class![env; NSMutableData alloc];
    msg![env; new initWithBytes:(bytes.cast_mut()) length:length]
}

- (ConstVoidPtr)bytes {
    env.objc.borrow::<NSDataHostObject>(this).bytes.cast_const()
}
- (NSUInteger)length {
    env.objc.borrow::<NSDataHostObject>(this).length
}

- (())getBytes:(MutPtr<u8>)buffer range:(NSRange)range {
    if range.length == 0 {
        return;
    }
    let &NSDataHostObject { bytes, length, .. } = env.objc.borrow(this);
    // TODO: throw NSRangeException if out-of-range instead of panic?
    assert!(range.location < length && range.location + range.length <= length);
    env.mem.memmove(
        buffer.cast(),
        bytes.cast_const() + range.location,
        range.length,
    );
}

- (())getBytes:(MutPtr<u8>)buffer {
    let &NSDataHostObject { bytes, length, .. } = env.objc.borrow(this);
    env.mem.memmove(
        buffer.cast(),
        bytes.cast_const(),
        length,
    );
}

@end

@implementation NSMutableData: NSData

- (id)copyWithZone:(NSZonePtr)_zone {
    let bytes: ConstVoidPtr = msg![env; this bytes];
    let length: NSUInteger = msg![env; this length];
    let new = msg_class![env; NSData alloc];
    msg![env; new initWithBytes:bytes length:length]
}

- (())increaseLengthBy:(NSUInteger)add_len {
    let &NSDataHostObject { bytes, length, .. } = env.objc.borrow(this);
    let new_len = length + add_len;
    let new_bytes = env.mem.realloc(bytes, new_len);
    let host = env.objc.borrow_mut::<NSDataHostObject>(this);
    host.length = new_len;
    host.bytes = new_bytes;
}

@end

};

pub fn from_rust_slice(env: &mut Environment, slice: &[u8]) -> id {
    let length = slice.len().try_into().unwrap();
    let bytes = env.mem.alloc(length);
    env.mem
        .bytes_at_mut(bytes.cast(), length)
        .copy_from_slice(slice);
    let guest_object = msg_class![env; NSData alloc];
    let host_object = env.objc.borrow_mut::<NSDataHostObject>(guest_object);
    host_object.bytes = bytes;
    host_object.length = length;
    autorelease(env, guest_object)
}

pub fn to_rust_slice(env: &mut Environment, data: id) -> &[u8] {
    let borrowed_data = env.objc.borrow::<NSDataHostObject>(data);
    assert!(!borrowed_data.bytes.is_null() && borrowed_data.length != 0);
    env.mem
        .bytes_at(borrowed_data.bytes.cast(), borrowed_data.length)
}
