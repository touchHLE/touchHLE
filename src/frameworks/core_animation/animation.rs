use crate::environment::Environment;
use crate::frameworks::core_animation::{ca_layer::CALayerHostObject, CACurrentMediaTime};
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::core_graphics::{cg_color::CGColorHostObject, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::msg;
use crate::objc::{id, nil};

trait Interpolatable {
    fn interpolate(
        env: &mut Environment,
        from_value: id,
        to_value: id,
        by_value: id,
        amount: f32,
    ) -> Self;
}
impl Interpolatable for bool {
    fn interpolate(
        env: &mut Environment,
        from_value: id,
        to_value: id,
        by_value: id,
        amount: f32,
    ) -> Self {
        assert_ne!(from_value, nil);
        assert_ne!(to_value, nil);
        assert_eq!(by_value, nil);
        let from_value: bool = msg![env; from_value boolValue];
        let to_value: bool = msg![env; to_value boolValue];
        if amount > 0.5 {
            to_value
        } else {
            from_value
        }
    }
}
impl Interpolatable for f32 {
    fn interpolate(
        env: &mut Environment,
        from_value: id,
        to_value: id,
        by_value: id,
        amount: f32,
    ) -> Self {
        assert_ne!(from_value, nil);
        assert_ne!(to_value, nil);
        assert_eq!(by_value, nil);
        let from_value: f32 = msg![env; from_value floatValue];
        let to_value: f32 = msg![env; to_value floatValue];
        let diff = to_value - from_value;
        from_value + diff * amount
    }
}
impl Interpolatable for CGPoint {
    fn interpolate(
        env: &mut Environment,
        from_value: id,
        to_value: id,
        by_value: id,
        amount: f32,
    ) -> Self {
        assert_ne!(from_value, nil);
        assert_ne!(to_value, nil);
        assert_eq!(by_value, nil);
        let from_value: CGPoint = msg![env; from_value CGPointValue];
        let to_value: CGPoint = msg![env; to_value CGPointValue];
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
        env: &mut Environment,
        from_value: id,
        to_value: id,
        by_value: id,
        amount: f32,
    ) -> Self {
        assert_ne!(from_value, nil);
        assert_ne!(to_value, nil);
        assert_eq!(by_value, nil);
        let from_value: CGSize = msg![env; from_value CGSizeValue];
        let to_value: CGSize = msg![env; to_value CGSizeValue];
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
        env: &mut Environment,
        from_value: id,
        to_value: id,
        by_value: id,
        amount: f32,
    ) -> Self {
        assert_ne!(from_value, nil);
        assert_ne!(to_value, nil);
        assert_eq!(by_value, nil);
        let from_value: CGRect = msg![env; from_value CGRectValue];
        let to_value: CGRect = msg![env; to_value CGRectValue];
        let diff: CGRect = CGRect {
            origin: CGPoint {
                x: to_value.origin.x - from_value.origin.x,
                y: to_value.origin.y - from_value.origin.y,
            },
            size: CGSize {
                width: to_value.size.width - from_value.size.width,
                height: to_value.size.height - from_value.size.height,
            },
        };
        CGRect {
            origin: CGPoint {
                x: from_value.origin.x + diff.origin.x * amount,
                y: from_value.origin.y + diff.origin.y * amount,
            },
            size: CGSize {
                width: from_value.size.width + diff.size.width * amount,
                height: from_value.size.width + diff.size.width * amount,
            },
        }
    }
}
impl Interpolatable for CGColorHostObject {
    fn interpolate(
        env: &mut Environment,
        from_value: id,
        to_value: id,
        by_value: id,
        amount: f32,
    ) -> Self {
        assert_ne!(from_value, nil);
        assert_ne!(to_value, nil);
        assert_eq!(by_value, nil);
        let from_value = env.objc.borrow::<CGColorHostObject>(from_value);
        let to_value = env.objc.borrow::<CGColorHostObject>(to_value);
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

        let duration: CFTimeInterval = msg![env; animation duration];
        let progress = ((current_time - begin_time) / duration).clamp(0.0, 1.0) as f32;

        let timing_function: id = msg![env; animation timingFunction];
        let interpolation_amount: f32 = msg![env; timing_function _solveForInput: progress];

        // Assuming all animations here are CABasicAnimation
        // TODO: Handle other types of animations

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
                presentation.anchor_point =
                    CGPoint::interpolate(env, from_value, to_value, by_value, interpolation_amount)
            }
            "backgroundColor" => {
                presentation.background_color = Some(CGColorHostObject::interpolate(
                    env,
                    from_value,
                    to_value,
                    by_value,
                    interpolation_amount,
                ))
            }
            "bounds" => {
                presentation.bounds =
                    CGRect::interpolate(env, from_value, to_value, by_value, interpolation_amount)
            }
            "cornerRadius" => {
                presentation.corner_radius =
                    f32::interpolate(env, from_value, to_value, by_value, interpolation_amount)
            }
            "hidden" => {
                presentation.hidden =
                    bool::interpolate(env, from_value, to_value, by_value, interpolation_amount)
            }
            "opacity" => {
                presentation.opacity =
                    f32::interpolate(env, from_value, to_value, by_value, interpolation_amount)
            }
            "position" => {
                presentation.position =
                    CGPoint::interpolate(env, from_value, to_value, by_value, interpolation_amount)
            }
            _ => panic!("Attempted to animate on key {}", key_path),
        }

        if progress == 1.0f32 {
            // TODO: Call delegate to notify animation end
            // TODO: Skip removal if removedOnCompletion is false
            msg![env; layer removeAnimationForKey: key]
        }
    }

    presentation
}
