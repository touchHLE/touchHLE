/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CAAnimation` and its subclasses

use crate::dyld::{ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::frameworks::foundation::NSTimeInterval;
use crate::objc::{
    autorelease, id, msg, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};
use crate::{export_c_func, impl_HostObject_with_superclass, msg_super};
use crate::environment::Environment;
use crate::frameworks::core_foundation::cf_string::CFStringRef;
use crate::frameworks::core_graphics::CGFloat;
use crate::libc::mach_time::mach_absolute_time;
use crate::mem::MutPtr;

type CATransitionType = id; // NSString*
const kCATransitionFade: &str = "kCATransitionFade";
const kCATransitionMoveIn: &str = "kCATransitionMoveIn";
const kCATransitionPush: &str = "kCATransitionPush";
const kCATransitionReveal: &str = "kCATransitionReveal";

/// `CATransitionType` values.
pub const CONSTANTS: ConstantExports = &[
    (
        "_kCATransitionFade",
        HostConstant::NSString(kCATransitionFade),
    ),
    (
        "_kCATransitionMoveIn",
        HostConstant::NSString(kCATransitionMoveIn),
    ),
    (
        "_kCATransitionPush",
        HostConstant::NSString(kCATransitionPush),
    ),
    (
        "_kCATransitionReveal",
        HostConstant::NSString(kCATransitionReveal),
    ),
];

#[derive(Default)]
struct CAAnimationHostObject {
    delegate: id,        // CAAnimationDelegate*
    timing_function: id, // CAMediaTimingFunction*
    autoreverses: bool,
    repeat_count: f32,
    duration: CFTimeInterval,
    is_removed_on_completion: bool,
    fill_mode: CFStringRef,
    calculation_mode: CFStringRef,
}
impl HostObject for CAAnimationHostObject {}

#[derive(Default)]
struct CAPropertyAnimationHostObject {
    superclass: CAAnimationHostObject,
    key_path: id, // NSString*
}
impl_HostObject_with_superclass!(CAPropertyAnimationHostObject);

#[derive(Default)]
struct CABasicAnimationHostObject {
    superclass: CAPropertyAnimationHostObject,
    duration: NSTimeInterval,
    from_value: id,
    to_value: id,
}
impl_HostObject_with_superclass!(CABasicAnimationHostObject);

#[derive(Default)]
struct CAKeyframeAnimationHostObject {
    superclass: CAPropertyAnimationHostObject,
    duration: CGFloat,
}
impl_HostObject_with_superclass!(CAKeyframeAnimationHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// CAAnimation is an abstract class.
@implementation CAAnimation: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<CAAnimationHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())setDelegate:(id)delegate { // CAAnimationDelegate*
    log_dbg!("[(CAAnimation*){:?} setDelegate:{:?}]", this, delegate);
    env.objc.borrow_mut::<CAAnimationHostObject>(this).delegate = delegate;
    retain(env, delegate);
}

- (())setTimingFunction:(id)timingFunction { // CAMediaTimingFunction*
    log_dbg!("[(CAAnimation*){:?} setTimingFunction:{:?}]", this, timingFunction);
    env.objc.borrow_mut::<CAAnimationHostObject>(this).timing_function = timingFunction;
    retain(env, timingFunction);
}
- (id)timingFunction {
    env.objc.borrow::<CAAnimationHostObject>(this).timing_function
}

// CAMediaTiming protocol implementation
- (())setAutoreverses:(bool)autoreverses {
    log_dbg!("[(CAAnimation*){:?} setAutoreverses:{:?}]", this, autoreverses);
    env.objc.borrow_mut::<CAAnimationHostObject>(this).autoreverses = autoreverses;
}
- (bool)autoreverses {
    env.objc.borrow::<CAAnimationHostObject>(this).autoreverses
}

- (())setRepeatCount:(f32)repeatCount {
    log_dbg!("[(CAAnimation*){:?} setRepeatCount:{:?}]", this, repeatCount);
    env.objc.borrow_mut::<CAAnimationHostObject>(this).repeat_count = repeatCount;
}
- (f32)repeatCount {
    env.objc.borrow::<CAAnimationHostObject>(this).repeat_count
}

- (())setDuration:(CFTimeInterval)duration {
    log_dbg!("[(CAAnimation*){:?} setDuration:{:?}]", this, duration);
    env.objc.borrow_mut::<CAAnimationHostObject>(this).duration = duration;
}

- (())setRemovedOnCompletion:(bool)removed {
    env.objc.borrow_mut::<CAAnimationHostObject>(this).is_removed_on_completion = removed;
}
- (bool)isRemovedOnCompletion {
    env.objc.borrow::<CAAnimationHostObject>(this).is_removed_on_completion
}

- (())setFillMode:(CFStringRef)mode {
    env.objc.borrow_mut::<CAAnimationHostObject>(this).fill_mode = mode
}
- (CFStringRef)fillMode {
    env.objc.borrow::<CAAnimationHostObject>(this).fill_mode
}

- (())setCalculationMode:(CFStringRef)mode {
    env.objc.borrow_mut::<CAAnimationHostObject>(this).calculation_mode = mode
}
- (CFStringRef)calculationMode {
    env.objc.borrow::<CAAnimationHostObject>(this).calculation_mode
}

- (())setValues:(MutPtr<id>)mode {
    log!("Ignoring [(CAAnimation*){:?} setValues:{:?}]", this, mode);
}

- (())setKeyTimes:(MutPtr<id>)mode {
    log!("Ignoring [(CAAnimation*){:?} setKeyTimes:{:?}]", this, mode);
}

- (())dealloc {
    let &CAAnimationHostObject { delegate, timing_function, .. } = env.objc.borrow(this);
    if delegate != nil {
        release(env, delegate);
    }
    if timing_function != nil {
        release(env, timing_function);
    }

    env.objc.dealloc_object(this, &mut env.mem)
}

@end


@implementation CAPropertyAnimation: CAAnimation

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<CAPropertyAnimationHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)animationWithKeyPath:(id)path { // NSString*
    let object = msg![env; this new];
    log_dbg!("[CAPropertyAnimation animationWithKeyPath:{:?} ({:?})] -> {:?}", path, to_rust_string(env, path), object);
    () = msg![env; object setKeyPath:path];
    autorelease(env, object)
}

- (())setKeyPath:(id)path { // NSString*
    log_dbg!("[(CAPropertyAnimation*){:?} setKeyPath:{:?} ({:?})]", this, path, to_rust_string(env, path));
    let path_copy: id = msg![env; path copy];
    env.objc.borrow_mut::<CAPropertyAnimationHostObject>(this).key_path = path_copy;
}
- (id)keyPath {
    env.objc.borrow::<CAPropertyAnimationHostObject>(this).key_path
}

- (())dealloc {
    let &CAPropertyAnimationHostObject { key_path, .. } = env.objc.borrow(this);
    if key_path != nil {
        release(env, key_path);
    }

    msg_super![env; this dealloc]
}

@end


@implementation CABasicAnimation: CAPropertyAnimation

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<CABasicAnimationHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())setDuration:(NSTimeInterval)duration {
    log_dbg!("[(CABasicAnimation*){:?} setDuration:{:?}]", this, duration);
    env.objc.borrow_mut::<CABasicAnimationHostObject>(this).duration = duration;
}
- (NSTimeInterval)duration {
    env.objc.borrow::<CABasicAnimationHostObject>(this).duration
}

- (())setFromValue:(id)value {
    log_dbg!("[(CABasicAnimation*){:?} setFromValue:{:?}]", this, value);
    env.objc.borrow_mut::<CABasicAnimationHostObject>(this).from_value = value;
    retain(env, value);
}
- (id)fromValue {
    env.objc.borrow::<CABasicAnimationHostObject>(this).from_value
}

- (())setToValue:(id)value {
    log_dbg!("[(CABasicAnimation*){:?} setToValue:{:?}]", this, value);
    env.objc.borrow_mut::<CABasicAnimationHostObject>(this).to_value = value;
    retain(env, value);
}
- (id)toValue {
    env.objc.borrow::<CABasicAnimationHostObject>(this).to_value
}

- (())dealloc {
    let &CABasicAnimationHostObject { from_value, to_value, .. } = env.objc.borrow(this);
    if from_value != nil {
        release(env, from_value);
    }
    if to_value != nil {
        release(env, to_value);
    }

    msg_super![env; this dealloc]
}

@end


@implementation CAKeyframeAnimation : CAPropertyAnimation
+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<CAKeyframeAnimationHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (CGFloat) duration {
    env.objc.borrow::<CAKeyframeAnimationHostObject>(this).duration
}
- (()) setDuration:(CGFloat)duration {
    env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).duration = duration

}
@end


@implementation CATransition : CAAnimation

+ (id)animation {
    msg![env; this new]
}

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<CABasicAnimationHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())setType:(CATransitionType)transitionType {
    log!("TODO: [(CATransition*){:?} setType:{:?} ({:?})]", this, transitionType, to_rust_string(env, transitionType));
}

@end

};

fn CACurrentMediaTime(env: &mut Environment) -> CFTimeInterval {
    mach_absolute_time(env) as f64 / 1e9f64
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CACurrentMediaTime())
];