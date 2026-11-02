/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UINavigationItem`. Navigation bar rendering is not yet implemented.

use crate::frameworks::foundation::ns_string::get_static_str;
use crate::objc::{id, msg, objc_classes, release, ClassExports, HostObject, NSZonePtr};

#[derive(Default)]
struct UINavigationItemHostObject {
    title: id,
}
impl HostObject for UINavigationItemHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UINavigationItem: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UINavigationItemHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithTitle:(id)title {
    () = msg![env; this setTitle:title];
    this
}

- (id)initWithCoder:(id)coder {
    let key = get_static_str(env, "UITitle");
    let title: id = msg![env; coder decodeObjectForKey:key];
    msg![env; this initWithTitle:title]
}

- (id)title {
    env.objc.borrow::<UINavigationItemHostObject>(this).title
}
- (())setTitle:(id)title {
    let title: id = msg![env; title copy];
    let host_object = env.objc.borrow_mut::<UINavigationItemHostObject>(this);
    let old_title = std::mem::replace(&mut host_object.title, title);
    release(env, old_title);
}

- (())dealloc {
    let title = env.objc.borrow::<UINavigationItemHostObject>(this).title;
    release(env, title);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};
