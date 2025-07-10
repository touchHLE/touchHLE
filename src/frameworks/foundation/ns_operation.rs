/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSOperation`, and associated classes
//!
//! Resources:
//! - [Apple's Concurrency Programming Guide](https://developer.apple.com/library/archive/documentation/General/Conceptual/ConcurrencyProgrammingGuide/OperationObjects/OperationObjects.html)

use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_super, nil, objc_classes, release, retain,
    ClassExports, HostObject, NSZonePtr, SEL,
};

struct NSOperationHostObject {
    is_canceled: bool,
}
impl HostObject for NSOperationHostObject {}

impl NSOperationHostObject {
    pub fn new() -> Self {
        Self { is_canceled: false }
    }
}

struct NSInvocationOperationHostObject {
    superclass: NSOperationHostObject,
    target: id,
    sel: Option<SEL>,
    arg: id,
}
impl_HostObject_with_superclass!(NSInvocationOperationHostObject);

impl NSInvocationOperationHostObject {
    pub fn new() -> Self {
        Self {
            superclass: NSOperationHostObject::new(),
            target: nil,
            sel: None,
            arg: nil,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);
@implementation NSOperation: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = NSOperationHostObject::new();
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

- (())start {
    let host_object: &NSOperationHostObject = env.objc.borrow(this);
    if host_object.is_canceled {
        return
    }
    let _: () = msg![env; this main];
}

- (())main {
    // "The default implementation of this method does nothing."
}

@end

@implementation NSInvocationOperation: NSOperation

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = NSInvocationOperationHostObject::new();
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

- (id)initWithTarget:(id)target selector:(SEL)sel object:(id)arg {
    let this: id  = msg_super![env; this init];
    let target = retain(env, target);
    let arg = retain(env, arg);
    if !env.objc.object_has_method(env.mem.as_mut(), target, sel) {
        return nil;
    }
    let host_object = env.objc.borrow_mut::<NSInvocationOperationHostObject>(this);
    host_object.target = target;
    host_object.sel = Some(sel);
    host_object.arg = arg;
    this
}

- (())main {
    let host_object = env.objc.borrow::<NSInvocationOperationHostObject>(this);

    let target = host_object.target;
    let sel = host_object.sel.unwrap();
    let arg = host_object.arg;
    log_dbg!("Running NSInvocationOperation [{:?} {:?}({}) {:?}]", target, sel, sel.as_str(env.mem.as_mut()),  arg);
    if arg.is_null() {
        let args = (target, sel);
        let _: () = crate::objc::msg_send(env, args);
    } else {
        let args = (target, sel, arg);
        let _: () = crate::objc::msg_send(env, args);
    }
}

-(())dealloc {
    let host_object = env.objc.borrow::<NSInvocationOperationHostObject>(this);
    let target = host_object.target;
    let arg = host_object.arg;
    release(env, target);
    release(env, arg);
    env.objc.dealloc_object(this, env.mem.as_mut());
}

@end

};
