/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSKeyedArchiver` - Currently just a fake implementation.

use crate::objc::{
    autorelease, id, msg_class, nil, objc_classes, ClassExports, HostObject,
    NSZonePtr,
};
use plist::Dictionary;

struct NSKeyedArchiverHostObject {
    data: id,
    plist: Dictionary,
    /// Something responding to NSKeyedUnarchiverDelegate
    delegate: id,
}
impl HostObject for NSKeyedArchiverHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSKeyedArchiver: NSCoder

+ (id)allocWithZone:(NSZonePtr)_zone { // struct _NSZone*
    let archiver = Box::new(NSKeyedArchiverHostObject {
        delegate: nil,
        data: nil,
        plist: Dictionary::new(),
    });
    env.objc.alloc_object(this, archiver, &mut env.mem)
}

+ (id)archivedDataWithRootObject:(id)_rootObject { // NSData *
    let data: id = msg_class![env; NSMutableData new];
    autorelease(env, data)
}

// TODO: other init methods.

- (id)initForWritingWithMutableData:(id)data { // NSData *
    if data == nil {
        return nil;
    }

    let host_obj = env.objc.borrow_mut::<NSKeyedArchiverHostObject>(this);
    assert!(host_obj.data.is_null());

    host_obj.data = data;
    host_obj.plist = Dictionary::new();

    this
}

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

// TODO: implement calls to delegate methods
// weak/non-retaining
- (())setDelegate:(id)delegate { // id<NSKeyedUnarchiverDelegate>
    let host_object = env.objc.borrow_mut::<NSKeyedArchiverHostObject>(this);
    host_object.delegate = delegate;
}
- (id)delegate {
    env.objc.borrow::<NSKeyedArchiverHostObject>(this).delegate
}

- (bool)containsValueForKey:(id)key { // NSString*
    assert!(key != nil);
    return false;
}

// TODO: add more decode methods

@end

};