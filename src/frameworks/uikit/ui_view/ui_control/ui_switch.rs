/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISwitch`.

use crate::environment::Environment;
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string;
use crate::frameworks::uikit::ui_font::UITextAlignmentCenter;
use crate::frameworks::uikit::ui_view::ui_control::{
    send_actions, UIControlEventTouchUpInside, UIControlEventValueChanged,
};
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes, release,
    ClassExports, NSZonePtr,
};

// TODO: rendering: round corners, shadows, gradients. etc.
// TODO: animation
// TODO: drag-to-flip

// Those parameters are not "exact",
// but just chosen to look nice ;)
const BACK_INSET: f32 = 1.0;
const THUMB_INSET: f32 = 1.0;
const THUMB_WIDTH: f32 = 42.0;
// Those correspond to debugDescription output of UISwitch
const TOTAL_WIDTH: f32 = 94.0;
const TOTAL_HEIGHT: f32 = 27.0;

pub struct UISwitchHostObject {
    superclass: super::UIControlHostObject,
    is_on: bool,
    /// `UIView*`
    back: id,
    /// `UIView*`
    thumb: id,
    /// `UILabel*`
    label_on: id,
    /// `UILabel*`
    label_off: id,
}
impl_HostObject_with_superclass!(UISwitchHostObject);
impl Default for UISwitchHostObject {
    fn default() -> Self {
        UISwitchHostObject {
            superclass: Default::default(),
            is_on: false,
            thumb: nil,
            back: nil,
            label_on: nil,
            label_off: nil,
        }
    }
}

fn update(env: &mut Environment, this: id) {
    let &mut UISwitchHostObject {
        is_on,
        back,
        label_on,
        label_off,
        ..
    } = env.objc.borrow_mut(this);

    let enabled: bool = msg![env; this isEnabled];
    if enabled {
        () = msg![env; this setAlpha:1.0f32];
    } else {
        () = msg![env; this setAlpha:0.8f32];
    };

    let back_color: id = if is_on {
        msg![env; label_on backgroundColor]
    } else {
        msg![env; label_off backgroundColor]
    };
    () = msg![env; back setBackgroundColor:back_color];

    () = msg![env; label_on setHidden:(!is_on)];
    () = msg![env; label_off setHidden:is_on];

    // we need to force re-layout
    () = msg![env; this layoutSubviews];
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UISwitch: UIControl

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UISwitchHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    log_dbg!("[(UISwitch*){:?} initWithFrame:{}] (frame.size is ignored)", this, frame);
    // According to the docs,
    // the size components of the frame rectangle are ignored.
    let frame = CGRect {
        origin: frame.origin,
        size: CGSize {
            width: TOTAL_WIDTH,
            height: TOTAL_HEIGHT,
        },
    };
    let this: id = msg_super![env; this initWithFrame:frame];

    let size: CGFloat = 17.0;
    let font: id = msg_class![env; UIFont boldSystemFontOfSize:size];

    let white_color: id = msg_class![env; UIColor whiteColor];
    let light_gray_color: id = msg_class![env; UIColor lightGrayColor];
    let blue_color: id = msg_class![env; UIColor colorWithRed:(83.0f32/255.0)
                                                        green:(141.0f32/255.0)
                                                         blue:(235.0f32/255.0)
                                                        alpha:1.0f32];
    let thumb_color: id = msg_class![env; UIColor colorWithRed:(205.0f32/255.0)
                                                         green:(205.0f32/255.0)
                                                          blue:(205.0f32/255.0)
                                                         alpha:1.0f32];

    // This is actually sets the "border" color
    () = msg![env; this setBackgroundColor:light_gray_color];

    let back: id = msg_class![env; UIView new];
    () = msg![env; back setBackgroundColor:white_color];

    let thumb: id = msg_class![env; UIView new];
    () = msg![env; thumb setBackgroundColor:thumb_color];

    let label_on: id = msg_class![env; UILabel new];
    () = msg![env; label_on setBackgroundColor:blue_color];
    () = msg![env; label_on setTextAlignment:UITextAlignmentCenter];
    let text = ns_string::get_static_str(env, "ON");
    () = msg![env; label_on setText:text];
    () = msg![env; label_on setTextColor:white_color];
    () = msg![env; label_on setFont:font];

    let label_off: id = msg_class![env; UILabel new];
    () = msg![env; label_off setBackgroundColor:white_color];
    () = msg![env; label_off setTextAlignment:UITextAlignmentCenter];
    let text = ns_string::get_static_str(env, "OFF");
    () = msg![env; label_off setText:text];
    let text_color: id = msg_class![env; UIColor darkGrayColor];
    () = msg![env; label_off setTextColor:text_color];
    () = msg![env; label_off setFont:font];

    let host_obj = env.objc.borrow_mut::<UISwitchHostObject>(this);
    host_obj.back = back;
    host_obj.thumb = thumb;
    host_obj.label_on = label_on;
    host_obj.label_off = label_off;

    () = msg![env; this addSubview:back];
    () = msg![env; this addSubview:label_on];
    () = msg![env; this addSubview:label_off];
    () = msg![env; this addSubview:thumb];
    update(env, this);

    let selector = env.objc.lookup_selector("_touchHLE_flipAction:forEvent:").unwrap();
    () = msg![env; this addTarget:this
                           action:selector
                 forControlEvents:UIControlEventTouchUpInside];

    this
}

- (id)initWithCoder:(id)_coder {
    todo!()
}

- (())layoutSubviews {
    let &mut UISwitchHostObject {
        is_on,
        back,
        thumb,
        label_on,
        label_off,
        ..
    } = env.objc.borrow_mut(this);

    let bounds: CGRect = msg![env; this bounds];

    let back_rect: CGRect = CGRect {
        origin: CGPoint {
            x: bounds.origin.x + BACK_INSET,
            y: bounds.origin.y + BACK_INSET
        },
        size: CGSize {
            width: bounds.size.width - 2.0 * BACK_INSET,
            height: bounds.size.height - 2.0 * BACK_INSET,
        }
    };
    // Below rects are all defined in reference to back_rect, not whole bounds,
    // in order to accommodate for the border
    let thumb_rect: CGRect = if is_on {
        CGRect {
            origin: CGPoint {
                x: back_rect.size.width - (THUMB_WIDTH - 2.0 * THUMB_INSET),
                y: back_rect.origin.y + THUMB_INSET
            },
            size: CGSize {
                width: THUMB_WIDTH - 2.0 * THUMB_INSET,
                height: back_rect.size.height - 2.0 * THUMB_INSET
            }
        }
    } else {
        CGRect {
            origin: CGPoint {
                x: back_rect.origin.x + THUMB_INSET,
                y: back_rect.origin.y + THUMB_INSET
            },
            size: CGSize {
                width: THUMB_WIDTH - 2.0 * THUMB_INSET,
                height: back_rect.size.height - 2.0 * THUMB_INSET
            }
        }
    };
    let label_on_rect: CGRect = CGRect {
        origin: CGPoint {
            x: back_rect.origin.x,
            y: back_rect.origin.y
        },
        size: CGSize {
            width: back_rect.size.width - THUMB_WIDTH,
            height: back_rect.size.height
        }
    };
    let label_off_rect: CGRect = CGRect {
        origin: CGPoint {
            x: back_rect.origin.x + THUMB_WIDTH,
            y: back_rect.origin.y
        },
        size: CGSize {
            width: back_rect.size.width - THUMB_WIDTH,
            height: back_rect.size.height
        }
    };

    () = msg![env; back setFrame:back_rect];
    () = msg![env; thumb setFrame:thumb_rect];
    () = msg![env; label_on setFrame:label_on_rect];
    () = msg![env; label_off setFrame:label_off_rect];
}

- (())dealloc {
    let UISwitchHostObject {
        superclass: _,
        is_on: _,
        back,
        thumb,
        label_on,
        label_off,
    } = std::mem::take(env.objc.borrow_mut(this));

    release(env, back);
    release(env, thumb);
    release(env, label_on);
    release(env, label_off);
    msg_super![env; this dealloc]
}

- (())setEnabled:(bool)enabled {
    () = msg_super![env; this setEnabled:enabled];
    update(env, this);
}
- (())setSelected:(bool)selected {
    () = msg_super![env; this setSelected:selected];
    update(env, this);
}
- (())setHighlighted:(bool)highlighted {
    () = msg_super![env; this setHighlighted:highlighted];
    update(env, this);
}

- (())setOn:(bool)on animated:(bool)_animated {
    // TODO: support animation
    env.objc.borrow_mut::<UISwitchHostObject>(this).is_on = on;
    update(env, this);
}

- (bool)isOn {
     env.objc.borrow::<UISwitchHostObject>(this).is_on
}

- (id)hitTest:(CGPoint)point
    withEvent:(id)event { // UIEvent* (possibly nil)
    // Hide subviews from hit testing so event goes straight to this control
    if msg![env; this pointInside:point withEvent:event] {
        this
    } else {
        nil
    }
}

// "Private" methods, not a part of API

- (())_touchHLE_flipAction:(id)sender forEvent:(id)event { // UIEvent*
    assert_eq!(this, sender);

    let is_on = env.objc.borrow::<UISwitchHostObject>(this).is_on;
    () = msg![env; this setOn:(!is_on) animated:false];

    send_actions(env, this, event, UIControlEventValueChanged);
}

@end

};
