/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Functions, traits, and all kinds of things to assist with bridging the gap
//! between guest and host when it comes to animations in Core Animation.
//! Based in Apple's documented behavior for Core Animation, although not an
//! exact match.
use crate::environment::Environment;
use crate::frameworks::core_animation::{ca_layer::CALayerHostObject, CACurrentMediaTime};
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::core_graphics::{cg_color::CGColorHostObject, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::msg;
use crate::objc::{id, nil};

trait Interpolatable {
    fn interpolate(
        from_value: Option<&Self>,
        to_value: Option<&Self>,
        by_value: Option<&Self>,
        amount: f32,
    ) -> Self;
}
impl Interpolatable for bool {
    fn interpolate(
        from_value: Option<&Self>,
        to_value: Option<&Self>,
        by_value: Option<&Self>,
        amount: f32,
    ) -> Self {
        assert!(from_value.is_some());
        assert!(to_value.is_some());
        assert!(by_value.is_none());
        if amount > 0.5 {
            to_value.unwrap().to_owned()
        } else {
            from_value.unwrap().to_owned()
        }
    }
}
impl Interpolatable for f32 {
    fn interpolate(
        from_value: Option<&Self>,
        to_value: Option<&Self>,
        by_value: Option<&Self>,
        amount: f32,
    ) -> Self {
        assert!(from_value.is_some());
        assert!(to_value.is_some());
        assert!(by_value.is_none());
        let from_value = from_value.unwrap();
        let to_value = to_value.unwrap();
        let diff = to_value - from_value;
        from_value + diff * amount
    }
}
impl Interpolatable for CGPoint {
    fn interpolate(
        from_value: Option<&Self>,
        to_value: Option<&Self>,
        by_value: Option<&Self>,
        amount: f32,
    ) -> Self {
        assert!(from_value.is_some());
        assert!(to_value.is_some());
        assert!(by_value.is_none());
        let from_value = from_value.unwrap();
        let to_value = to_value.unwrap();
        let diff = CGPoint {
            x: to_value.x - from_value.x,
            y: to_value.y - from_value.y,
        };
        CGPoint {
            x: from_value.x + diff.x * amount,
            y: from_value.y + diff.y * amount,
        }
    }
}
impl Interpolatable for CGSize {
    fn interpolate(
        from_value: Option<&Self>,
        to_value: Option<&Self>,
        by_value: Option<&Self>,
        amount: f32,
    ) -> Self {
        assert!(from_value.is_some());
        assert!(to_value.is_some());
        assert!(by_value.is_none());
        let from_value = from_value.unwrap();
        let to_value = to_value.unwrap();
        let diff = CGSize {
            width: to_value.width - from_value.width,
            height: to_value.height - from_value.height,
        };
        CGSize {
            width: from_value.width + diff.width * amount,
            height: from_value.height + diff.height * amount,
        }
    }
}
impl Interpolatable for CGRect {
    // TODO: Reuse code for CGPoint and CGSize
    fn interpolate(
        from_value: Option<&Self>,
        to_value: Option<&Self>,
        by_value: Option<&Self>,
        amount: f32,
    ) -> Self {
        assert!(from_value.is_some());
        assert!(to_value.is_some());
        assert!(by_value.is_none());
        let from_value = from_value.unwrap();
        let to_value = to_value.unwrap();
        CGRect {
            origin: CGPoint::interpolate(
                Some(&from_value.origin),
                Some(&to_value.origin),
                None,
                amount,
            ),
            size: CGSize::interpolate(Some(&from_value.size), Some(&to_value.size), None, amount),
        }
    }
}
impl Interpolatable for CGColorHostObject {
    fn interpolate(
        from_value: Option<&Self>,
        to_value: Option<&Self>,
        by_value: Option<&Self>,
        amount: f32,
    ) -> Self {
        assert!(from_value.is_some());
        assert!(to_value.is_some());
        assert!(by_value.is_none());
        let from_value = from_value.unwrap();
        let to_value = to_value.unwrap();
        // TODO: support differing color spaces
        assert_eq!(from_value.color_space_name, to_value.color_space_name);
        let diff = CGColorHostObject {
            color_space_name: to_value.color_space_name,
            r: to_value.r - from_value.r,
            g: to_value.g - from_value.g,
            b: to_value.b - from_value.b,
            a: to_value.a - from_value.a,
        };
        CGColorHostObject {
            color_space_name: diff.color_space_name,
            r: from_value.r + diff.r * amount,
            g: from_value.g + diff.g * amount,
            b: from_value.b + diff.b * amount,
            a: from_value.a + diff.a * amount,
        }
    }
}

macro_rules! id_as_option {
    ($value:expr) => {
        if $value == nil {
            None
        } else {
            Some($value)
        }
    };
}

pub fn create_presentation_layer(env: &mut Environment, layer: id) -> CALayerHostObject {
    // Clone given layer
    let original = env.objc.borrow::<CALayerHostObject>(layer);
    let mut presentation = original.clone();

    // Loop over all animations and set the presentation layer's current values
    for (key, animation) in presentation.animations.clone() {
        log_dbg!(
            "Animate CALayer {:?} animation {}: {:?}",
            layer,
            to_rust_string(env, key),
            animation
        );

        // TODO: Convert to local time
        let current_time = CACurrentMediaTime(env);
        let mut begin_time: CFTimeInterval = msg![env; animation beginTime];
        if begin_time == 0.0 {
            begin_time = current_time;
            // TODO: Should the property be updated?
            // If not, where do we keep track of it?
            () = msg![env; animation setBeginTime: begin_time];
            // TODO: Call delegate to notify animation start
        }

        if current_time < begin_time {
            // Animation hasn't started yet
            continue;
        }

        let duration: CFTimeInterval = msg![env; animation duration];
        let current_repeat = ((current_time - begin_time) / duration) as f32;
        let repeat_count: f32 = msg![env; animation repeatCount];
        assert!(repeat_count >= 0.0);
        let mut progress = current_repeat.fract();

        let autoreverses: bool = msg![env; animation autoreverses];
        if autoreverses && (current_repeat.floor() as i32) % 2 == 1 {
            // Reverse progress
            progress = 1.0 - progress;
        }

        let timing_function: id = msg![env; animation timingFunction];
        let interpolation_amount: f32 = msg![env; timing_function _solveForInput: progress];

        // Assuming all animations here are CABasicAnimation
        // TODO: Handle other types of animations

        // TODO: Setting [repeatCount] to greatestFiniteMagnitude will cause
        // the animation to repeat forever.
        let effective_repeat_count = if repeat_count == 0.0 {
            1.0
        } else {
            repeat_count
        };
        if current_repeat >= effective_repeat_count {
            // TODO: Call delegate to notify animation end
            // TODO: Skip removal if removedOnCompletion is false
            () = msg![env; layer removeAnimationForKey: key];
            continue;
        }

        // TODO: Interpolation value behaviour with nil or byValue
        // https://developer.apple.com/documentation/quartzcore/cabasicanimation?language=objc
        let from_value: id = msg![env; animation fromValue];
        let to_value: id = msg![env; animation toValue];
        let by_value: id = msg![env; animation byValue];

        // Update values only in the presentation layer
        let key_path: id = msg![env; animation keyPath];
        let key_path = to_rust_string(env, key_path);
        // Only these properties are animatable
        // https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/CoreAnimation_guide/AnimatableProperties/AnimatableProperties.html#//apple_ref/doc/uid/TP40004514-CH11-SW2
        // TODO: Implement for all properties
        match &*key_path {
            "anchorPoint" => {
                let from_value = id_as_option!(from_value).map(|obj| msg![env; obj CGPointValue]);
                let to_value = id_as_option!(to_value).map(|obj| msg![env; obj CGPointValue]);
                let by_value = id_as_option!(by_value).map(|obj| msg![env; obj CGPointValue]);
                presentation.anchor_point = CGPoint::interpolate(
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                    interpolation_amount,
                )
            }
            "backgroundColor" => {
                let from_value =
                    id_as_option!(from_value).map(|obj| env.objc.borrow::<CGColorHostObject>(obj));
                let to_value =
                    id_as_option!(to_value).map(|obj| env.objc.borrow::<CGColorHostObject>(obj));
                let by_value =
                    id_as_option!(by_value).map(|obj| env.objc.borrow::<CGColorHostObject>(obj));
                presentation.background_color = Some(CGColorHostObject::interpolate(
                    from_value,
                    to_value,
                    by_value,
                    interpolation_amount,
                ))
            }
            "bounds" => {
                let from_value = id_as_option!(from_value).map(|obj| msg![env; obj CGRectValue]);
                let to_value = id_as_option!(to_value).map(|obj| msg![env; obj CGRectValue]);
                let by_value = id_as_option!(by_value).map(|obj| msg![env; obj CGRectValue]);
                presentation.bounds = CGRect::interpolate(
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                    interpolation_amount,
                )
            }
            "cornerRadius" => {
                let from_value = id_as_option!(from_value).map(|obj| msg![env; obj floatValue]);
                let to_value = id_as_option!(to_value).map(|obj| msg![env; obj floatValue]);
                let by_value = id_as_option!(by_value).map(|obj| msg![env; obj floatValue]);
                presentation.corner_radius = f32::interpolate(
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                    interpolation_amount,
                )
            }
            "hidden" => {
                let from_value = id_as_option!(from_value).map(|obj| msg![env; obj boolValue]);
                let to_value = id_as_option!(to_value).map(|obj| msg![env; obj boolValue]);
                let by_value = id_as_option!(by_value).map(|obj| msg![env; obj boolValue]);
                presentation.hidden = bool::interpolate(
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                    interpolation_amount,
                )
            }
            "opacity" => {
                let from_value = id_as_option!(from_value).map(|obj| msg![env; obj floatValue]);
                let to_value = id_as_option!(to_value).map(|obj| msg![env; obj floatValue]);
                let by_value = id_as_option!(by_value).map(|obj| msg![env; obj floatValue]);
                presentation.opacity = f32::interpolate(
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                    interpolation_amount,
                )
            }
            "position" => {
                let from_value = id_as_option!(from_value).map(|obj| msg![env; obj CGPointValue]);
                let to_value = id_as_option!(to_value).map(|obj| msg![env; obj CGPointValue]);
                let by_value = id_as_option!(by_value).map(|obj| msg![env; obj CGPointValue]);
                presentation.position = CGPoint::interpolate(
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                    interpolation_amount,
                )
            }
            _ => panic!("Attempted to animate on key {}", key_path),
        }
    }

    presentation
}
