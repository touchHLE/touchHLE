/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIViewController`.

use crate::objc::{id, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr};

#[derive(Default)]
struct UIViewControllerHostObject {
    view: id,
}
impl HostObject for UIViewControllerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIViewController: UIResponder

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIViewControllerHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    let &UIViewControllerHostObject { view } = env.objc.borrow(this);

    release(env, view);

    env.objc.dealloc_object(this, &mut env.mem);
}

- (())setView:(id)new_view { // UIView*
    let host_obj = env.objc.borrow_mut::<UIViewControllerHostObject>(this);
    let old_view = std::mem::replace(&mut host_obj.view, new_view);
    retain(env, new_view);
    release(env, old_view);
}
- (id)view {
    let view = env.objc.borrow_mut::<UIViewControllerHostObject>(this).view;
    assert!(view != nil); // TODO: call loadView if nil
    view
}

- (())setEditing:(bool)editing {
    log!("TODO: [(UIViewController*){:?} setEditing:{}]", this, editing); // TODO
}

@end

};
