/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIBarButtonItem`.

use super::UIBarItemHostObject;
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::frameworks::foundation::NSInteger;
use crate::frameworks::uikit::ui_font::UITextAlignmentCenter;
use crate::frameworks::uikit::ui_view::ui_control::UIControlEventTouchUpInside;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes, release,
    retain, ClassExports, NSZonePtr, SEL,
};

pub type UIBarButtonItemStyle = NSInteger;
pub const UIBarButtonItemStylePlain: UIBarButtonItemStyle = 0;
pub const UIBarButtonItemStyleBordered: UIBarButtonItemStyle = 1;
pub const UIBarButtonItemStyleDone: UIBarButtonItemStyle = 2;

pub type UIBarButtonSystemItem = NSInteger;
pub const UIBarButtonSystemItemFlexibleSpace: UIBarButtonSystemItem = 5;
pub const UIBarButtonSystemItemFixedSpace: UIBarButtonSystemItem = 6;

pub struct UIBarButtonItemHostObject {
    superclass: UIBarItemHostObject,
    pub style: UIBarButtonItemStyle,
    pub target: id,
    pub action: Option<SEL>,
    pub system_item: Option<UIBarButtonSystemItem>,
    /// Internal UIControl used for rendering and touch handling in UIToolbar.
    pub custom_view: id,
    /// Internal UILabel within custom_view, exposed for sizing queries.
    pub label: id,
    pub width: CGFloat,
}

impl_HostObject_with_superclass!(UIBarButtonItemHostObject);

impl Default for UIBarButtonItemHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            style: UIBarButtonItemStylePlain,
            target: nil,
            action: None,
            system_item: None,
            custom_view: nil,
            label: nil,
            width: 0.0,
        }
    }
}

fn make_label(env: &mut crate::Environment, title: id) -> id {
    let font: id = msg_class![env; UIFont systemFontOfSize:17_f32];
    let title_color: id = msg_class![env; UIColor blackColor];
    let item_bg_color: id = msg_class![env; UIColor whiteColor];
    let init_frame = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width: 0.0,
            height: 44.0,
        },
    };
    let label: id = msg_class![env; UILabel new];
    let label: id = msg![env; label initWithFrame:init_frame];
    () = msg![env; label setTextAlignment:UITextAlignmentCenter];
    () = msg![env; label setText:title];
    () = msg![env; label setTextColor:title_color];
    () = msg![env; label setFont:font];
    () = msg![env; label setBackgroundColor:item_bg_color];
    let layer: id = msg![env; label layer];
    () = msg![env; layer setCornerRadius:(10.0 as CGFloat)];
    label
}

fn make_custom_view(env: &mut crate::Environment, label: id, target: id, action: SEL) -> id {
    let init_frame = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: CGSize {
            width: 0.0,
            height: 44.0,
        },
    };
    let control: id = msg_class![env; UIControl alloc];
    let control: id = msg![env; control initWithFrame:init_frame];
    if label != nil {
        () = msg![env; control addSubview:label];
    }
    if target != nil {
        () = msg![env; control addTarget:target action:action forControlEvents:UIControlEventTouchUpInside];
    }
    control
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIBarButtonItem: UIBarItem

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIBarButtonItemHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCustomView:(id)customView {
    log_dbg!(
        "[(UIBarButtonItem*){:?} initWithCustomView:{:?}]",
        this,
        customView
    );

    {
        let host = env.objc.borrow_mut::<UIBarButtonItemHostObject>(this);
        host.custom_view = customView;
    }

    if customView != nil {
        retain(env, customView);
    }

    msg_super![env; this init]
}

- (id)initWithTitle:(id)title
                style:(UIBarButtonItemStyle)style
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

    let label = make_label(env, title);
    let custom_view = make_custom_view(env, label, target, action);

    {
        let host = env.objc.borrow_mut::<UIBarButtonItemHostObject>(this);
        host.superclass.title = title;
        host.style = style;
        host.target = target;
        host.action = Some(action);
        host.custom_view = custom_view;
        host.label = label;
    }

    if title != nil {
        retain(env, title);
    }
    if target != nil {
        retain(env, target);
    }
    retain(env, custom_view);

    msg_super![env; this init]
}

- (id)initWithBarButtonSystemItem:(UIBarButtonSystemItem)system_item
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

    let label = if system_item == UIBarButtonSystemItemFlexibleSpace
        || system_item == UIBarButtonSystemItemFixedSpace
    {
        nil
    } else {
        unimplemented!(
            "UIBarButtonSystemItem {} rendering not implemented",
            system_item
        );
    };

    let custom_view = make_custom_view(env, label, target, action);

    {
        let host = env.objc.borrow_mut::<UIBarButtonItemHostObject>(this);
        host.system_item = Some(system_item);
        host.target = target;
        host.action = Some(action);
        host.custom_view = custom_view;
        host.label = label;
    }

    if target != nil {
        retain(env, target);
    }
    retain(env, custom_view);

    msg_super![env; this init]
}

- (id)customView {
    env.objc.borrow::<UIBarButtonItemHostObject>(this).custom_view
}

- (id)label {
    env.objc.borrow::<UIBarButtonItemHostObject>(this).label
}

- (id)title {
    env.objc.borrow::<UIBarButtonItemHostObject>(this).superclass.title
}

- (())setTitle:(id)title {
    let label = env.objc.borrow::<UIBarButtonItemHostObject>(this).label;
    let old_title = env.objc.borrow::<UIBarButtonItemHostObject>(this).superclass.title;
    retain(env, title);
    release(env, old_title);
    env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).superclass.title = title;
    if label != nil {
        () = msg![env; label setText:title];
    }
}

- (UIBarButtonItemStyle)style {
    env.objc.borrow::<UIBarButtonItemHostObject>(this).style
}

- (())setStyle:(UIBarButtonItemStyle)style {
    match style {
        UIBarButtonItemStylePlain
        | UIBarButtonItemStyleBordered
        | UIBarButtonItemStyleDone => {}
        _ => unimplemented!("UIBarButtonItemStyle {}", style),
    }
    env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).style = style;
}

- (id)target {
    env.objc.borrow::<UIBarButtonItemHostObject>(this).target
}

- (())setTarget:(id)target {
    env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).target = target;
}

- (CGFloat)width {
    env.objc.borrow::<UIBarButtonItemHostObject>(this).width
}

- (())setWidth:(CGFloat)width {
    env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).width = width;
}

- (CGSize)sizeThatFits:(CGSize)size {
    let host = env.objc.borrow::<UIBarButtonItemHostObject>(this);
    let label = host.label;
    let custom_view = host.custom_view;
    if label != nil {
        let label_size: CGSize = msg![env; label sizeThatFits:size];
        CGSize {
            width: label_size.width + 16.0,
            height: size.height,
        }
    } else if custom_view != nil {
        msg![env; custom_view sizeThatFits:size]
    } else {
        CGSize { width: 0.0, height: size.height }
    }
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

- (())dealloc {
    let UIBarButtonItemHostObject {
        superclass,
        style: _,
        target,
        action: _,
        system_item: _,
        custom_view,
        label,
        width: _,
    } = std::mem::take(env.objc.borrow_mut(this));

    log_dbg!(
        "dealloc [(UIBarButtonItem*){:?} title {:?}, target {:?}, custom_view {:?}, label {:?}]",
        this, superclass.title, target, custom_view, label
    );

    release(env, superclass.title);
    release(env, target);
    release(env, custom_view);
    msg_super![env; this dealloc]
}

@end

};
