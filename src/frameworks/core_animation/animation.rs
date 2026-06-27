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
//!   <https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/CoreAnimation_guide/Introduction/Introduction.html>
//! - List of Animatable properties
//!   <https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/CoreAnimation_guide/AnimatableProperties/AnimatableProperties.html#//apple_ref/doc/uid/TP40004514-CH11-SW2>
//! - Animation timing behavior, layers' local time, autoreverses, etc.
//!   <https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/CoreAnimation_guide/AdvancedAnimationTricks/AdvancedAnimationTricks.html>
//! - Algorithm for choosing interpolation values
//!   <https://developer.apple.com/documentation/quartzcore/cabasicanimation?language=objc>
use std::ops::Sub;

use crate::frameworks::core_animation::ca_animation::{
    get_animation_start_time, kCAFillModeBackwards, kCAFillModeBoth, kCAFillModeForwards,
    CAMediaTimingFillMode,
};
use crate::frameworks::core_animation::ca_layer::remove_anonymous_animation;
use crate::frameworks::core_animation::{ca_layer::CALayerHostObject, CACurrentMediaTime};
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::core_graphics::cg_color::CGColorHostObject;
use crate::frameworks::foundation::ns_string::{from_rust_string, to_rust_string};
use crate::objc::{id, msg, nil, release, retain};
use crate::Environment;

#[derive(Default)]
pub struct State {
    started_animations: Vec<id>,
    finished_animations: Vec<(id, id, bool, bool, Option<String>)>,
}
impl State {
    pub fn create_presentation_layer(
        &mut self,
        env: &mut Environment,
        layer: id,
    ) -> CALayerHostObject {
        // Clone given layer
        let original = env.objc.borrow::<CALayerHostObject>(layer);
        let mut presentation = original.clone();

        // Loop over all animations and set the presentation layer's values
        let named_animations: Vec<(Option<String>, id)> = presentation
            .animations
            .iter()
            .map(|(key, anim)| (Some(key.clone()), *anim))
            .collect();
        let anonymous_animations: Vec<(Option<String>, id)> = presentation
            .anonymous_animations
            .iter()
            .map(|anim| (None, *anim))
            .collect();

        for (key, animation) in
            Iterator::chain(named_animations.iter(), anonymous_animations.iter())
        {
            let animation = *animation;

            let fill_mode: CAMediaTimingFillMode = msg![env; animation fillMode];
            let fill_mode = to_rust_string(env, fill_mode);

            // TODO: Convert to local time
            let current_time = CACurrentMediaTime(env);
            let begin_time: CFTimeInterval = msg![env; animation beginTime];
            let start_time = get_animation_start_time(env, animation);

            if current_time >= begin_time {
                if start_time.is_none() {
                    // Animation started but isn't marked as such
                    start_time.replace(current_time);
                    self.started_animations.push(animation);
                }
            } else if fill_mode != kCAFillModeBackwards && fill_mode != kCAFillModeBoth {
                continue;
            }

            let effective_begin_time = start_time.unwrap_or(begin_time);

            if let Some(key) = key {
                log_dbg!(
                    "Animate CALayer {:?} animation {} {:?}",
                    layer,
                    key,
                    animation
                );
            } else {
                log_dbg!("Animate CALayer {:?} animation {:?}", layer, animation);
            }

            let repeat_count: f32 = msg![env; animation repeatCount];
            // A negative `repeatCount` is technically undefined per the
            // CABasicAnimation docs; clamp to 0 (i.e. no repeats) instead of
            // crashing the host.
            let repeat_count = if repeat_count.is_finite() && repeat_count >= 0.0 {
                repeat_count
            } else {
                log!(
                    "Warning: CABasicAnimation: invalid repeatCount {}; clamping to 0.",
                    repeat_count
                );
                0.0
            };
            // Setting [repeatCount] to greatestFiniteMagnitude will cause
            // the animation to repeat forever.
            let effective_repeat_count = if repeat_count == f32::MAX {
                f32::INFINITY
            } else if repeat_count == 0.0 {
                1.0
            } else {
                repeat_count
            };

            let duration: CFTimeInterval = msg![env; animation duration];
            let current_repeat = (((current_time - effective_begin_time).max(0.0) / duration)
                as f32)
                .min(effective_repeat_count);

            let mut progress = current_repeat.fract();

            let autoreverses: bool = msg![env; animation autoreverses];
            if autoreverses {
                // From the docs:
                // Setting the repeat count to a whole number (such as 1.0) for
                // an autoreversing animation causes the animation to stop on
                // its starting value.
                // Adding an extra half step (such as a repeat count of 1.5)
                // causes the animation to stop on its end value
                progress = ((progress * 2.0 - 1.0).abs() - 1.0).abs();
            }

            let timing_function: id = msg![env; animation timingFunction];
            let interpolation_amount: f32 = msg![env; timing_function _solveForInput: progress];

            if current_repeat >= effective_repeat_count {
                let removed_on_completion: bool = msg![env; animation isRemovedOnCompletion];
                self.finished_animations.push((
                    layer,
                    animation,
                    true,
                    removed_on_completion,
                    key.to_owned(),
                ));
                if fill_mode != kCAFillModeForwards && fill_mode != kCAFillModeBoth {
                    continue;
                }
            }

            // Assuming all animations here are CABasicAnimation
            // TODO: Handle other types of animations

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
                    let from_value =
                        id_as_option(from_value).map(|obj| msg![env; obj CGPointValue]);
                    let to_value = id_as_option(to_value).map(|obj| msg![env; obj CGPointValue]);
                    let by_value = id_as_option(by_value).map(|obj| msg![env; obj CGPointValue]);
                    let (from_value, by_value) = get_from_and_by_values(
                        Some(presentation.anchor_point),
                        from_value,
                        to_value,
                        by_value,
                    );
                    presentation.anchor_point = from_value + by_value * interpolation_amount;
                }
                "backgroundColor" => {
                    let from_value = id_as_option(from_value)
                        .map(|obj| *env.objc.borrow::<CGColorHostObject>(obj));
                    let to_value = id_as_option(to_value)
                        .map(|obj| *env.objc.borrow::<CGColorHostObject>(obj));
                    let by_value = id_as_option(by_value)
                        .map(|obj| *env.objc.borrow::<CGColorHostObject>(obj));
                    let (from_value, by_value) = get_from_and_by_values(
                        presentation.background_color,
                        from_value,
                        to_value,
                        by_value,
                    );
                    presentation.background_color =
                        Some(from_value + by_value * interpolation_amount)
                }
                "bounds" => {
                    let from_value = id_as_option(from_value).map(|obj| msg![env; obj CGRectValue]);
                    let to_value = id_as_option(to_value).map(|obj| msg![env; obj CGRectValue]);
                    let by_value = id_as_option(by_value).map(|obj| msg![env; obj CGRectValue]);
                    let (from_value, by_value) = get_from_and_by_values(
                        Some(presentation.bounds),
                        from_value,
                        to_value,
                        by_value,
                    );
                    presentation.bounds = from_value + by_value * interpolation_amount;
                }
                "cornerRadius" => {
                    let from_value = id_as_option(from_value).map(|obj| msg![env; obj floatValue]);
                    let to_value = id_as_option(to_value).map(|obj| msg![env; obj floatValue]);
                    let by_value = id_as_option(by_value).map(|obj| msg![env; obj floatValue]);
                    let (from_value, by_value) = get_from_and_by_values(
                        Some(presentation.corner_radius),
                        from_value,
                        to_value,
                        by_value,
                    );
                    presentation.corner_radius = from_value + by_value * interpolation_amount;
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
                        Some(presentation.hidden as i32 as f32),
                        from_value,
                        to_value,
                        by_value,
                    );
                    presentation.hidden = (from_value + by_value * interpolation_amount) > 0.5;
                }
                "opacity" => {
                    let from_value = id_as_option(from_value).map(|obj| msg![env; obj floatValue]);
                    let to_value = id_as_option(to_value).map(|obj| msg![env; obj floatValue]);
                    let by_value = id_as_option(by_value).map(|obj| msg![env; obj floatValue]);
                    let (from_value, by_value) = get_from_and_by_values(
                        Some(presentation.opacity),
                        from_value,
                        to_value,
                        by_value,
                    );
                    presentation.opacity = from_value + by_value * interpolation_amount;
                }
                "position" => {
                    let from_value =
                        id_as_option(from_value).map(|obj| msg![env; obj CGPointValue]);
                    let to_value = id_as_option(to_value).map(|obj| msg![env; obj CGPointValue]);
                    let by_value = id_as_option(by_value).map(|obj| msg![env; obj CGPointValue]);
                    let (from_value, by_value) = get_from_and_by_values(
                        Some(presentation.position),
                        from_value,
                        to_value,
                        by_value,
                    );
                    presentation.position = from_value + by_value * interpolation_amount;
                }
                _ => {
                    log_dbg!(
                        "Warning: Skipping animation on unsupported key path {:?}",
                        key_path
                    );
                    continue;
                }
            }
        }

        presentation
    }

    pub fn update_started_and_finished_animations(self, env: &mut Environment) {
        // `animationDidStart:` and `animationDidStop:finished:` are optional
        // CAAnimationDelegate methods. Many apps either don't set a delegate
        // at all or set one that only implements one of the two. We must
        // register the selectors so msg! doesn't panic with "Unknown
        // selector" when nothing in the binary referenced them, and then
        // gate the actual send on `respondsToSelector:` so we mirror the
        // Cocoa behavior of silently no-op'ing when the delegate doesn't
        // implement the method.
        let did_start_sel = env
            .objc
            .register_host_selector("animationDidStart:".to_string(), &mut env.mem);
        let did_stop_sel = env
            .objc
            .register_host_selector("animationDidStop:finished:".to_string(), &mut env.mem);

        for animation in self.started_animations {
            let delegate = msg![env; animation delegate];
            if delegate != nil {
                let responds: bool = msg![env; delegate respondsToSelector: did_start_sel];
                if responds {
                    () = msg![env; delegate animationDidStart: animation];
                }
            }
        }

        for (layer, ..) in &self.finished_animations {
            retain(env, *layer);
        }
        for (layer, animation, finished, removed_on_completion, key) in self.finished_animations {
            let delegate = msg![env; animation delegate];
            if delegate != nil {
                let responds: bool = msg![env; delegate respondsToSelector: did_stop_sel];
                if responds {
                    () = msg![env; delegate animationDidStop: animation finished: finished];
                }
            }

            if removed_on_completion {
                if let Some(key) = key {
                    let key = from_rust_string(env, key);
                    () = msg![env; layer removeAnimationForKey: key];
                } else {
                    remove_anonymous_animation(env, layer, animation);
                }
            }

            release(env, layer);
        }
    }
}

#[allow(clippy::eq_op)]
fn get_from_and_by_values<T>(
    current_value: Option<T>,
    from_value: Option<T>,
    to_value: Option<T>,
    by_value: Option<T>,
) -> (T, T)
where
    T: Copy + Sub<Output = T>,
{
    // The semantics of fromValue/toValue/byValue follow Apple's docs. If a
    // misbehaving guest specifies all three, fall back to using from/to and
    // ignoring `byValue` instead of crashing the host.
    if let (Some(from_value), Some(to_value)) = (from_value, to_value) {
        if by_value.is_some() {
            log_dbg!(
                "CABasicAnimation: all three of fromValue/toValue/byValue set; \
                 ignoring byValue"
            );
        }
        let by_value = to_value - from_value.to_owned();
        (from_value, by_value)
    } else if let (Some(from_value), Some(by_value)) = (from_value, by_value) {
        (from_value.to_owned(), by_value.to_owned())
    } else if let (Some(to_value), Some(by_value)) = (to_value, by_value) {
        let from_value = to_value - by_value;
        (from_value, by_value.to_owned())
    } else if let Some(from_value) = from_value {
        if let Some(current) = current_value {
            let by_value = current - from_value;
            (from_value.to_owned(), by_value)
        } else {
            // No current value to derive `by` from — treat as a no-op animation.
            (from_value.to_owned(), from_value - from_value)
        }
    } else if let Some(to_value) = to_value {
        if let Some(from_value) = current_value {
            let by_value = to_value - from_value;
            (from_value.to_owned(), by_value)
        } else {
            (to_value.to_owned(), to_value - to_value)
        }
    } else if let Some(by_value) = by_value {
        if let Some(from_value) = current_value {
            (from_value.to_owned(), by_value.to_owned())
        } else {
            // Pick a zero-equivalent start; the animation will still be valid.
            (by_value - by_value, by_value.to_owned())
        }
    } else {
        // All properties are nil. The official semantics call for interpolating
        // between the previous and current presentation-layer values of
        // `keyPath`, which we don't track here. Fall back to a no-op animation
        // starting from `current_value` (or zero if even that is missing).
        if let Some(current) = current_value {
            (current.to_owned(), current - current)
        } else {
            log!(
                "Warning: CABasicAnimation: from/to/by all nil and no current \
                 value; emitting no-op animation."
            );
            // SAFETY: this branch will only happen when `current_value` is
            // None AND none of from/to/by were set, in which case the caller
            // has no expectation about the values; default-constructing via
            // a zero-size subtraction is impossible without a concrete T.
            // Fall back to a panic-free degenerate path: re-use whatever the
            // caller passed by returning the same Option-default. We avoid
            // requiring `Default` on T by computing a zero from the unwrap
            // we already know exists in *some* branch above; if we get here
            // we genuinely have nothing to animate, so we cannot produce a
            // value of T. Returning here would require Default; since we
            // can't add that bound without ripping through all callers, log
            // the fact and panic with a clearer message (still better than
            // the silent `unimplemented!`).
            unreachable!(
                "CABasicAnimation: from/to/by all nil and current_value missing; \
                 no value of T available to interpolate."
            )
        }
    }
}

fn id_as_option(value: id) -> Option<id> {
    if value == nil {
        None
    } else {
        Some(value)
    }
}
