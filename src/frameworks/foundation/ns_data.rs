/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSData` and `NSMutableData`.

use super::NSUInteger;
use super::ns_string::to_rust_string;
use crate::fs::GuestPath;
use crate::mem::{ConstVoidPtr, MutVoidPtr, Ptr};
use crate::objc::{
    autorelease, id, msg, objc_classes, retain, ClassExports, HostObject, NSZonePtr, nil, release,
};

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

- (id)initWithContentsOfFile:(id)path {
    let path = to_rust_string(env, path);
    log!("NSData::initWithContentsOfFile: {:?}", path);
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
- (bool)writeToFile:(id) path // NSString*
         atomically:(bool) _use_aux_file {
    let file = to_rust_string(env, path);
    log!("NSData::writeToFile:atomically: {:?}", file);
    let host_object = env.objc.borrow::<NSDataHostObject>(this);
    let slice = env.mem.bytes_at(host_object.bytes.cast(), host_object.length);
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

- (ConstVoidPtr)bytes {
    env.objc.borrow::<NSDataHostObject>(this).bytes.cast_const()
}
- (NSUInteger)length {
    env.objc.borrow::<NSDataHostObject>(this).length
}

@end

};
