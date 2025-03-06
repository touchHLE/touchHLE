/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIBarButtonItem`.

use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg_super, nil, objc_classes, ClassExports, NSZonePtr,
};

struct UIBarButtonItemHostObject {
    superclass: super::UIControlHostObject,
    title: id,
    style: id,
    target: id,
    action: id,
    system_item: i32,
}

impl_HostObject_with_superclass!(UIBarButtonItemHostObject);

impl Default for UIBarButtonItemHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            title: nil,
            style: nil,
            target: nil,
            action: nil,
            system_item: 0,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

    (env, this, _cmd);

    @implementation UIBarButtonItem: UIControl

    + (id)allocWithZone:(NSZonePtr)_zone {
        let host_object = Box::<UIBarButtonItemHostObject>::default();
        env.objc.alloc_object(this, host_object, &mut env.mem)
    }

    - (id)init {
        msg_super![env; this init]
    }

    - (id)initWithTitle:(id)title
                  style:(id)style
                 target:(id)target
                 action:(id)action
    {
        log_dbg!(
            "[(UIBarButtonItem*){:?} initWithTitle:{:?} style:{:?} target:{:?} action:{:?}]",
            this,
            to_rust_string(env, title),
            style,
            target,
            action
        );

        let this: id = msg_super![env; this init];

        let host = env.objc.borrow_mut::<UIBarButtonItemHostObject>(this);
        host.title = title;
        host.style = style;
        host.target = target;
        host.action = action;
        host.system_item = 0;

        this
    }

    - (id)initWithBarButtonSystemItem:(i32)system_item
                               target:(id)target
                               action:(id)action
    {
        log_dbg!(
            "[(UIBarButtonItem*){:?} initWithBarButtonSystemItem:{} target:{:?} action:{:?}]",
            this,
            system_item,
            target,
            action
        );

        let this: id = msg_super![env; this init];

        let host = env.objc.borrow_mut::<UIBarButtonItemHostObject>(this);
        host.system_item = system_item;
        host.target = target;
        host.action = action;

        this
    }

    - (id)title {
        env.objc.borrow::<UIBarButtonItemHostObject>(this).title
    }
    - (())setTitle:(id)title {
        env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).title = title;
    }

    - (id)style {
        env.objc.borrow::<UIBarButtonItemHostObject>(this).style
    }
    - (())setStyle:(id)style {
        env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).style = style;
    }

    - (id)target {
        env.objc.borrow::<UIBarButtonItemHostObject>(this).target
    }
    - (())setTarget:(id)target {
        env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).target = target;
    }

    - (id)action {
        env.objc.borrow::<UIBarButtonItemHostObject>(this).action
    }
    - (())setAction:(id)action {
        env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).action = action;
    }

    - (i32)systemItem {
        env.objc.borrow::<UIBarButtonItemHostObject>(this).system_item
    }

    - (())dealloc {
        let UIBarButtonItemHostObject {
            superclass: _,
            title,
            style: _,
            target,
            action: _,
            ..
        } = std::mem::take(env.objc.borrow_mut(this));

        log_dbg!("dealloc [(UIBarButtonItem*){:?} title {:?}, target {:?}]", this, title, target);
        msg_super![env; this dealloc]
    }

    @end
};
