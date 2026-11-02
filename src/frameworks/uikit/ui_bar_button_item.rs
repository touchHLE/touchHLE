/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Basic bar item properties. Button rendering is not yet implemented.

use crate::abi::GuestArg;
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_super, nil, objc_classes, release, ClassExports,
    HostObject, NSZonePtr, SEL,
};

struct UIBarItemHostObject {
    title: id,
    enabled: bool,
}
impl HostObject for UIBarItemHostObject {}
impl Default for UIBarItemHostObject {
    fn default() -> Self {
        Self {
            title: nil,
            enabled: true,
        }
    }
}

struct UIBarButtonItemHostObject {
    superclass: UIBarItemHostObject,
    style: NSInteger,
    /// Non-retaining target for the action.
    target: id,
    action: SEL,
}
impl_HostObject_with_superclass!(UIBarButtonItemHostObject);
impl Default for UIBarButtonItemHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            style: 0,
            target: nil,
            // A bar button item can have a null action selector.
            action: SEL::from_regs(&[0]),
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIBarItem: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIBarItemHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    let key = get_static_str(env, "UITitle");
    let title: id = msg![env; coder decodeObjectForKey:key];
    () = msg![env; this setTitle:title];
    let key = get_static_str(env, "UIEnabled");
    let has_enabled: bool = msg![env; coder containsValueForKey:key];
    if has_enabled {
        let enabled: bool = msg![env; coder decodeBoolForKey:key];
        () = msg![env; this setEnabled:enabled];
    }
    this
}

- (id)title {
    env.objc.borrow::<UIBarItemHostObject>(this).title
}
- (())setTitle:(id)title {
    let title: id = msg![env; title copy];
    let host_object = env.objc.borrow_mut::<UIBarItemHostObject>(this);
    let old_title = std::mem::replace(&mut host_object.title, title);
    release(env, old_title);
}

- (bool)isEnabled {
    env.objc.borrow::<UIBarItemHostObject>(this).enabled
}
- (())setEnabled:(bool)enabled {
    env.objc.borrow_mut::<UIBarItemHostObject>(this).enabled = enabled;
}

- (())dealloc {
    let title = env.objc.borrow::<UIBarItemHostObject>(this).title;
    release(env, title);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

@implementation UIBarButtonItem: UIBarItem

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIBarButtonItemHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];
    let key = get_static_str(env, "UIStyle");
    let style: NSInteger = msg![env; coder decodeIntForKey:key];
    () = msg![env; this setStyle:style];
    this
}

- (NSInteger)style {
    env.objc.borrow::<UIBarButtonItemHostObject>(this).style
}
- (())setStyle:(NSInteger)style {
    env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).style = style;
}

- (id)target {
    env.objc.borrow::<UIBarButtonItemHostObject>(this).target
}
- (())setTarget:(id)target {
    env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).target = target;
}

- (SEL)action {
    env.objc.borrow::<UIBarButtonItemHostObject>(this).action
}
- (())setAction:(SEL)action {
    env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).action = action;
}

@end

};
