/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIBarButtonItem`.

use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::objc::{
    id, msg, msg_super, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
    SEL,
};

type UIBarStyle = NSInteger;

pub struct UIBarButtonItemHostObject {
    title: id,
    image: id,
    /// Weak, matching UIKit's target-action ownership convention.
    target: id,
    action: Option<SEL>,
    style: UIBarStyle,
    enabled: bool,
}
impl HostObject for UIBarButtonItemHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIBarButtonItem: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(
        this,
        Box::new(UIBarButtonItemHostObject {
            title: nil,
            image: nil,
            target: nil,
            action: None,
            style: 0,
            enabled: true,
        }),
        &mut env.mem
    )
}

- (id)initWithCoder:(id)coder {
    let this = msg_super![env; this init];

    let title_key = get_static_str(env, "UITitle");
    let image_key = get_static_str(env, "UIImage");
    let style_key = get_static_str(env, "UIBarStyle");
    let title: id = msg![env; coder decodeObjectForKey:title_key];
    let image: id = msg![env; coder decodeObjectForKey:image_key];
    let style: UIBarStyle = msg![env; coder decodeIntegerForKey:style_key];

    retain(env, title);
    retain(env, image);
    let item = env.objc.borrow_mut::<UIBarButtonItemHostObject>(this);
    item.title = title;
    item.image = image;
    item.style = style;
    this
}

- (id)initWithTitle:(id)new_title
              style:(UIBarStyle)new_style
             target:(id)new_target
             action:(SEL)new_action {
    let this = msg_super![env; this init];
    retain(env, new_title);
    let item = env.objc.borrow_mut::<UIBarButtonItemHostObject>(this);
    item.title = new_title;
    item.style = new_style;
    item.target = new_target;
    item.action = Some(new_action);
    this
}

- (id)initWithImage:(id)new_image
              style:(UIBarStyle)new_style
             target:(id)new_target
             action:(SEL)new_action {
    let this = msg_super![env; this init];
    retain(env, new_image);
    let item = env.objc.borrow_mut::<UIBarButtonItemHostObject>(this);
    item.image = new_image;
    item.style = new_style;
    item.target = new_target;
    item.action = Some(new_action);
    this
}

- (id)title {
    env.objc.borrow::<UIBarButtonItemHostObject>(this).title
}

- (())setTitle:(id)new_title {
    let item = env.objc.borrow_mut::<UIBarButtonItemHostObject>(this);
    let old_title = std::mem::replace(&mut item.title, new_title);
    retain(env, new_title);
    release(env, old_title);
}

- (id)image {
    env.objc.borrow::<UIBarButtonItemHostObject>(this).image
}

- (())setImage:(id)new_image {
    let item = env.objc.borrow_mut::<UIBarButtonItemHostObject>(this);
    let old_image = std::mem::replace(&mut item.image, new_image);
    retain(env, new_image);
    release(env, old_image);
}

- (UIBarStyle)style {
    env.objc.borrow::<UIBarButtonItemHostObject>(this).style
}

- (())setStyle:(UIBarStyle)new_style {
    env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).style = new_style;
}

- (id)target {
    env.objc.borrow::<UIBarButtonItemHostObject>(this).target
}

- (())setTarget:(id)new_target {
    env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).target = new_target;
}

- (())setAction:(SEL)new_action {
    env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).action = Some(new_action);
}

- (())addTarget:(id)new_target
         action:(SEL)new_action
forControlEvents:(NSUInteger)_events {
    // UIBarButtonItem has one target-action pair, unlike UIControl's
    // event-specific action list. NIB event connections use this selector.
    let item = env.objc.borrow_mut::<UIBarButtonItemHostObject>(this);
    item.target = new_target;
    item.action = Some(new_action);
}

- (bool)isEnabled {
    env.objc.borrow::<UIBarButtonItemHostObject>(this).enabled
}

- (())setEnabled:(bool)new_enabled {
    env.objc.borrow_mut::<UIBarButtonItemHostObject>(this).enabled = new_enabled;
}

- (())dealloc {
    let (title, image) = {
        let item = env.objc.borrow::<UIBarButtonItemHostObject>(this);
        (item.title, item.image)
    };
    release(env, title);
    release(env, image);
    msg_super![env; this dealloc]
}

@end

};
