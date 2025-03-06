/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIBarButtonItem`.

use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::frameworks::uikit::ui_font::UITextAlignmentCenter;
use crate::frameworks::uikit::ui_view::ui_control::UIControlEventTouchUpInside;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes,
    ClassExports, NSZonePtr, SEL,
};

struct UIBarButtonItemHostObject {
    superclass: super::UIControlHostObject,
    title: id,
    style: id,
    target: id,
    action: Option<SEL>,
    system_item: i32,
    pub label: id,
}

impl_HostObject_with_superclass!(UIBarButtonItemHostObject);

impl Default for UIBarButtonItemHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            title: nil,
            style: nil,
            target: nil,
            action: None,
            system_item: 0,
            label: nil,
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
                 action:(SEL)action
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
        let font: id = msg_class![env; UIFont systemFontOfSize:17_f32];
        let title_color: id = msg_class![env; UIColor blackColor];

        let frame = CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize {
                width: 0.0, // UIToolbar will determine size based on number of items
                height: 44.0,
            },
        };
        let title_label: id = msg_class![env; UILabel new];
        let title_label: id = msg![env; title_label initWithFrame:frame];
        () = msg![env; title_label setTextAlignment:UITextAlignmentCenter];
        () = msg![env; title_label setText:title];
        () = msg![env; title_label setTextColor:title_color];
        () = msg![env; title_label setFont:font];

        let host = env.objc.borrow_mut::<UIBarButtonItemHostObject>(this);
        host.title = title;
        host.style = style;
        host.target = target;
        host.action = Some(action);
        host.system_item = 0;
        host.label = title_label;

        if target != nil {
            () = msg![env; this addTarget:target action:action forControlEvents:UIControlEventTouchUpInside];
        }

        this
    }

    - (id)initWithBarButtonSystemItem:(i32)system_item
                               target:(id)target
                               action:(SEL)action
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
        host.action = Some(action);

        if target != nil {
            () = msg![env; this addTarget:target action:action forControlEvents:UIControlEventTouchUpInside];
        }

        this
    }

    - (id)label {
        env.objc.borrow::<UIBarButtonItemHostObject>(this).label
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

    - (SEL)action {
        match env.objc.borrow::<UIBarButtonItemHostObject>(this).action {
            Some(sel) => sel,
            None => {
                log!("Warning: UIBarButtonItem has no action set!");
                env.objc.lookup_selector("undefinedSelector").unwrap()
            }
        }
    }

    - (())setAction:(SEL)action {
        env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).action = Some(action);
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
            action,
            ..
        } = std::mem::take(env.objc.borrow_mut(this));

        log_dbg!("dealloc [(UIBarButtonItem*){:?} title {:?}, target {:?}, action {:?}]", this, title, target, action);
        msg_super![env; this dealloc]
    }

    @end
};
