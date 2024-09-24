/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIViewController`.

use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::uikit::ui_view::set_view_controller;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};

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

- (id)initWithCoder:(id)coder {
    let key_ns_string = get_static_str(env, "UIView");
    let view: id = msg![env; coder decodeObjectForKey:key_ns_string];

    () = msg![env; this setView:view];

    this
}

- (())dealloc {
    let &UIViewControllerHostObject { view } = env.objc.borrow(this);

    release(env, view);

    env.objc.dealloc_object(this, &mut env.mem);
}

- (())loadView {
    // TODO: Check if the UIViewController has an associated nib file and load
    // the view from there instead if it does
    let view: id = msg_class![env; UIView alloc];
    let view: id = msg![env; view init];
    () = msg![env; this setView: view];
}
- (())setView:(id)new_view { // UIView*
    let host_obj = env.objc.borrow_mut::<UIViewControllerHostObject>(this);
    let old_view = std::mem::replace(&mut host_obj.view, new_view);
    if old_view != nil {
        set_view_controller(env, old_view, nil);
    }
    if new_view != nil {
        set_view_controller(env, new_view, this);
    }
    retain(env, new_view);
    release(env, old_view);
}
- (id)view {
    let view = env.objc.borrow_mut::<UIViewControllerHostObject>(this).view;
    if view == nil {
        () = msg![env; this loadView];
        let view = env.objc.borrow_mut::<UIViewControllerHostObject>(this).view;
        view
    } else {
        view
    }
}

- (())setEditing:(bool)editing {
    log!("TODO: [(UIViewController*){:?} setEditing:{}]", this, editing); // TODO
}

- (())dismissModalViewControllerAnimated:(bool)animated {
    log!("TODO: [(UIViewController*){:?} dismissModalViewControllerAnimated:{}]", this, animated); // TODO
}

@end

};
