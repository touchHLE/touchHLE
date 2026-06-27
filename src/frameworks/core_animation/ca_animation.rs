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
use crate::Environment;
use crate::{impl_HostObject_with_superclass, msg_class, msg_super};

type CATransitionType = id; // NSString*
type CATransitionSubtype = id; // NSString*
pub const kCATransitionFade: &str = "fade";
pub const kCATransitionMoveIn: &str = "moveIn";
pub const kCATransitionPush: &str = "push";
pub const kCATransitionReveal: &str = "reveal";

// CATransitionSubtype values: directions for the transition animation.
pub const kCATransitionFromTop: &str = "fromTop";
pub const kCATransitionFromBottom: &str = "fromBottom";
pub const kCATransitionFromLeft: &str = "fromLeft";
pub const kCATransitionFromRight: &str = "fromRight";

pub type CAMediaTimingFillMode = id; // NSString*
pub const kCAFillModeBackwards: &str = "backwards";
pub const kCAFillModeBoth: &str = "both";
pub const kCAFillModeForwards: &str = "forwards";
pub const kCAFillModeRemoved: &str = "removed";

pub const kCAAnimationDiscrete: &str = "discrete";
pub const kCAAnimationLinear: &str = "linear";
pub const kCAAnimationPaced: &str = "paced";

pub const CONSTANTS: ConstantExports = &[
    // `kCATransition` — the animation key used when adding a CATransition to a
    // layer via `[CALayer addAnimation:forKey:]`. Equal to @"transition".
    ("_kCATransition", HostConstant::NSString("transition")),
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
    // `CATransitionSubtype` values.
    (
        "_kCATransitionFromTop",
        HostConstant::NSString(kCATransitionFromTop),
    ),
    (
        "_kCATransitionFromBottom",
        HostConstant::NSString(kCATransitionFromBottom),
    ),
    (
        "_kCATransitionFromLeft",
        HostConstant::NSString(kCATransitionFromLeft),
    ),
    (
        "_kCATransitionFromRight",
        HostConstant::NSString(kCATransitionFromRight),
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
    // `CAAnimation` calculation modes.
    (
        "_kCAAnimationDiscrete",
        HostConstant::NSString(kCAAnimationDiscrete),
    ),
    (
        "_kCAAnimationLinear",
        HostConstant::NSString(kCAAnimationLinear),
    ),
    (
        "_kCAAnimationPaced",
        HostConstant::NSString(kCAAnimationPaced),
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

#[derive(Default)]
struct CAAnimationGroupHostObject {
    superclass: CAAnimationHostObject,
    animations: id, // NSArray*
    key_path: id,
    from_value: id,
    to_value: id,
    by_value: id,
}
impl_HostObject_with_superclass!(CAAnimationGroupHostObject);

/// Host object for `CATransition`.
///
/// In Apple's class hierarchy `CATransition` is a direct subclass of
/// `CAAnimation` (not `CABasicAnimation`). We therefore inherit only from
/// `CAAnimationHostObject` and add the `CATransition`-specific properties
/// directly: `type`, `subtype`, `startProgress`, `endProgress`, `filter`.
///
/// Many real-world apps also call `setKeyPath:` / `setFromValue:` /
/// `setToValue:` / `setByValue:` on a `CATransition` (e.g. when configuring a
/// transition via key-value coding). Apple's runtime silently stores those
/// values via the generic KVC machinery, so we mirror that here by giving
/// `CATransition` its own backing storage for them.
struct CATransitionHostObject {
    superclass: CAAnimationHostObject,
    // `CATransition` properties.
    type_value: &'static str,
    subtype: Option<&'static str>,
    start_progress: f32,
    end_progress: f32,
    filter: id, // CIFilter*
    // KVC-style storage that mirrors `CABasicAnimation` / `CAPropertyAnimation`
    // so that apps that set these on a `CATransition` round-trip correctly.
    key_path: id, // NSString*
    from_value: id,
    to_value: id,
    by_value: id,
}
impl Default for CATransitionHostObject {
    fn default() -> Self {
        Self {
            superclass: CAAnimationHostObject::default(),
            // Apple defaults `type` to `kCATransitionFade`.
            type_value: kCATransitionFade,
            subtype: None,
            start_progress: 0.0,
            end_progress: 1.0,
            filter: nil,
            key_path: nil,
            from_value: nil,
            to_value: nil,
            by_value: nil,
        }
    }
}
impl_HostObject_with_superclass!(CATransitionHostObject);

/// Match an NSString against the set of valid `CATransitionType` values.
/// Returns the canonical static string for known types, or logs a warning and
/// defaults to `kCATransitionFade` for unknown values.
fn canonicalize_transition_type(env: &mut Environment, value: id) -> &'static str {
    if value == nil {
        return kCATransitionFade;
    }
    let s = to_rust_string(env, value);
    match &*s {
        kCATransitionFade => kCATransitionFade,
        kCATransitionMoveIn => kCATransitionMoveIn,
        kCATransitionPush => kCATransitionPush,
        kCATransitionReveal => kCATransitionReveal,
        other => {
            log!(
                "Warning: CATransition setType: unknown transition type {:?}; \
                 defaulting to kCATransitionFade.",
                other
            );
            kCATransitionFade
        }
    }
}

/// Match an NSString against the set of valid `CATransitionSubtype` values.
/// Returns `Some(...)` for a known subtype, `None` for nil (subtypes are
/// optional on Apple's CATransition), and logs a warning for unknown values.
fn canonicalize_transition_subtype(env: &mut Environment, value: id) -> Option<&'static str> {
    if value == nil {
        return None;
    }
    let s = to_rust_string(env, value);
    match &*s {
        kCATransitionFromTop => Some(kCATransitionFromTop),
        kCATransitionFromBottom => Some(kCATransitionFromBottom),
        kCATransitionFromLeft => Some(kCATransitionFromLeft),
        kCATransitionFromRight => Some(kCATransitionFromRight),
        other => {
            log!(
                "Warning: CATransition setSubtype: unknown subtype {:?}; storing as nil.",
                other
            );
            None
        }
    }
}

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
        other => {
            log!(
                "Warning: CAAnimation setFillMode: unknown fill mode {:?}; defaulting to kCAFillModeRemoved.",
                other
            );
            kCAFillModeRemoved
        }
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


@implementation CAAnimationGroup: CAAnimation

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<CAAnimationGroupHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())setAnimations:(id)animations { // NSArray*
    log_dbg!("[(CAAnimationGroup*){:?} setAnimations:{:?}]", this, animations);
    env.objc.borrow_mut::<CAAnimationGroupHostObject>(this).animations = animations;
    retain(env, animations);
}

- (id)animations {
    env.objc.borrow::<CAAnimationGroupHostObject>(this).animations
}


- (())setKeyPath:(id)path {
    let path_copy: id = if path == nil { nil } else { msg![env; path copy] };
    let old = env.objc.borrow::<CAAnimationGroupHostObject>(this).key_path;
    env.objc.borrow_mut::<CAAnimationGroupHostObject>(this).key_path = path_copy;
    if old != nil {
        release(env, old);
    }
}
- (id)keyPath { env.objc.borrow::<CAAnimationGroupHostObject>(this).key_path }

- (())setFromValue:(id)value {
    let old = env.objc.borrow::<CAAnimationGroupHostObject>(this).from_value;
    retain(env, value);
    env.objc.borrow_mut::<CAAnimationGroupHostObject>(this).from_value = value;
    if old != nil { release(env, old); }
}
- (id)fromValue { env.objc.borrow::<CAAnimationGroupHostObject>(this).from_value }

- (())setToValue:(id)value {
    let old = env.objc.borrow::<CAAnimationGroupHostObject>(this).to_value;
    retain(env, value);
    env.objc.borrow_mut::<CAAnimationGroupHostObject>(this).to_value = value;
    if old != nil { release(env, old); }
}
- (id)toValue { env.objc.borrow::<CAAnimationGroupHostObject>(this).to_value }

- (())setByValue:(id)value {
    let old = env.objc.borrow::<CAAnimationGroupHostObject>(this).by_value;
    retain(env, value);
    env.objc.borrow_mut::<CAAnimationGroupHostObject>(this).by_value = value;
    if old != nil { release(env, old); }
}
- (id)byValue { env.objc.borrow::<CAAnimationGroupHostObject>(this).by_value }

- (())dealloc {
    let &CAAnimationGroupHostObject {
        animations, key_path, from_value, to_value, by_value, ..
    } = env.objc.borrow(this);
    if animations != nil { release(env, animations); }
    if key_path != nil { release(env, key_path); }
    if from_value != nil { release(env, from_value); }
    if to_value != nil { release(env, to_value); }
    if by_value != nil { release(env, by_value); }

    msg_super![env; this dealloc]
}

@end


@implementation CATransition : CAAnimation

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<CATransitionHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)animation {
    let object: id = msg![env; this new];
    autorelease(env, object)
}

// CATransition-specific properties.

- (())setType:(CATransitionType)transition_type {
    let canonical = canonicalize_transition_type(env, transition_type);
    log_dbg!(
        "[(CATransition*){:?} setType:{:?} ({:?})]",
        this, transition_type, canonical
    );
    env.objc.borrow_mut::<CATransitionHostObject>(this).type_value = canonical;
}
- (CATransitionType)type {
    let type_value = env.objc.borrow::<CATransitionHostObject>(this).type_value;
    get_static_str(env, type_value)
}

- (())setSubtype:(CATransitionSubtype)subtype {
    let canonical = canonicalize_transition_subtype(env, subtype);
    log_dbg!(
        "[(CATransition*){:?} setSubtype:{:?} ({:?})]",
        this, subtype, canonical
    );
    env.objc.borrow_mut::<CATransitionHostObject>(this).subtype = canonical;
}
- (CATransitionSubtype)subtype {
    let subtype = env.objc.borrow::<CATransitionHostObject>(this).subtype;
    match subtype {
        Some(s) => get_static_str(env, s),
        None => nil,
    }
}

- (())setStartProgress:(f32)progress {
    let clamped = progress.clamp(0.0, 1.0);
    log_dbg!(
        "[(CATransition*){:?} setStartProgress:{:?}]",
        this, clamped
    );
    env.objc.borrow_mut::<CATransitionHostObject>(this).start_progress = clamped;
}
- (f32)startProgress {
    env.objc.borrow::<CATransitionHostObject>(this).start_progress
}

- (())setEndProgress:(f32)progress {
    let clamped = progress.clamp(0.0, 1.0);
    log_dbg!(
        "[(CATransition*){:?} setEndProgress:{:?}]",
        this, clamped
    );
    env.objc.borrow_mut::<CATransitionHostObject>(this).end_progress = clamped;
}
- (f32)endProgress {
    env.objc.borrow::<CATransitionHostObject>(this).end_progress
}

- (())setFilter:(id)filter { // CIFilter*
    log_dbg!("[(CATransition*){:?} setFilter:{:?}]", this, filter);
    let old = env.objc.borrow::<CATransitionHostObject>(this).filter;
    if filter != nil {
        retain(env, filter);
    }
    env.objc.borrow_mut::<CATransitionHostObject>(this).filter = filter;
    if old != nil {
        release(env, old);
    }
}
- (id)filter {
    env.objc.borrow::<CATransitionHostObject>(this).filter
}

// `CAPropertyAnimation`-style methods. `CATransition` does not actually
// inherit from `CAPropertyAnimation` on Apple's platforms, but many apps set
// these values through Key-Value Coding, so we provide explicit accessors.

- (())setKeyPath:(id)path { // NSString*
    log_dbg!(
        "[(CATransition*){:?} setKeyPath:{:?} ({:?})]",
        this, path, if path == nil { String::new() } else { to_rust_string(env, path).to_string() }
    );
    let path_copy: id = if path == nil { nil } else { msg![env; path copy] };
    let old = env.objc.borrow::<CATransitionHostObject>(this).key_path;
    env.objc.borrow_mut::<CATransitionHostObject>(this).key_path = path_copy;
    if old != nil {
        release(env, old);
    }
}
- (id)keyPath {
    env.objc.borrow::<CATransitionHostObject>(this).key_path
}

// `CABasicAnimation`-style methods (KVC compatibility, see comment above).

- (())setFromValue:(id)value {
    log_dbg!("[(CATransition*){:?} setFromValue:{:?}]", this, value);
    let old = env.objc.borrow::<CATransitionHostObject>(this).from_value;
    if value != nil {
        retain(env, value);
    }
    env.objc.borrow_mut::<CATransitionHostObject>(this).from_value = value;
    if old != nil {
        release(env, old);
    }
}
- (id)fromValue {
    env.objc.borrow::<CATransitionHostObject>(this).from_value
}

- (())setToValue:(id)value {
    log_dbg!("[(CATransition*){:?} setToValue:{:?}]", this, value);
    let old = env.objc.borrow::<CATransitionHostObject>(this).to_value;
    if value != nil {
        retain(env, value);
    }
    env.objc.borrow_mut::<CATransitionHostObject>(this).to_value = value;
    if old != nil {
        release(env, old);
    }
}
- (id)toValue {
    env.objc.borrow::<CATransitionHostObject>(this).to_value
}

- (())setByValue:(id)value {
    log_dbg!("[(CATransition*){:?} setByValue:{:?}]", this, value);
    let old = env.objc.borrow::<CATransitionHostObject>(this).by_value;
    if value != nil {
        retain(env, value);
    }
    env.objc.borrow_mut::<CATransitionHostObject>(this).by_value = value;
    if old != nil {
        release(env, old);
    }
}
- (id)byValue {
    env.objc.borrow::<CATransitionHostObject>(this).by_value
}

- (())dealloc {
    let &CATransitionHostObject {
        filter, key_path, from_value, to_value, by_value, ..
    } = env.objc.borrow(this);
    if filter != nil {
        release(env, filter);
    }
    if key_path != nil {
        release(env, key_path);
    }
    if from_value != nil {
        release(env, from_value);
    }
    if to_value != nil {
        release(env, to_value);
    }
    if by_value != nil {
        release(env, by_value);
    }
    msg_super![env; this dealloc]
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
