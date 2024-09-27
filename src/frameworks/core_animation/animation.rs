/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Functions, traits, and all kinds of things to assist with bridging the gap
//! between guest and host when it comes to animations in Core Animation.
//! Based in Apple's documented behavior for Core Animation, although not an
//! exact match.
use std::ops::{Add, Mul, Sub};

use crate::environment::Environment;
use crate::frameworks::core_animation::{ca_layer::CALayerHostObject, CACurrentMediaTime};
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::core_graphics::CGSize;
use crate::frameworks::core_graphics::{cg_color::CGColorHostObject, CGPoint, CGRect};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::msg;
use crate::objc::{id, nil};

macro_rules! id_as_option {
    ($value:expr) => {
        if $value == nil {
            None
        } else {
            Some($value)
        }
    };
}

impl Mul<f32> for CGPoint {
    type Output = CGPoint;

    fn mul(self, rhs: f32) -> Self::Output {
        CGPoint {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}
impl Mul<f32> for CGSize {
    type Output = CGSize;

    fn mul(self, rhs: f32) -> Self::Output {
        CGSize {
            width: self.width * rhs,
            height: self.height * rhs,
        }
    }
}
impl Mul<f32> for CGRect {
    type Output = CGRect;

    fn mul(self, rhs: f32) -> Self::Output {
        CGRect {
            origin: self.origin * rhs,
            size: self.size * rhs,
        }
    }
}
impl Mul<f32> for CGColorHostObject {
    type Output = CGColorHostObject;

    fn mul(self, rhs: f32) -> Self::Output {
        CGColorHostObject {
            color_space_name: self.color_space_name,
            r: self.r * rhs,
            g: self.g * rhs,
            b: self.b * rhs,
            a: self.a * rhs,
        }
    }
}

impl Add<CGPoint> for CGPoint {
    type Output = CGPoint;

    fn add(self, rhs: CGPoint) -> Self::Output {
        CGPoint {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}
impl Sub<&CGPoint> for &CGPoint {
    type Output = CGPoint;

    fn sub(self, rhs: &CGPoint) -> Self::Output {
        CGPoint {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Add<CGSize> for CGSize {
    type Output = CGSize;

    fn add(self, rhs: CGSize) -> Self::Output {
        CGSize {
            width: self.width + rhs.width,
            height: self.height + rhs.height,
        }
    }
}
impl Sub<&CGSize> for &CGSize {
    type Output = CGSize;

    fn sub(self, rhs: &CGSize) -> Self::Output {
        CGSize {
            width: self.width - rhs.width,
            height: self.height - rhs.height,
        }
    }
}

impl Add<CGRect> for CGRect {
    type Output = CGRect;

    fn add(self, rhs: CGRect) -> Self::Output {
        CGRect {
            origin: self.origin + rhs.origin,
            size: self.size + rhs.size,
        }
    }
}
impl Sub<&CGRect> for &CGRect {
    type Output = CGRect;

    fn sub(self, rhs: &CGRect) -> Self::Output {
        CGRect {
            origin: &self.origin - &rhs.origin,
            size: &self.size - &rhs.size,
        }
    }
}

impl Add<CGColorHostObject> for CGColorHostObject {
    type Output = CGColorHostObject;

    fn add(self, rhs: CGColorHostObject) -> Self::Output {
        CGColorHostObject {
            color_space_name: self.color_space_name,
            r: self.r + rhs.r,
            g: self.g + rhs.g,
            b: self.b + rhs.b,
            a: self.a + rhs.a,
        }
    }
}
impl Sub<&CGColorHostObject> for &CGColorHostObject {
    type Output = CGColorHostObject;

    fn sub(self, rhs: &CGColorHostObject) -> Self::Output {
        CGColorHostObject {
            color_space_name: self.color_space_name,
            r: self.r - rhs.r,
            g: self.g - rhs.g,
            b: self.b - rhs.b,
            a: self.a - rhs.a,
        }
    }
}

fn get_from_and_by_values<T>(
    current_value: Option<&T>,
    from_value: Option<&T>,
    to_value: Option<&T>,
    by_value: Option<&T>,
) -> (T, T)
where
    for<'a> &'a T: Sub<Output = T>,
    T: Clone,
{
    if from_value.is_some() && to_value.is_some() && by_value.is_some() {
        panic!("Cannot specify all three of fromValue, toValue, and byValue");
    } else if let (Some(from_value), Some(to_value)) = (from_value, to_value) {
        let by_value = to_value - from_value;
        (from_value.to_owned(), by_value)
    } else if let (Some(from_value), Some(by_value)) = (from_value, by_value) {
        (from_value.to_owned(), by_value.to_owned())
    } else if let (Some(to_value), Some(by_value)) = (to_value, by_value) {
        let from_value = to_value - by_value;
        (from_value, by_value.to_owned())
    } else if let Some(from_value) = from_value {
        let by_value = current_value.unwrap() - from_value;
        (from_value.to_owned(), by_value)
    } else if let Some(to_value) = to_value {
        let from_value = current_value.unwrap();
        let by_value = to_value - from_value;
        (from_value.to_owned(), by_value)
    } else if let Some(by_value) = by_value {
        let from_value = current_value.unwrap();
        (from_value.to_owned(), by_value.to_owned())
    } else {
        // TODO: All properties are nil. Interpolates between the previous
        // value of keyPath in the target layer’s presentation layer and the
        // current value of keyPath in the target layer’s presentation layer.
        unimplemented!()
    }
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
                let (from_value, by_value) = get_from_and_by_values(
                    Some(&presentation.anchor_point),
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                );
                presentation.anchor_point = from_value + by_value * interpolation_amount;
            }
            "backgroundColor" => {
                let from_value =
                    id_as_option!(from_value).map(|obj| env.objc.borrow::<CGColorHostObject>(obj));
                let to_value =
                    id_as_option!(to_value).map(|obj| env.objc.borrow::<CGColorHostObject>(obj));
                let by_value =
                    id_as_option!(by_value).map(|obj| env.objc.borrow::<CGColorHostObject>(obj));
                let (from_value, by_value) = get_from_and_by_values(
                    presentation.background_color.as_ref(),
                    from_value,
                    to_value,
                    by_value,
                );
                presentation.background_color = Some(from_value + by_value * interpolation_amount);
            }
            "bounds" => {
                let from_value = id_as_option!(from_value).map(|obj| msg![env; obj CGRectValue]);
                let to_value = id_as_option!(to_value).map(|obj| msg![env; obj CGRectValue]);
                let by_value = id_as_option!(by_value).map(|obj| msg![env; obj CGRectValue]);
                let (from_value, by_value) = get_from_and_by_values(
                    Some(&presentation.bounds),
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                );
                presentation.bounds = from_value + by_value * interpolation_amount;
            }
            "cornerRadius" => {
                let from_value = id_as_option!(from_value).map(|obj| msg![env; obj floatValue]);
                let to_value = id_as_option!(to_value).map(|obj| msg![env; obj floatValue]);
                let by_value = id_as_option!(by_value).map(|obj| msg![env; obj floatValue]);
                let (from_value, by_value) = get_from_and_by_values(
                    Some(&presentation.corner_radius),
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                );
                presentation.corner_radius = from_value + by_value * interpolation_amount;
            }
            "hidden" => {
                let from_value = id_as_option!(from_value)
                    .map(|obj| msg![env; obj boolValue])
                    .map(|val: bool| val as i32 as f32);
                let to_value = id_as_option!(to_value)
                    .map(|obj| msg![env; obj boolValue])
                    .map(|val: bool| val as i32 as f32);
                let by_value = id_as_option!(by_value)
                    .map(|obj| msg![env; obj boolValue])
                    .map(|val: bool| val as i32 as f32);
                let (from_value, by_value) = get_from_and_by_values(
                    Some(&(presentation.hidden as i32 as f32)),
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                );
                presentation.hidden = from_value + by_value * interpolation_amount > 0.5;
            }
            "opacity" => {
                let from_value = id_as_option!(from_value).map(|obj| msg![env; obj floatValue]);
                let to_value = id_as_option!(to_value).map(|obj| msg![env; obj floatValue]);
                let by_value = id_as_option!(by_value).map(|obj| msg![env; obj floatValue]);
                let (from_value, by_value) = get_from_and_by_values(
                    Some(&presentation.opacity),
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                );
                presentation.opacity = from_value + by_value * interpolation_amount;
            }
            "position" => {
                let from_value = id_as_option!(from_value).map(|obj| msg![env; obj CGPointValue]);
                let to_value = id_as_option!(to_value).map(|obj| msg![env; obj CGPointValue]);
                let by_value = id_as_option!(by_value).map(|obj| msg![env; obj CGPointValue]);
                let (from_value, by_value) = get_from_and_by_values(
                    Some(&presentation.position),
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                );
                presentation.position = from_value + by_value * interpolation_amount;
            }
            _ => panic!("Attempted to animate on key {}", key_path),
        }
    }

    presentation
}
