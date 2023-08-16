/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIButton`.

use super::{UIControlState, UIControlStateNormal};
use crate::frameworks::core_graphics::CGRect;
use crate::frameworks::foundation::NSInteger;
use crate::frameworks::uikit::ui_font::UITextAlignmentCenter;
use crate::objc::{
    autorelease, id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes,
    release, ClassExports, NSZonePtr,
};

type UIButtonType = NSInteger;
const UIButtonTypeCustom: UIButtonType = 0;
const UIButtonTypeRoundedRect: UIButtonType = 1;
#[allow(dead_code)]
const UIButtonTypeDetailDisclosure: UIButtonType = 2;
#[allow(dead_code)]
const UIButtonTypeInfoLight: UIButtonType = 3;
#[allow(dead_code)]
const UIButtonTypeInfoDark: UIButtonType = 4;
#[allow(dead_code)]
const UIButtonTypeContactAdd: UIButtonType = 5;

pub struct UIButtonHostObject {
    superclass: super::UIControlHostObject,
    type_: UIButtonType,
    /// `UILabel*`
    title_label: id,
}
impl_HostObject_with_superclass!(UIButtonHostObject);
impl Default for UIButtonHostObject {
    fn default() -> Self {
        UIButtonHostObject {
            superclass: Default::default(),
            type_: UIButtonTypeCustom,
            title_label: nil,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIButton: UIControl

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIButtonHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)buttonWithType:(UIButtonType)type_ {
    let button: id = msg![env; this new];
    match type_ {
        UIButtonTypeCustom => (),
        UIButtonTypeRoundedRect => {
            let bg_color: id = msg_class![env; UIColor whiteColor];
            () = msg![env; button setBackgroundColor:bg_color];
            // On the real iPhone OS, this is a semi-dark, desaturated blue.
            // Should we match it?
            let text_color: id = msg_class![env; UIColor blackColor];
            () = msg![env; button setTitleColor:text_color
                                       forState:UIControlStateNormal];
            // TODO: set border and corner rounding, once supported
        },
        _ => {
            log!("TODO: UIButtonType {}", type_);
        }
    }
    autorelease(env, button)
}

- (id)init {
    let this: id = msg_super![env; this init];

    () = msg![env; this setOpaque:false];
    let bg_color: id = msg_class![env; UIColor clearColor];
    () = msg![env; this setBackgroundColor:bg_color];

    let title_label: id = msg_class![env; UILabel new];
    let text_color: id = msg_class![env; UIColor whiteColor];
    () = msg![env; title_label setTextColor:text_color];
    () = msg![env; title_label setBackgroundColor:bg_color];
    () = msg![env; title_label setTextAlignment:UITextAlignmentCenter];

    env.objc.borrow_mut::<UIButtonHostObject>(this).title_label = title_label;

    () = msg![env; this addSubview:title_label];

    this
}

- (())dealloc {
    let &UIButtonHostObject {
        superclass: _,
        type_: _,
        title_label
    } = env.objc.borrow(this);
    release(env, title_label);
    msg_super![env; this dealloc]
}

- (())layoutSubviews {
    let label = env.objc.borrow_mut::<UIButtonHostObject>(this).title_label;
    let bounds: CGRect = msg![env; this bounds];
    () = msg![env; label setFrame:bounds];
}

- (UIButtonType)buttonType {
    env.objc.borrow_mut::<UIButtonHostObject>(this).type_
}

- (id)titleLabel {
    env.objc.borrow_mut::<UIButtonHostObject>(this).title_label
}

- (())setTitle:(id)title // NSString*
      forState:(UIControlState)state {
    // TODO: handle state changes
    if state != UIControlStateNormal {
        return;
    }
    let label: id = msg![env; this titleLabel];
    () = msg![env; label setText:title];
}

- (())setTitleColor:(id)color // UIColor*
      forState:(UIControlState)state {
    // TODO: handle state changes
    if state != UIControlStateNormal {
        return;
    }
    let label: id = msg![env; this titleLabel];
    () = msg![env; label setTextColor:color];
}

// TODO: images, touch input, etc

@end

};
