/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Functions, traits, and all kinds of things to assist with bridging the gap
//! between guest and host when it comes to animations in Core Animation.
//! Based in Apple's documented behavior for Core Animation, although not an
//! exact match.
//! References:
//! - Core Animation Programming Guide
//!     https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/CoreAnimation_guide/Introduction/Introduction.html
//! - List of Animatable properties
//!     https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/CoreAnimation_guide/AnimatableProperties/AnimatableProperties.html#//apple_ref/doc/uid/TP40004514-CH11-SW2
//! - Animation timing behavior, layers' local time, autoreverses, etc.
//!     https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/CoreAnimation_guide/AdvancedAnimationTricks/AdvancedAnimationTricks.html
//! - Algorithm for choosing interpolation values
//!     https://developer.apple.com/documentation/quartzcore/cabasicanimation?language=objc
use std::ops::Sub;

use crate::frameworks::core_animation::ca_animation::{
    has_animation_started, set_animation_started,
};
use crate::frameworks::core_animation::ca_layer::remove_anonymous_animation_at_index;
use crate::frameworks::core_animation::{ca_layer::CALayerHostObject, CACurrentMediaTime};
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::core_graphics::cg_color::CGColorHostObject;
use crate::frameworks::foundation::ns_string::{from_rust_string, to_rust_string};
use crate::objc::{id, msg, nil, release, retain};
use crate::Environment;

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
    let named_animations: Vec<(bool, usize, String, id)> = presentation
        .animations
        .iter()
        .enumerate()
        .map(|(idx, (key, anim))| (true, idx, key.clone(), *anim))
        .collect();
    let anonymous_animations: Vec<(bool, usize, String, id)> = presentation
        .anonymous_animations
        .iter()
        .enumerate()
        .map(|(idx, anim)| (false, idx, format!("anonymous #{}", idx), *anim))
        .collect();

    // Retain layer a little bit longer just in case the delegate
    // decides to release it and we're left with a dangling pointer
    retain(env, layer);

    for (is_named_animation, idx, key, animation) in
        Iterator::chain(named_animations.iter(), anonymous_animations.iter())
    {
        let is_named_animation = *is_named_animation;
        let idx = *idx;
        let animation = *animation;

        // TODO: Convert to local time
        let current_time = CACurrentMediaTime(env);
        let mut begin_time: CFTimeInterval = msg![env; animation beginTime];
        if begin_time == 0.0 {
            begin_time = current_time;
            // TODO: Should the property be updated?
            // If not, where do we keep track of it?
            () = msg![env; animation setBeginTime: begin_time];
        }

        if current_time >= begin_time && !has_animation_started(env, animation) {
            // Animation started but isn't marked as such
            set_animation_started(env, animation);
            let delegate = msg![env; animation delegate];
            if delegate != nil {
                () = msg![env; delegate animationDidStart: animation];
            }
        }

        log_dbg!(
            "Animate CALayer {:?} animation {}: {:?}",
            layer,
            key,
            animation
        );

        let duration: CFTimeInterval = msg![env; animation duration];
        let current_repeat = ((current_time - begin_time).max(0.0) / duration) as f32;
        let repeat_count: f32 = msg![env; animation repeatCount];
        assert!(repeat_count >= 0.0);
        let mut progress = current_repeat.fract();

        let autoreverses: bool = msg![env; animation autoreverses];
        if autoreverses {
            // From the docs:
            // Setting the repeat count to a whole number (such as 1.0) for an
            // autoreversing animation causes the animation to stop on its
            // starting value.
            // Adding an extra half step (such as a repeat count of 1.5)
            // causes the animation to stop on its end value
            progress = ((progress * 2.0 - 1.0).abs() - 1.0).abs();
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
            let delegate = msg![env; animation delegate];
            if delegate != nil {
                () = msg![env; delegate animationDidStop: animation finished: true];
            }

            // TODO: Skip removal if removedOnCompletion is false
            if is_named_animation {
                let key = from_rust_string(env, key.clone());
                () = msg![env; layer removeAnimationForKey: key];
            } else {
                remove_anonymous_animation_at_index(env, layer, idx);
            }
            continue;
        }

        let from_value: id = msg![env; animation fromValue];
        let to_value: id = msg![env; animation toValue];
        let by_value: id = msg![env; animation byValue];

        // Update values only in the presentation layer
        let key_path: id = msg![env; animation keyPath];
        let key_path = to_rust_string(env, key_path);
        // Only these properties are animatable
        // TODO: Implement for all properties
        match &*key_path {
            "anchorPoint" => {
                let from_value = id_as_option(from_value).map(|obj| msg![env; obj CGPointValue]);
                let to_value = id_as_option(to_value).map(|obj| msg![env; obj CGPointValue]);
                let by_value = id_as_option(by_value).map(|obj| msg![env; obj CGPointValue]);
                let (from_value, by_value) = get_from_and_by_values(
                    Some(&presentation.anchor_point),
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                );
                presentation.anchor_point = &from_value + &(&by_value * interpolation_amount);
            }
            "backgroundColor" => {
                let from_value =
                    id_as_option(from_value).map(|obj| env.objc.borrow::<CGColorHostObject>(obj));
                let to_value =
                    id_as_option(to_value).map(|obj| env.objc.borrow::<CGColorHostObject>(obj));
                let by_value =
                    id_as_option(by_value).map(|obj| env.objc.borrow::<CGColorHostObject>(obj));
                let (from_value, by_value) = get_from_and_by_values(
                    presentation.background_color.as_ref(),
                    from_value,
                    to_value,
                    by_value,
                );
                presentation.background_color =
                    Some(&from_value + &(&by_value * interpolation_amount));
            }
            "bounds" => {
                let from_value = id_as_option(from_value).map(|obj| msg![env; obj CGRectValue]);
                let to_value = id_as_option(to_value).map(|obj| msg![env; obj CGRectValue]);
                let by_value = id_as_option(by_value).map(|obj| msg![env; obj CGRectValue]);
                let (from_value, by_value) = get_from_and_by_values(
                    Some(&presentation.bounds),
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                );
                presentation.bounds = &from_value + &(&by_value * interpolation_amount);
            }
            "cornerRadius" => {
                let from_value = id_as_option(from_value).map(|obj| msg![env; obj floatValue]);
                let to_value = id_as_option(to_value).map(|obj| msg![env; obj floatValue]);
                let by_value = id_as_option(by_value).map(|obj| msg![env; obj floatValue]);
                let (from_value, by_value) = get_from_and_by_values(
                    Some(&presentation.corner_radius),
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                );
                presentation.corner_radius = &from_value + &(&by_value * interpolation_amount);
            }
            "hidden" => {
                let from_value = id_as_option(from_value)
                    .map(|obj| msg![env; obj boolValue])
                    .map(|val: bool| val as i32 as f32);
                let to_value = id_as_option(to_value)
                    .map(|obj| msg![env; obj boolValue])
                    .map(|val: bool| val as i32 as f32);
                let by_value = id_as_option(by_value)
                    .map(|obj| msg![env; obj boolValue])
                    .map(|val: bool| val as i32 as f32);
                let (from_value, by_value) = get_from_and_by_values(
                    Some(&(presentation.hidden as i32 as f32)),
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                );
                presentation.hidden = (&from_value + &(&by_value * interpolation_amount)) > 0.5;
            }
            "opacity" => {
                let from_value = id_as_option(from_value).map(|obj| msg![env; obj floatValue]);
                let to_value = id_as_option(to_value).map(|obj| msg![env; obj floatValue]);
                let by_value = id_as_option(by_value).map(|obj| msg![env; obj floatValue]);
                let (from_value, by_value) = get_from_and_by_values(
                    Some(&presentation.opacity),
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                );
                presentation.opacity = &from_value + &(&by_value * interpolation_amount);
            }
            "position" => {
                let from_value = id_as_option(from_value).map(|obj| msg![env; obj CGPointValue]);
                let to_value = id_as_option(to_value).map(|obj| msg![env; obj CGPointValue]);
                let by_value = id_as_option(by_value).map(|obj| msg![env; obj CGPointValue]);
                let (from_value, by_value) = get_from_and_by_values(
                    Some(&presentation.position),
                    from_value.as_ref(),
                    to_value.as_ref(),
                    by_value.as_ref(),
                );
                presentation.position = &from_value + &(&by_value * interpolation_amount);
            }
            _ => panic!("Attempted to animate on key {}", key_path),
        }
    }

    release(env, layer);

    presentation
}

fn id_as_option(value: id) -> Option<id> {
    if value == nil {
        None
    } else {
        Some(value)
    }
}
