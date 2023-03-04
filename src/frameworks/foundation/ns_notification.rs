/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSNotification`.

use crate::objc::{
    autorelease, id, msg, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

/// `NSString*`
pub type NSNotificationName = id;

struct NSNotificationHostObject {
    name: id,
    object: id,
    user_info: id,
}
impl HostObject for NSNotificationHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSNotification: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSNotificationHostObject {
        name: nil,
        object: nil,
        user_info: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)notificationWithName:(NSNotificationName)name
                    object:(id)object {
    msg![env; this notificationWithName:name object:object userInfo:nil]
}
+ (id)notificationWithName:(NSNotificationName)name
                    object:(id)object
                  userInfo:(id)user_info {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithName:name object:object userInfo:user_info];
    autorelease(env, new)
}

- (id)initWithName:(NSNotificationName)name
            object:(id)object
          userInfo:(id)user_info { // NSDictionary*
    retain(env, name);
    retain(env, object);
    let user_info: id = msg![env; user_info copy];
    *env.objc.borrow_mut(this) = NSNotificationHostObject { name, object, user_info };
    this
}
- (())dealloc {
    let &NSNotificationHostObject { name, object, user_info } = env.objc.borrow(this);
    release(env, name);
    release(env, object);
    release(env, user_info);
    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)name {
    env.objc.borrow::<NSNotificationHostObject>(this).name
}
- (id)object {
    env.objc.borrow::<NSNotificationHostObject>(this).object
}
- (id)userInfo {
    env.objc.borrow::<NSNotificationHostObject>(this).user_info
}
// TODO: setters

@end

};
