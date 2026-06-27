/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISlider`.
//!
//! Useful resources:
//! - Apple's UISlider documentation:
//!   https://developer.apple.com/documentation/uikit/uislider

use crate::environment::Environment;
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::uikit::ui_view::ui_control::{send_actions, UIControlEventValueChanged};
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes, release,
    ClassExports, NSZonePtr,
};

const TRACK_HEIGHT: f32 = 7.0;
const THUMB_SIZE: f32 = 23.0;
const THUMB_RADIUS: f32 = THUMB_SIZE / 2.0;

// MARK: - HostObject

pub struct UISliderHostObject {
    pub(super) superclass: super::UIControlHostObject,
    pub(super) value: f32,
    pub(super) minimum_value: f32,
    pub(super) maximum_value: f32,
    pub(super) continuous: bool,
    track_left: id,
    track_right: id,
    thumb: id,
}
impl_HostObject_with_superclass!(UISliderHostObject);

impl Default for UISliderHostObject {
    fn default() -> Self {
        UISliderHostObject {
            superclass: Default::default(),
            value: 0.0,
            minimum_value: 0.0,
            maximum_value: 1.0,
            continuous: true,
            track_left: nil,
            track_right: nil,
            thumb: nil,
        }
    }
}

// MARK: - Helpers

fn normalized(value: f32, min: f32, max: f32) -> f32 {
    if (max - min).abs() < f32::EPSILON {
        return 0.0;
    }
    ((value - min) / (max - min)).max(0.0).min(1.0)
}

fn layout(env: &mut Environment, this: id) {
    let &UISliderHostObject {
        value,
        minimum_value,
        maximum_value,
        track_left,
        track_right,
        thumb,
        ..
    } = env.objc.borrow(this);

    let bounds: CGRect = msg![env; this bounds];
    let frac = normalized(value, minimum_value, maximum_value);

    let track_start_x = bounds.origin.x + THUMB_RADIUS;
    let track_end_x = bounds.origin.x + bounds.size.width - THUMB_RADIUS;
    let track_width = (track_end_x - track_start_x).max(0.0);
    let track_y = bounds.origin.y + (bounds.size.height - TRACK_HEIGHT) / 2.0;
    let thumb_cx = track_start_x + frac * track_width;

    let left_rect = CGRect {
        origin: CGPoint {
            x: track_start_x,
            y: track_y,
        },
        size: CGSize {
            width: (thumb_cx - track_start_x).max(0.0),
            height: TRACK_HEIGHT,
        },
    };
    let right_rect = CGRect {
        origin: CGPoint {
            x: thumb_cx,
            y: track_y,
        },
        size: CGSize {
            width: (track_end_x - thumb_cx).max(0.0),
            height: TRACK_HEIGHT,
        },
    };
    let thumb_rect = CGRect {
        origin: CGPoint {
            x: thumb_cx - THUMB_RADIUS,
            y: bounds.origin.y + (bounds.size.height - THUMB_SIZE) / 2.0,
        },
        size: CGSize {
            width: THUMB_SIZE,
            height: THUMB_SIZE,
        },
    };

    () = msg![env; track_left  setFrame:left_rect];
    () = msg![env; track_right setFrame:right_rect];
    () = msg![env; thumb       setFrame:thumb_rect];
}

fn init_common(env: &mut Environment, this: id) -> id {
    // Цвет обводки треков — тёмнее самого трека, полупрозрачный
    let track_border_color: id = msg_class![env; UIColor colorWithRed:(0.0f32)
                                                                green:(0.0f32)
                                                                 blue:(0.0f32)
                                                                alpha:(0.15f32)];
    let track_border_cg: id = msg![env; track_border_color CGColor];
    let track_r: CGFloat = (TRACK_HEIGHT / 2.0) as CGFloat;
    let track_bw: CGFloat = 0.5;

    // Синий трек
    let blue_color: id = msg_class![env; UIColor colorWithRed:(0.0f32)
                                                        green:(0.478f32)
                                                         blue:(1.0f32)
                                                        alpha:1.0f32];
    let track_left: id = msg_class![env; UIView new];
    () = msg![env; track_left setBackgroundColor:blue_color];
    {
        let layer: id = msg![env; track_left layer];
        () = msg![env; layer setCornerRadius:track_r];
        () = msg![env; layer setBorderColor:track_border_cg];
        () = msg![env; layer setBorderWidth:track_bw];
    }

    // Серый трек
    let gray_color: id = msg_class![env; UIColor colorWithRed:(0.78f32)
                                                        green:(0.78f32)
                                                         blue:(0.78f32)
                                                        alpha:1.0f32];
    let track_right: id = msg_class![env; UIView new];
    () = msg![env; track_right setBackgroundColor:gray_color];
    {
        let layer: id = msg![env; track_right layer];
        () = msg![env; layer setCornerRadius:track_r];
        () = msg![env; layer setBorderColor:track_border_cg];
        () = msg![env; layer setBorderWidth:track_bw];
    }

    // Белый thumb — обводка как у реального iOS-слайдера: серая, чуть темнее
    // Тусклый белый (чуть серый) — как на реальном iOS-слайдере
    let white_color: id = msg_class![env; UIColor colorWithRed:(0.85f32)
                                                         green:(0.85f32)
                                                          blue:(0.85f32)
                                                         alpha:1.0f32];
    let thumb: id = msg_class![env; UIView new];
    () = msg![env; thumb setBackgroundColor:white_color];
    {
        let layer: id = msg![env; thumb layer];
        let r: CGFloat = THUMB_RADIUS as CGFloat;
        () = msg![env; layer setCornerRadius:r];
        let thumb_border_color: id = msg_class![env; UIColor colorWithRed:(0.55f32)
                                                                     green:(0.55f32)
                                                                      blue:(0.55f32)
                                                                     alpha:(0.45f32)];
        let thumb_border_cg: id = msg![env; thumb_border_color CGColor];
        () = msg![env; layer setBorderColor:thumb_border_cg];
        let bw: CGFloat = 0.5;
        () = msg![env; layer setBorderWidth:bw];
    }

    {
        let host = env.objc.borrow_mut::<UISliderHostObject>(this);
        host.track_left = track_left;
        host.track_right = track_right;
        host.thumb = thumb;
    }

    () = msg![env; this addSubview:track_left];
    () = msg![env; this addSubview:track_right];
    () = msg![env; this addSubview:thumb];

    let clear: id = msg_class![env; UIColor clearColor];
    () = msg![env; this setBackgroundColor:clear];

    layout(env, this);
    this
}

fn value_from_touch_x(env: &mut Environment, this: id, touch_x: f32) -> f32 {
    let bounds: CGRect = msg![env; this bounds];
    let track_start_x = bounds.origin.x + THUMB_RADIUS;
    let track_end_x = bounds.origin.x + bounds.size.width - THUMB_RADIUS;
    let track_width = (track_end_x - track_start_x).max(1.0);
    let frac = ((touch_x - track_start_x) / track_width).max(0.0).min(1.0);
    let host = env.objc.borrow::<UISliderHostObject>(this);
    host.minimum_value + frac * (host.maximum_value - host.minimum_value)
}

// MARK: - Class

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UISlider: UIControl

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UISliderHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg_super![env; this initWithFrame:frame];
    init_common(env, this)
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];
    let this = init_common(env, this);

    let key = get_static_str(env, "UIValue");
    let value: f32 = msg![env; coder decodeFloatForKey:key];
    let key = get_static_str(env, "UIMinValue");
    let min: f32 = msg![env; coder decodeFloatForKey:key];
    let key = get_static_str(env, "UIMaxValue");
    let max: f32 = msg![env; coder decodeFloatForKey:key];

    {
        let host = env.objc.borrow_mut::<UISliderHostObject>(this);
        host.minimum_value = min;
        host.maximum_value = max;
        host.value = value.max(min).min(max);
    }
    layout(env, this);
    this
}

- (())dealloc {
    let UISliderHostObject {
        superclass: _, value: _, minimum_value: _, maximum_value: _, continuous: _,
        track_left, track_right, thumb,
    } = std::mem::take(env.objc.borrow_mut(this));
    release(env, track_left);
    release(env, track_right);
    release(env, thumb);
    msg_super![env; this dealloc]
}

- (())layoutSubviews {
    layout(env, this);
}

- (f32)value {
    env.objc.borrow::<UISliderHostObject>(this).value
}

- (())setValue:(f32)value {
    let (min, max) = {
        let h = env.objc.borrow::<UISliderHostObject>(this);
        (h.minimum_value, h.maximum_value)
    };
    env.objc.borrow_mut::<UISliderHostObject>(this).value = value.max(min).min(max);
    layout(env, this);
}

- (())setValue:(f32)value animated:(bool)_animated {
    msg![env; this setValue:value]
}

- (f32)minimumValue {
    env.objc.borrow::<UISliderHostObject>(this).minimum_value
}

- (())setMinimumValue:(f32)new_min {
    {
        let host = env.objc.borrow_mut::<UISliderHostObject>(this);
        host.minimum_value = new_min;
        if host.value < new_min { host.value = new_min; }
    }
    layout(env, this);
}

- (f32)maximumValue {
    env.objc.borrow::<UISliderHostObject>(this).maximum_value
}

- (())setMaximumValue:(f32)new_max {
    {
        let host = env.objc.borrow_mut::<UISliderHostObject>(this);
        host.maximum_value = new_max;
        if host.value > new_max { host.value = new_max; }
    }
    layout(env, this);
}

- (bool)isContinuous {
    env.objc.borrow::<UISliderHostObject>(this).continuous
}

- (())setContinuous:(bool)value {
    env.objc.borrow_mut::<UISliderHostObject>(this).continuous = value;
}

- (bool)beginTrackingWithTouch:(id)touch withEvent:(id)event {
    let touch_point: CGPoint = msg![env; touch locationInView:this];
    let new_value = value_from_touch_x(env, this, touch_point.x);
    env.objc.borrow_mut::<UISliderHostObject>(this).value = new_value;
    layout(env, this);
    let continuous = env.objc.borrow::<UISliderHostObject>(this).continuous;
    if continuous { send_actions(env, this, event, UIControlEventValueChanged); }
    true
}

- (bool)continueTrackingWithTouch:(id)touch withEvent:(id)event {
    let touch_point: CGPoint = msg![env; touch locationInView:this];
    let new_value = value_from_touch_x(env, this, touch_point.x);
    env.objc.borrow_mut::<UISliderHostObject>(this).value = new_value;
    layout(env, this);
    let continuous = env.objc.borrow::<UISliderHostObject>(this).continuous;
    if continuous { send_actions(env, this, event, UIControlEventValueChanged); }
    true
}

- (())endTrackingWithTouch:(id)touch withEvent:(id)event {
    let touch_point: CGPoint = msg![env; touch locationInView:this];
    let new_value = value_from_touch_x(env, this, touch_point.x);
    env.objc.borrow_mut::<UISliderHostObject>(this).value = new_value;
    layout(env, this);
    send_actions(env, this, event, UIControlEventValueChanged);
    msg_super![env; this endTrackingWithTouch:touch withEvent:event]
}

- (id)hitTest:(CGPoint)point withEvent:(id)event {
    if msg![env; this pointInside:point withEvent:event] { this } else { nil }
}

- (())setMinimumValueImage:(id)_image {}
- (())setMaximumValueImage:(id)_image {}
- (())setThumbImage:(id)_image forState:(u32)_state {}
- (())setMinimumTrackImage:(id)_image forState:(u32)_state {}
- (())setMaximumTrackImage:(id)_image forState:(u32)_state {}
- (id)thumbImageForState:(u32)_state        { nil }
- (id)minimumTrackImageForState:(u32)_state { nil }
- (id)maximumTrackImageForState:(u32)_state { nil }
- (id)minimumValueImage                     { nil }
- (id)maximumValueImage                     { nil }

@end

};
