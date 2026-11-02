/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UINavigationBar` configuration. Rendering is not yet implemented.

use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_super, nil, objc_classes, release, ClassExports,
    NSZonePtr,
};

#[derive(Default)]
struct UINavigationBarHostObject {
    superclass: super::UIViewHostObject,
    items: id,
    /// Non-retaining UINavigationBarDelegate reference.
    delegate: id,
    bar_style: NSInteger,
    translucent: bool,
}
impl_HostObject_with_superclass!(UINavigationBarHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UINavigationBar: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UINavigationBarHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];
    let key = get_static_str(env, "UIItems");
    let items: id = msg![env; coder decodeObjectForKey:key];
    () = msg![env; this setItems:items];
    this
}

- (id)items {
    env.objc.borrow::<UINavigationBarHostObject>(this).items
}
- (())setItems:(id)items {
    () = msg![env; this setItems:items animated:false];
}
- (())setItems:(id)items animated:(bool)_animated {
    let items: id = msg![env; items copy];
    let host_object = env.objc.borrow_mut::<UINavigationBarHostObject>(this);
    let old_items = std::mem::replace(&mut host_object.items, items);
    release(env, old_items);
}

- (id)topItem {
    let items: id = msg![env; this items];
    msg![env; items lastObject]
}
- (id)backItem {
    let items: id = msg![env; this items];
    let count: NSUInteger = msg![env; items count];
    if count < 2 {
        nil
    } else {
        msg![env; items objectAtIndex:(count - 2)]
    }
}

- (id)delegate {
    env.objc.borrow::<UINavigationBarHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<UINavigationBarHostObject>(this).delegate = delegate;
}

- (NSInteger)barStyle {
    env.objc.borrow::<UINavigationBarHostObject>(this).bar_style
}
- (())setBarStyle:(NSInteger)style {
    env.objc.borrow_mut::<UINavigationBarHostObject>(this).bar_style = style;
}

- (bool)isTranslucent {
    env.objc.borrow::<UINavigationBarHostObject>(this).translucent
}
- (())setTranslucent:(bool)translucent {
    env.objc.borrow_mut::<UINavigationBarHostObject>(this).translucent = translucent;
}

- (())dealloc {
    let items = env.objc.borrow::<UINavigationBarHostObject>(this).items;
    release(env, items);
    msg_super![env; this dealloc]
}

@end

};
