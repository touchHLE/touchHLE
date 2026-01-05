/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CAAnimation` and its subclasses

use crate::dyld::{ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::core_animation::ca_media_timing_function::kCAMediaTimingFunctionDefault;
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::foundation::ns_string::{get_static_str, to_rust_string};
use crate::objc::{
    autorelease, id, msg, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};
use crate::{export_c_func, Environment};
use crate::{impl_HostObject_with_superclass, msg_class, msg_super};
use crate::libc::mach::time::mach_absolute_time;

type CATransitionType = id; // NSString*
const kCATransitionFade: &str = "fade";
const kCATransitionMoveIn: &str = "moveIn";
const kCATransitionPush: &str = "push";
const kCATransitionReveal: &str = "reveal";

pub type CAAnimationCalculationModeType = id; // NSString*
pub const kCAAnimationLinear: &str = "kCAAnimationLinear";

pub type CAMediaTimingFillMode = id; // NSString*
pub const kCAFillModeBackwards: &str = "backwards";
pub const kCAFillModeBoth: &str = "both";
pub const kCAFillModeForwards: &str = "forwards";
pub const kCAFillModeRemoved: &str = "removed";

pub const CONSTANTS: ConstantExports = &[
    // `CATransitionType` values.
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
    // `CAAnimationCalculationMode` values.
    (
        "_kCAAnimationLinear",
        HostConstant::NSString(kCAAnimationLinear),
    ),
    // `CAMediaTimingFillMode` values.
    (
        "_kCAFillModeBackwards",
        HostConstant::NSString(kCAFillModeBackwards),
    ),
    ("_kCAFillModeBoth", HostConstant::NSString(kCAFillModeBoth)),
    (
        "_kCAFillModeForwards",
        HostConstant::NSString(kCAFillModeForwards),
    ),
    (
        "_kCAFillModeRemoved",
        HostConstant::NSString(kCAFillModeRemoved),
    ),
];

struct CAAnimationHostObject {
    removed_on_completion: bool,
    timing_function: id, // CAMediaTimingFunction*
    delegate: id,        // CAAnimationDelegate*
    autoreverses: bool,
    repeat_count: f32,
    begin_time: CFTimeInterval,
    duration: CFTimeInterval,
    fill_mode: &'static str,
    calculation_mode: &'static str,
    started_at: Option<CFTimeInterval>,
}
impl HostObject for CAAnimationHostObject {}
impl Default for CAAnimationHostObject {
    fn default() -> Self {
        Self {
            removed_on_completion: true,
            timing_function: Default::default(),
            delegate: Default::default(),
            autoreverses: Default::default(),
            repeat_count: Default::default(),
            begin_time: Default::default(),
            duration: Default::default(),
            fill_mode: kCAFillModeRemoved,
            started_at: None,
            calculation_mode: kCAAnimationLinear,
        }
    }
}

#[derive(Default)]
struct CAPropertyAnimationHostObject {
    superclass: CAAnimationHostObject,
    key_path: id, // NSString*
}
impl_HostObject_with_superclass!(CAPropertyAnimationHostObject);

#[derive(Default)]
struct CABasicAnimationHostObject {
    superclass: CAPropertyAnimationHostObject,
    from_value: id,
    to_value: id,
    by_value: id,
    animation_name: id,
}
impl_HostObject_with_superclass!(CABasicAnimationHostObject);

#[derive(Default)]
pub struct CAKeyframeAnimationHostObject {
    superclass: CAPropertyAnimationHostObject,
    duration: CFTimeInterval,
    keyTimes: id, // NSArray<NSNumber>
    values: id, // NSArray
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

+ (id)animation {
    let object = msg![env; this new];
    autorelease(env, object)
}

- (id)init {
    let default_timing_function_name: id = get_static_str(env, kCAMediaTimingFunctionDefault);
    let default_timing_function: id = msg_class![env; CAMediaTimingFunction functionWithName: default_timing_function_name];
    () = msg![env; this setTimingFunction: default_timing_function];
    this
}

- (())setRemovedOnCompletion:(bool)removed_on_completion {
    log_dbg!("[(CAAnimation*){:?} setRemovedOnCompletion:{:?}]", this, removed_on_completion);
    env.objc.borrow_mut::<CAAnimationHostObject>(this).removed_on_completion = removed_on_completion;
}
- (bool)isRemovedOnCompletion {
    env.objc.borrow::<CAAnimationHostObject>(this).removed_on_completion
}

- (())setDelegate:(id)delegate { // CAAnimationDelegate*
    log_dbg!("[(CAAnimation*){:?} setDelegate:{:?}]", this, delegate);
    env.objc.borrow_mut::<CAAnimationHostObject>(this).delegate = delegate;
    retain(env, delegate);
}
- (id)delegate {
    env.objc.borrow::<CAAnimationHostObject>(this).delegate
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

- (())setBeginTime:(CFTimeInterval)beginTime {
    log_dbg!("[(CAAnimation*){:?} setBeginTime:{:?}]", this, beginTime);
    env.objc.borrow_mut::<CAAnimationHostObject>(this).begin_time = beginTime;
}
- (CFTimeInterval)beginTime {
    env.objc.borrow::<CAAnimationHostObject>(this).begin_time
}

- (())setDuration:(CFTimeInterval)duration {
    log_dbg!("[(CAAnimation*){:?} setDuration:{:?}]", this, duration);
    env.objc.borrow_mut::<CAAnimationHostObject>(this).duration = duration;
}
- (CFTimeInterval)duration {
    env.objc.borrow::<CAAnimationHostObject>(this).duration
}

- (())setFillMode:(CAMediaTimingFillMode)fill_mode {
    let fill_mode_str = to_rust_string(env, fill_mode);
    log_dbg!("[(CAAnimation*){:?} setFillMode:{:?} ({})]", this, fill_mode, fill_mode_str);
    let fill_mode_str = match &*fill_mode_str {
        kCAFillModeBackwards => kCAFillModeBackwards,
        kCAFillModeBoth => kCAFillModeBoth,
        kCAFillModeForwards => kCAFillModeForwards ,
        kCAFillModeRemoved => kCAFillModeRemoved ,
        _ => panic!("Unknown fill mode \"{}\"", fill_mode_str)
    };
    env.objc.borrow_mut::<CAAnimationHostObject>(this).fill_mode = fill_mode_str;
}
- (CAMediaTimingFillMode)fillMode {
    let fill_mode = env.objc.borrow::<CAAnimationHostObject>(this).fill_mode;
    get_static_str(env, fill_mode)
}

- (())setCalculationMode:(CAAnimationCalculationModeType)mode {
    let calculation_mode_str = to_rust_string(env, mode);
    let calculation_mode_str = match &*calculation_mode_str {
        kCAAnimationLinear => kCAAnimationLinear,
        _ => panic!("Unknown calculation mode \"{}\"", calculation_mode_str)
    };
    env.objc.borrow_mut::<CAAnimationHostObject>(this).calculation_mode = calculation_mode_str;
}
- (CAAnimationCalculationModeType)calculationMode {
    let calculation_mode = env.objc.borrow::<CAAnimationHostObject>(this).calculation_mode;
    get_static_str(env, calculation_mode)
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

- (())setByValue:(id)value {
    log_dbg!("[(CABasicAnimation*){:?} setByValue:{:?}]", this, value);
    env.objc.borrow_mut::<CABasicAnimationHostObject>(this).by_value = value;
    retain(env, value);
}
- (id)byValue {
    env.objc.borrow::<CABasicAnimationHostObject>(this).by_value
}

// Cause of Death expects to be able to write to "animation-name"
// Currently we have no way to either declare an ivar or declare a setter with a non-identifier name
- (())setValue:(id)value
       forKey:(id)key { // NSString*
    let key_string = to_rust_string(env, key); // TODO: avoid copy?
    if key_string == "animation-name" {
        let value_string = to_rust_string(env, value);
        log!("setting \"animation-name\" to \"{}\" on CABasicAnimation({:?})!", value_string, this);
        env.objc.borrow_mut::<CABasicAnimationHostObject>(this).animation_name = value;
        return;
    }
    msg_super![env; this setValue:value forKey:key]
}

- (id) valueForKey:(id)key {
    let key_string = to_rust_string(env, key); // TODO: avoid copy?
    if key_string == "animation-name" {
        return env.objc.borrow::<CABasicAnimationHostObject>(this).animation_name;
    }
    msg_super![env; this valueForKey:key]
}

- (()) setBeginTime: (f32) beginTime {
    log!("Ignoring setBeginTime: {}", beginTime);
    // Apple docs does not mention beginTime in any
    // class of CABasicAnimation but Cause of Death calls it?
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

- (CFTimeInterval) duration {
    env.objc.borrow::<CAKeyframeAnimationHostObject>(this).duration
}
- (()) setDuration:(CFTimeInterval)duration {
    env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).duration = duration
}

- (())setValues:(id)values {
    env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).values = values;
}

- (())setKeyTimes:(id)keyTimes { // NSArray<NSNumber*>*
    env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).keyTimes = keyTimes;
}

- (id)values { // NSArray
    env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).values
}

- (id)keyTimes { // NSArray<NSNumber*>*
    env.objc.borrow_mut::<CAKeyframeAnimationHostObject>(this).keyTimes
}
@end


@implementation CATransition : CAAnimation

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<CABasicAnimationHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())setType:(CATransitionType)transitionType {
    log!("TODO: [(CATransition*){:?} setType:{:?} ({:?})]", this, transitionType, to_rust_string(env, transitionType));
}

@end

};

pub fn get_animation_start_time(
    env: &mut Environment,
    animation: id,
) -> &mut Option<CFTimeInterval> {
    &mut env
        .objc
        .borrow_mut::<CAAnimationHostObject>(animation)
        .started_at
}
