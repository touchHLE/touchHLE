/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CAAnimation` and its subclasses

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_animation::ca_media_timing_function::kCAMediaTimingFunctionDefault;
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::foundation::ns_string::{get_static_str, to_rust_string};
use crate::objc::{
    autorelease, id, msg, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};
use crate::{impl_HostObject_with_superclass, msg_class, msg_super};

type CATransitionType = id; // NSString*
const kCATransitionFade: &str = "kCATransitionFade";
const kCATransitionMoveIn: &str = "kCATransitionMoveIn";
const kCATransitionPush: &str = "kCATransitionPush";
const kCATransitionReveal: &str = "kCATransitionReveal";

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
    // `CAMediaTimingFillMode` values.
    (
        "_kCAFillModeBackwards",
        HostConstant::NSString(kCAFillModeBackwards)
    ),
    (
        "_kCAFillModeBoth",
        HostConstant::NSString(kCAFillModeBoth)
    ),
    (
        "_kCAFillModeForwards",
        HostConstant::NSString(kCAFillModeForwards)
    ),
    (
        "_kCAFillModeRemoved",
        HostConstant::NSString(kCAFillModeRemoved)
    ),
];

struct CAAnimationHostObject {
    delegate: id,        // CAAnimationDelegate*
    timing_function: id, // CAMediaTimingFunction*
    autoreverses: bool,
    repeat_count: f32,
    begin_time: CFTimeInterval,
    duration: CFTimeInterval,
    fill_mode: &'static str,
}
impl HostObject for CAAnimationHostObject {}
impl Default for CAAnimationHostObject {
    fn default() -> Self {
        Self {
            delegate: Default::default(),
            timing_function: Default::default(),
            autoreverses: Default::default(),
            repeat_count: Default::default(),
            begin_time: Default::default(),
            duration: Default::default(),
            fill_mode: kCAFillModeRemoved,
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
}
impl_HostObject_with_superclass!(CABasicAnimationHostObject);

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
