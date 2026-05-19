/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIView`.
//!
//! Useful resources:
//! - Apple's [View Programming Guide for iOS](https://developer.apple.com/library/archive/documentation/WindowsViews/Conceptual/ViewPG_iPhoneOS/Introduction/Introduction.html)

pub mod ui_alert_view;
pub mod ui_control;
pub mod ui_image_view;
pub mod ui_label;
pub mod ui_picker_view;
pub mod ui_scroll_view;
pub mod ui_web_view;
pub mod ui_window;

use core::panic;

use super::ui_graphics::{UIGraphicsPopContext, UIGraphicsPushContext};
use crate::frameworks::core_animation::ca_animation::{
    kCAFillModeBackwards, CAMediaTimingFillMode,
};
use crate::frameworks::core_animation::ca_media_timing_function::{
    kCAMediaTimingFunctionEaseIn, kCAMediaTimingFunctionEaseInEaseOut,
    kCAMediaTimingFunctionEaseOut, kCAMediaTimingFunctionLinear,
};
use crate::frameworks::core_animation::ca_transaction;
use crate::frameworks::core_animation::CACurrentMediaTime;
use crate::frameworks::core_foundation::time::CFTimeInterval;
use crate::frameworks::core_graphics::cg_affine_transform::CGAffineTransform;
use crate::frameworks::core_graphics::cg_color::CGColorRef;
use crate::frameworks::core_graphics::cg_context::{CGContextClearRect, CGContextRef};
use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::{from_rust_string, get_static_str, to_rust_string};
use crate::frameworks::foundation::{ns_array, NSInteger, NSTimeInterval, NSUInteger};
use crate::mem::{ConstVoidPtr, GuestUSize};
use crate::objc::{
    autorelease, id, msg, msg_class, msg_send, nil, objc_classes, release, retain,
    todo_objc_setter, Class, ClassExports, HostObject, NSZonePtr, ObjC, SEL,
};
use crate::Environment;

// Internal keys used to store UIView animation parameters in the wrapped
// CATransaction created by beginAnimations, set by the various setAnimation*
// methods, and later read by commitAnimations.
const touchHLE_kCATransactionAnimationId: &str = "_touchHLE_kCATransactionAnimationId";
const touchHLE_kCATransactionAnimationContext: &str = "_touchHLE_kCATransactionAnimationContext";
const touchHLE_kCATransactionAnimationDelay: &str = "_touchHLE_kCATransactionAnimationDelay";
const touchHLE_kCATransactionAnimationRepeatCount: &str =
    "_touchHLE_kCATransactionAnimationRepeatCount";
const touchHLE_kCATransactionAnimationRepeatAutoreverses: &str =
    "_touchHLE_kCATransactionAnimationRepeatAutoreverses";
const touchHLE_kCATransactionAnimationDelegate: &str = "_touchHLE_kCATransactionAnimationDelegate";
const touchHLE_kCATransactionAnimationWillStartSelector: &str =
    "_touchHLE_kCATransactionAnimationWillStartSelector";
const touchHLE_kCATransactionAnimationDidStopSelector: &str =
    "_touchHLE_kCATransactionAnimationDidStopSelector";

type UIViewAnimationCurve = NSInteger;
const UIViewAnimationCurveEaseInOut: UIViewAnimationCurve = 0;
const UIViewAnimationCurveEaseIn: UIViewAnimationCurve = 1;
const UIViewAnimationCurveEaseOut: UIViewAnimationCurve = 2;
const UIViewAnimationCurveLinear: UIViewAnimationCurve = 3;

#[derive(Default)]
pub struct State {
    /// List of views for internal purposes. Non-retaining!
    pub(super) views: Vec<id>,
    pub ui_window: ui_window::State,
    pub animation_block_count: usize,
}

pub(super) struct UIViewHostObject {
    /// CALayer or subclass.
    layer: id,
    /// Subviews in back-to-front order. These are strong references.
    subviews: Vec<id>,
    /// The superview. This is a weak reference.
    superview: id,
    /// The view controller that controls this view. This is a weak reference
    view_controller: id,
    tag: NSInteger,
    clears_context_before_drawing: bool,
    user_interaction_enabled: bool,
    multiple_touch_enabled: bool,
}
impl HostObject for UIViewHostObject {}
impl Default for UIViewHostObject {
    fn default() -> UIViewHostObject {
        // The Default trait is implemented so subclasses will get the same
        // defaults.
        UIViewHostObject {
            layer: nil,
            subviews: Vec::new(),
            superview: nil,
            view_controller: nil,
            tag: 0,
            clears_context_before_drawing: true,
            user_interaction_enabled: true,
            multiple_touch_enabled: false,
        }
    }
}

#[derive(Default)]
struct UIViewAnimationDelegateHostObject {
    animation_id: id, // NSString*
    context: ConstVoidPtr,
    delegate: id,
    will_start_selector: Option<SEL>,
    did_stop_selector: Option<SEL>,
    total_animation_count: u32,
    started_animation_count: u32,
    finished_animation_count: u32,
}
impl HostObject for UIViewAnimationDelegateHostObject {}

pub fn set_view_controller(env: &mut Environment, view: id, controller: id) {
    let host_obj = env.objc.borrow_mut::<UIViewHostObject>(view);
    host_obj.view_controller = controller;
}

/// Shared parts of `initWithCoder:` and `initWithFrame:`. These can't call
/// `init`: the subclass may have overridden `init` and will not expect to be
/// called here.
///
/// Do not call this in subclasses of `UIView`.
fn init_common(env: &mut Environment, this: id) -> id {
    let view_class: Class = msg![env; this class];
    let layer_class: Class = msg![env; view_class layerClass];
    let layer: id = msg![env; layer_class layer];

    // CALayer is not opaque by default, but UIView is
    () = msg![env; layer setDelegate:this];
    () = msg![env; layer setOpaque:true];

    env.objc.borrow_mut::<UIViewHostObject>(this).layer = layer;

    env.framework_state.uikit.ui_view.views.push(this);

    this
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIView: UIResponder

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIViewHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (Class)layerClass {
    env.objc.get_known_class("CALayer", &mut env.mem)
}

+ (())setAnimationDuration:(NSTimeInterval)duration {
    log_dbg!("[UIView setAnimationDuration:{:?}]", duration);
    () = msg_class![env; CATransaction setAnimationDuration:duration];
}

+ (())setAnimationDelay:(NSTimeInterval)delay {
    log_dbg!("[UIView setAnimationDelay:{:?}]", delay);
    let value: id = msg_class![env; NSNumber numberWithDouble:delay];
    () = msg_class![env; CATransaction setValue:value forKey:(get_static_str(env, touchHLE_kCATransactionAnimationDelay))];
}

+ (())setAnimationCurve:(UIViewAnimationCurve)curve {
    log_dbg!("[UIView setAnimationCurve:{:?}]", curve);
    let timing_function: id = match curve {
        UIViewAnimationCurveEaseInOut => {
            msg_class![env; CAMediaTimingFunction functionWithName:
                (get_static_str(env, kCAMediaTimingFunctionEaseInEaseOut))]
        },
        UIViewAnimationCurveEaseIn => {
            msg_class![env; CAMediaTimingFunction functionWithName:
                (get_static_str(env, kCAMediaTimingFunctionEaseIn))]
        },
        UIViewAnimationCurveEaseOut => {
            msg_class![env; CAMediaTimingFunction functionWithName:
                (get_static_str(env, kCAMediaTimingFunctionEaseOut))]
        },
        UIViewAnimationCurveLinear => {
            msg_class![env; CAMediaTimingFunction functionWithName:
                (get_static_str(env, kCAMediaTimingFunctionLinear))]
        },
        _ => panic!("Unknown UIViewAnimationCurve {:?}", curve),
    };
    () = msg_class![env; CATransaction setAnimationTimingFunction:timing_function];
}

+ (())setAnimationRepeatAutoreverses:(bool)repeat_autoreverses {
    log_dbg!("[UIView setAnimationRepeatAutoreverses:{:?}]", repeat_autoreverses);
    let value: id = msg_class![env; NSNumber numberWithBool:repeat_autoreverses];
    () = msg_class![env; CATransaction setValue:value forKey:(get_static_str(env, touchHLE_kCATransactionAnimationRepeatAutoreverses))];
}

+ (())setAnimationRepeatCount:(f32)repeat_count {
    log_dbg!("[UIView setAnimationRepeatCount:{:?}]", repeat_count);
    assert!(repeat_count >= 0.0);
    let value: id = msg_class![env; NSNumber numberWithFloat:repeat_count];
    () = msg_class![env; CATransaction setValue:value forKey:(get_static_str(env, touchHLE_kCATransactionAnimationRepeatCount))];
}

+ (())setAnimationDelegate:(id)delegate {
    log_dbg!("[UIView setAnimationDelegate:{:?}]", delegate);
    retain(env, delegate);
    () = msg_class![env; CATransaction setValue:delegate forKey:(get_static_str(env, touchHLE_kCATransactionAnimationDelegate))];
}

+ (())setAnimationWillStartSelector:(SEL)selector {
    let selector_str = selector.as_str(&env.mem);
    log_dbg!("[UIView setAnimationWillStartSelector:{:?} ({})]", selector, selector_str);
    let selector_nsstring = from_rust_string(env, selector_str.to_string());
    () = msg_class![env; CATransaction setValue:selector_nsstring forKey:(get_static_str(env, touchHLE_kCATransactionAnimationWillStartSelector))];
}

+ (())setAnimationDidStopSelector:(SEL)selector {
    let selector_str = selector.as_str(&env.mem);
    log_dbg!("[UIView setAnimationDidStopSelector:{:?} ({})]", selector, selector_str);
    let selector_nsstring = from_rust_string(env, selector_str.to_string());
    () = msg_class![env; CATransaction setValue:selector_nsstring forKey:(get_static_str(env, touchHLE_kCATransactionAnimationDidStopSelector))];
}

+ (())beginAnimations:(id)animation_id // NSString*
              context:(ConstVoidPtr)context {
    log_dbg!("[UIView beginAnimations:{:?} context:{:?}]", animation_id, context);
    () = msg_class![env; CATransaction begin];
    () = msg_class![env; CATransaction setValue:animation_id forKey:(get_static_str(env, touchHLE_kCATransactionAnimationId))];
    if !context.is_null() {
        let context: id = msg_class![env; NSNumber numberWithUnsignedInt:(context.to_bits())];
        () = msg_class![env; CATransaction setValue:context forKey:(get_static_str(env, touchHLE_kCATransactionAnimationContext))];
    }
    // Default values
    () = msg_class![env; UIView setAnimationDuration:0.2];
    () = msg_class![env; UIView setAnimationCurve:UIViewAnimationCurveEaseInOut];

    env.framework_state.uikit.ui_view.animation_block_count += 1;
}

+ (())commitAnimations {
    log_dbg!("[UIView commitAnimations]");

    // TODO: What if there's interleaved UIView animations and CATransactions?
    let animations = ca_transaction::ThreadLocalState::get_current_transaction(env).unwrap().get_animations();

    let delegate: id = msg_class![env; CATransaction valueForKey:(get_static_str(env, touchHLE_kCATransactionAnimationDelegate))];
    if animations.is_empty() && delegate == nil {
        log_dbg!("[UIView commitAnimations] with no animations and no delegate, skipping");
    } else {
        // Even if the animation block is committed with no animations,
        // we still proceed so the delegate gets called
        let animation_delegate = if delegate == nil {
            nil
        } else {
            let animation_delegate = msg_class![env; _touchHLE_UIView_AnimationDelegate new];
            () = msg![env; animation_delegate setDelegate:delegate];
            let animation_id: id = msg_class![env; CATransaction valueForKey:(get_static_str(env, touchHLE_kCATransactionAnimationId))];
            () = msg![env; animation_delegate setAnimationId:animation_id];
            let context: id = msg_class![env; CATransaction valueForKey:(get_static_str(env, touchHLE_kCATransactionAnimationContext))];
            if context != nil {
                let context: u32 = msg![env; context unsignedIntValue];
                let context: ConstVoidPtr = ConstVoidPtr::from_bits(context as GuestUSize);
                () = msg![env; animation_delegate setContext:context];
            }
            let will_start_selector: id = msg_class![env; CATransaction valueForKey:(get_static_str(env, touchHLE_kCATransactionAnimationWillStartSelector))];
            if will_start_selector != nil {
                let will_start_selector = to_rust_string(env, will_start_selector);
                let will_start_selector = env.objc.lookup_selector(&will_start_selector).unwrap();
                () = msg![env; animation_delegate setWillStartSelector:will_start_selector];
            }
            let did_stop_selector: id = msg_class![env; CATransaction valueForKey:(get_static_str(env, touchHLE_kCATransactionAnimationDidStopSelector))];
            if did_stop_selector != nil {
                let did_stop_selector = to_rust_string(env, did_stop_selector);
                let did_stop_selector = env.objc.lookup_selector(&did_stop_selector).unwrap();
                () = msg![env; animation_delegate setDidStopSelector:did_stop_selector];
            }
            let total_animation_count = animations.len() as u32;
            () = msg![env; animation_delegate setTotalAnimationCount:total_animation_count];
            animation_delegate
        };
        let delay: id = msg_class![env; CATransaction valueForKey:(get_static_str(env, touchHLE_kCATransactionAnimationDelay))];
        let repeat_count: id = msg_class![env; CATransaction valueForKey:(get_static_str(env, touchHLE_kCATransactionAnimationRepeatCount))];
        let repeat_autoreverses: id = msg_class![env; CATransaction valueForKey:(get_static_str(env, touchHLE_kCATransactionAnimationRepeatAutoreverses))];
        for (layer, animation) in animations {
            log_dbg!("[UIView commitAnimations] adding animation {:?} to layer {:?}", animation, layer);
            () = msg![env; animation setDelegate:animation_delegate];
            if delay != nil {
                let delay: f32 = msg![env; delay floatValue];
                let begin_time: CFTimeInterval = CACurrentMediaTime(env) + delay as f64;
                () = msg![env; animation setBeginTime:begin_time];
                let fill_mode: CAMediaTimingFillMode = get_static_str(env, kCAFillModeBackwards);
                () = msg![env; animation setFillMode:fill_mode];
            }
            if repeat_count != nil {
                let repeat_count: f32 = msg![env; repeat_count floatValue];
                () = msg![env; animation setRepeatCount:repeat_count];
            }
            if repeat_autoreverses != nil {
                let repeat_autoreverses: bool = msg![env; repeat_autoreverses boolValue];
                () = msg![env; animation setAutoreverses:repeat_autoreverses];
            }
        }
    }

    () = msg_class![env; CATransaction commit];

    env.framework_state.uikit.ui_view.animation_block_count -= 1;
}

// TODO: accessors etc

// initWithCoder: and initWithFrame: are basically UIView's designated
// initializers. init is not, it's a shortcut for the latter.
// Subclasses need to override both.

- (id)init {
    msg![env; this initWithFrame:(<CGRect as Default>::default())]
}

- (id)initWithFrame:(CGRect)frame {
    let this = init_common(env, this);

    () = msg![env; this setFrame:frame];

    log_dbg!(
        "[(UIView*){:?} initWithFrame:{:?}] => bounds {:?}, center {:?}",
        this,
        frame,
        { let bounds: CGRect = msg![env; this bounds]; bounds },
        { let center: CGPoint = msg![env; this center]; center },
    );

    this
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    let this = init_common(env, this);

    // TODO: decode the various other UIView properties

    let key_ns_string = get_static_str(env, "UIBounds");
    let bounds: CGRect = msg![env; coder decodeCGRectForKey:key_ns_string];

    let key_ns_string = get_static_str(env, "UICenter");
    let center: CGPoint = msg![env; coder decodeCGPointForKey:key_ns_string];

    let key_ns_string = get_static_str(env, "UIHidden");
    let hidden: bool = msg![env; coder decodeBoolForKey:key_ns_string];

    let key_ns_string = get_static_str(env, "UIOpaque");
    let opaque: bool = msg![env; coder decodeBoolForKey:key_ns_string];

    let key_ns_string = get_static_str(env, "UIBackgroundColor");
    let bg_color: id = msg![env; coder decodeObjectForKey:key_ns_string];

    let key_ns_string = get_static_str(env, "UITag");
    let tag: NSInteger = msg![env; coder decodeIntegerForKey:key_ns_string];

    let key_ns_string = get_static_str(env, "UIMultipleTouchEnabled");
    let multi_touch_enabled: bool = msg![env; coder decodeBoolForKey:key_ns_string];

    let key_ns_string = get_static_str(env, "UISubviews");
    let subviews: id = msg![env; coder decodeObjectForKey:key_ns_string];
    let subview_count: NSUInteger = msg![env; subviews count];

    log_dbg!(
        "[(UIView*){:?} initWithCoder:{:?}] => bounds {}, center {}, hidden {}, bg color {:?}, tag {}, opaque {}, multi touch enabled {}, {} subviews",
        this,
        coder,
        bounds,
        center,
        hidden,
        bg_color,
        tag,
        opaque,
        multi_touch_enabled,
        subview_count,
    );

    () = msg![env; this setBounds:bounds];
    () = msg![env; this setCenter:center];
    () = msg![env; this setHidden:hidden];
    () = msg![env; this setOpaque:opaque];
    () = msg![env; this setBackgroundColor:bg_color];
    () = msg![env; this setTag:tag];
    () = msg![env; this setMultipleTouchEnabled:multi_touch_enabled];

    for i in 0..subview_count {
        let subview: id = msg![env; subviews objectAtIndex:i];
        () = msg![env; this addSubview:subview];
    }

    this
}

- (NSInteger)tag {
    env.objc.borrow::<UIViewHostObject>(this).tag
}
- (())setTag:(NSInteger)tag {
    env.objc.borrow_mut::<UIViewHostObject>(this).tag = tag;
}

- (id)viewWithTag:(NSInteger)tag {
    let &UIViewHostObject {
        ref subviews,
        tag: view_tag,
        ..
    } = env.objc.borrow(this);
    if view_tag == tag {
        return this;
    }
    for view in subviews {
        if env.objc.borrow::<UIViewHostObject>(*view).tag == tag {
            return *view;
        }
    }
    nil
}

- (bool)isUserInteractionEnabled {
    env.objc.borrow::<UIViewHostObject>(this).user_interaction_enabled
}
- (())setUserInteractionEnabled:(bool)enabled {
    env.objc.borrow_mut::<UIViewHostObject>(this).user_interaction_enabled = enabled;
}

- (bool)isMultipleTouchEnabled {
    env.objc.borrow::<UIViewHostObject>(this).multiple_touch_enabled
}
- (())setMultipleTouchEnabled:(bool)enabled {
    env.objc.borrow_mut::<UIViewHostObject>(this).multiple_touch_enabled = enabled;
}

- (())setExclusiveTouch:(bool)exclusive {
    log!("TODO: ignoring setExclusiveTouch:{} for view {:?}", exclusive, this);
}

- (())layoutSubviews {
    // On iOS 5.1 and earlier, the default implementation of this method does
    // nothing.
}

- (id)superview {
    env.objc.borrow::<UIViewHostObject>(this).superview
}

- (id)window {
    // Looks up window in the superview hierarchy
    // TODO: cache the result somehow?
    let mut window: id = env.objc.borrow::<UIViewHostObject>(this).superview;
    let window_class = env.objc.get_known_class("UIWindow", &mut env.mem);
    while window != nil {
        let current_class: Class = msg![env; window class];
        log_dbg!("maybe window {:?} curr class {}", window, env.objc.get_class_name(current_class));
        if env.objc.class_is_subclass_of(current_class, window_class) {
            break;
        }
        window = env.objc.borrow::<UIViewHostObject>(window).superview;
    }
    log_dbg!("view {:?} has window {:?}", this, window);
    window
}

- (id)subviews {
    let views = env.objc.borrow::<UIViewHostObject>(this).subviews.clone();
    for view in &views {
        retain(env, *view);
    }
    let subs = ns_array::from_vec(env, views);
    autorelease(env, subs)
}

- (())addSubview:(id)view {
    log_dbg!("[(UIView*){:?} addSubview:{:?}] => ()", this, view);

    if view == nil {
        log_dbg!("Tolerating [(UIView*){:?} addSubview:nil]", this);
        return;
    }

    if env.objc.borrow::<UIViewHostObject>(view).superview == this {
        () = msg![env; this bringSubviewToFront:view];
    } else {
        retain(env, view);
        () = msg![env; view removeFromSuperview];
        let subview_obj = env.objc.borrow_mut::<UIViewHostObject>(view);
        subview_obj.superview = this;
        let subview_layer = subview_obj.layer;
        let this_obj = env.objc.borrow_mut::<UIViewHostObject>(this);
        this_obj.subviews.push(view);
        let this_layer = this_obj.layer;
        () = msg![env; this_layer addSublayer:subview_layer];
    }
}

- (())insertSubview:(id)view atIndex:(NSInteger)index {
    assert!(view != nil);
    retain(env, view);
    () = msg![env; view removeFromSuperview];

    let subview_obj = env.objc.borrow_mut::<UIViewHostObject>(view);
    subview_obj.superview = this;
    let subview_layer = subview_obj.layer;

    let &mut UIViewHostObject {
        ref mut subviews,
        layer: this_layer,
        ..
    } = env.objc.borrow_mut(this);

    subviews.insert(index as usize, view);

    assert!(index >= 0);
    () = msg![env; this_layer insertSublayer:subview_layer atIndex:(index as u32)];
}

- (())insertSubview:(id)view belowSubview:(id)sibling {
    retain(env, view);
    () = msg![env; view removeFromSuperview];

    let subview_obj = env.objc.borrow_mut::<UIViewHostObject>(view);
    subview_obj.superview = this;
    let subview_layer = subview_obj.layer;

    let sibling_layer = env.objc.borrow_mut::<UIViewHostObject>(sibling).layer;

    let &mut UIViewHostObject {
        ref mut subviews,
        layer: this_layer,
        ..
    } = env.objc.borrow_mut(this);

    let idx = subviews.iter().position(|&subview2| subview2 == sibling).unwrap();
    subviews.insert(idx, view);

    () = msg![env; this_layer insertSublayer:subview_layer below:sibling_layer];
}

- (())bringSubviewToFront:(id)subview {
    if subview == nil {
        // This happens in Touch & Go LITE. It's probably due to the ad classes
        // being replaced with fakes.
        log_dbg!("Tolerating [{:?} bringSubviewToFront:nil]", this);
        return;
    }

    let &mut UIViewHostObject {
        ref mut subviews,
        layer,
        ..
    } = env.objc.borrow_mut(this);

    let Some(idx) = subviews.iter().position(|&subview2| subview2 == subview) else {
        log_dbg!("Warning: Unable to find the subview {:?} in subviews of {:?}", subview, this);
        return;
    };
    let subview2 = subviews.remove(idx);
    assert!(subview2 == subview);
    subviews.push(subview);

    let subview_layer = env.objc.borrow::<UIViewHostObject>(subview).layer;
    () = msg![env; subview_layer removeFromSuperlayer];
    () = msg![env; layer addSublayer:subview_layer];
}

- (())sendSubviewToBack:(id)subview {
    if subview == nil {
        log_dbg!("Tolerating [{:?} sendSubviewToBack:nil]", this);
        return;
    }

    let &mut UIViewHostObject {
        ref mut subviews,
        layer,
        ..
    } = env.objc.borrow_mut(this);

    let Some(idx) = subviews.iter().position(|&subview2| subview2 == subview) else {
        log_dbg!("Warning: Unable to find the subview {:?} in subviews of {:?}", subview, this);
        return;
    };
    let subview2 = subviews.remove(idx);
    assert!(subview2 == subview);
    subviews.insert(0, subview);

    let subview_layer = env.objc.borrow::<UIViewHostObject>(subview).layer;
    () = msg![env; subview_layer removeFromSuperlayer];
    () = msg![env; layer insertSublayer:subview_layer atIndex:0u32];
}

- (())removeFromSuperview {
    let &mut UIViewHostObject {
        ref mut superview,
        layer: this_layer,
        ..
    } = env.objc.borrow_mut(this);
    let superview = std::mem::take(superview);
    if superview == nil {
        return;
    }
    () = msg![env; this_layer removeFromSuperlayer];

    let UIViewHostObject { ref mut subviews, .. } = env.objc.borrow_mut(superview);
    let idx = subviews.iter().position(|&subview| subview == this).unwrap();
    let subview = subviews.remove(idx);
    assert!(subview == this);
    release(env, this);
}

- (())dealloc {
    let UIViewHostObject {
        layer,
        superview,
        subviews,
        view_controller,
        tag: _,
        clears_context_before_drawing: _,
        user_interaction_enabled: _,
        multiple_touch_enabled: _,
    } = std::mem::take(env.objc.borrow_mut(this));

    release(env, layer);
    assert!(view_controller == nil);
    assert!(superview == nil);
    for subview in subviews {
        env.objc.borrow_mut::<UIViewHostObject>(subview).superview = nil;
        release(env, subview);
    }

    let state = &mut env.framework_state.uikit.ui_view.views;
    state.swap_remove(
        state.iter().position(|&v| v == this).unwrap()
    );

    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)layer {
    env.objc.borrow_mut::<UIViewHostObject>(this).layer
}

- (bool)isHidden {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer isHidden]
}
- (())setHidden:(bool)hidden {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer setHidden:hidden]
}

- (())setClipsToBounds:(bool)clips {
    todo_objc_setter!(this, clips);
}

- (bool)isOpaque {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer isOpaque]
}
- (())setOpaque:(bool)opaque {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer setOpaque:opaque]
}

- (CGFloat)alpha {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer opacity]
}
- (())setAlpha:(CGFloat)alpha {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer setOpacity:alpha]
}

- (id)backgroundColor {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    let cg_color: CGColorRef = msg![env; layer backgroundColor];
    msg_class![env; UIColor colorWithCGColor:cg_color]
}
- (())setBackgroundColor:(id)color { // UIColor*
    let color: CGColorRef = msg![env; color CGColor];
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer setBackgroundColor:color]
}

// TODO: support setNeedsDisplayInRect:
- (())setNeedsDisplay {
    // UIView has a method called drawRect: that subclasses override if they
    // need custom drawing. touchHLE's UIView (a CALayerDelegate) provides
    // an implementation of drawLayer:inContext: that calls drawRect:.
    // This maintains a clean separation of UIView and CALayer.
    //
    // To avoid wasting space and time on unnecessary bitmaps and drawing,
    // let's optimize here by only marking the layer as needing display if
    // the UIView's subclass overrides drawRect: or drawLayer:inContext:.
    let this_class = ObjC::read_isa(this, &env.mem);

    let ui_view_class = env.objc.get_known_class("UIView", &mut env.mem);

    let draw_layer_sel = env.objc.lookup_selector("drawLayer:inContext:").unwrap();
    let draw_rect_sel = env.objc.lookup_selector("drawRect:").unwrap();

    if env
        .objc
        .class_overrides_method_of_superclass(this_class, draw_rect_sel, ui_view_class)
        || env
            .objc
            .class_overrides_method_of_superclass(this_class, draw_layer_sel, ui_view_class)
    {
        let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
        msg![env; layer setNeedsDisplay]
    }
}

- (CGRect)bounds {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer bounds]
}
- (())setBounds:(CGRect)bounds {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer setBounds:bounds]
}
- (CGPoint)center {
    // FIXME: what happens if [layer anchorPoint] isn't (0.5, 0.5)?
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer position]
}
- (())setCenter:(CGPoint)center {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer setPosition:center]
}
- (CGRect)frame {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer frame]
}
- (())setFrame:(CGRect)frame {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer setFrame:frame]
}
- (CGAffineTransform)transform {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer affineTransform]
}
- (())setTransform:(CGAffineTransform)transform {
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer setAffineTransform:transform]
}

- (())setContentMode:(NSInteger)content_mode { // should be UIViewContentMode
    todo_objc_setter!(this, content_mode);
}

- (bool)clearsContextBeforeDrawing {
    env.objc.borrow::<UIViewHostObject>(this).clears_context_before_drawing
}
- (())setClearsContextBeforeDrawing:(bool)v {
    env.objc.borrow_mut::<UIViewHostObject>(this).clears_context_before_drawing = v;
}

// Drawing stuff that views should override
- (())drawRect:(CGRect)_rect {
    // default implementation does nothing
}

// CALayerDelegate implementation
- (())drawLayer:(id)layer // CALayer*
      inContext:(CGContextRef)context {
    let mut bounds: CGRect = msg![env; layer bounds];
    bounds.origin = CGPoint { x: 0.0, y: 0.0 }; // FIXME: not tested
    if env.objc.borrow::<UIViewHostObject>(this).clears_context_before_drawing {
        CGContextClearRect(env, context, bounds);
    }
    UIGraphicsPushContext(env, context);
    () = msg![env; this drawRect:bounds];
    UIGraphicsPopContext(env);
}

// Event handling

- (bool)pointInside:(CGPoint)point
          withEvent:(id)_event { // UIEvent* (possibly nil)
    let layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    msg![env; layer containsPoint:point]
}

- (id)hitTest:(CGPoint)point
    withEvent:(id)event { // UIEvent* (possibly nil)
    if !msg![env; this pointInside:point withEvent:event] {
        return nil;
    }
    // TODO: avoid copy somehow?
    let subviews = env.objc.borrow::<UIViewHostObject>(this).subviews.clone();
    for subview in subviews.into_iter().rev() { // later views are on top
        let hidden: bool = msg![env; subview isHidden];
        let alpha: CGFloat = msg![env; subview alpha];
        let interactible: bool = msg![env; subview isUserInteractionEnabled];
        if hidden || alpha < 0.01 || !interactible {
           continue;
        }
        let point: CGPoint = msg![env; subview convertPoint:point fromView:this];
        let subview: id = msg![env; subview hitTest:point withEvent:event];
        if subview != nil {
            return subview;
        }
    }
    this
}

// Ending a view-editing session

- (bool)endEditing:(bool)force {
    assert!(force);
    let responder: id = env.framework_state.uikit.ui_responder.first_responder;
    let class = msg![env; responder class];
    let ui_text_field_class = env.objc.get_known_class("UITextField", &mut env.mem);
    if responder != nil && env.objc.class_is_subclass_of(class, ui_text_field_class) {
        // we need to check if text field is in the current view hierarchy
        let mut to_find = responder;
        while to_find != nil {
            if to_find == this {
                return msg![env; responder resignFirstResponder];
            }
            to_find = msg![env; to_find superview];
        }
    }
    false
}

// UIResponder implementation
// From the Apple UIView docs regarding [UIResponder nextResponder]:
// "UIView implements this method and returns the UIViewController object that
//  manages it (if it has one) or its superview (if it doesn’t)."
- (id)nextResponder {
    let host_object = env.objc.borrow::<UIViewHostObject>(this);
    if host_object.view_controller != nil {
        host_object.view_controller
    } else {
        host_object.superview
    }
}

// Co-ordinate space conversion

- (CGPoint)convertPoint:(CGPoint)point
               fromView:(id)other { // UIView*
    if other == nil {
        let window: id = msg![env; this window];
        assert!(window != nil);
        return msg![env; this convertPoint:point fromView:window]
    }
    let this_layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    let other_layer = env.objc.borrow::<UIViewHostObject>(other).layer;
    msg![env; this_layer convertPoint:point fromLayer:other_layer]
}
- (CGPoint)convertPoint:(CGPoint)point
                 toView:(id)other { // UIView*
    if other == nil {
        let window: id = msg![env; this window];
        assert!(window != nil);
        return msg![env; this convertPoint:point toView:window]
    }
    let this_layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    let other_layer = env.objc.borrow::<UIViewHostObject>(other).layer;
    msg![env; this_layer convertPoint:point toLayer:other_layer]
}
- (CGRect)convertRect:(CGRect)rect
             fromView:(id)other { // UIView*
    if other == nil {
        let window: id = msg![env; this window];
        assert!(window != nil);
        return msg![env; this convertRect:rect fromView:window]
    }
    let this_layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    let other_layer = env.objc.borrow::<UIViewHostObject>(other).layer;
    msg![env; this_layer convertRect:rect fromLayer:other_layer]
}
- (CGRect)convertRect:(CGRect)rect
               toView:(id)other { // UIView*
    if other == nil {
        let window: id = msg![env; this window];
        assert!(window != nil);
        return msg![env; this convertRect:rect toView:window]
    }
    let this_layer = env.objc.borrow::<UIViewHostObject>(this).layer;
    let other_layer = env.objc.borrow::<UIViewHostObject>(other).layer;
    msg![env; this_layer convertRect:rect toLayer:other_layer]
}

- (())setAutoresizingMask:(NSUInteger)mask {
    todo_objc_setter!(this, mask);
}
- (())setAutoresizesSubviews:(bool)enabled {
    todo_objc_setter!(this, enabled);
}

- (CGSize)sizeThatFits:(CGSize)size {
    // default implementation, subclasses can override
    size
}
- (())sizeToFit {
    let current_frame: CGRect = msg![env; this frame];
    let current_bounds: CGRect = msg![env; this bounds];
    let size_that_fits: CGSize = msg![env; this sizeThatFits:(current_bounds.size)];

    let new_frame = CGRect {
        origin: current_frame.origin,
        size: size_that_fits,
    };

    () = msg![env; this setFrame:new_frame];
}

- (())setContentScaleFactor:(CGFloat)factor {
    todo_objc_setter!(this, factor);
}
- (CGFloat)contentScaleFactor {
    1.0 // TODO
}

@end

@implementation _touchHLE_UIView_AnimationDelegate: NSObject

+ (id)alloc {
    let host_object = Box::<UIViewAnimationDelegateHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())setAnimationId:(id)animation_id { // NSString*
    retain(env, animation_id);
    env.objc.borrow_mut::<UIViewAnimationDelegateHostObject>(this).animation_id = animation_id;
}

- (())setContext:(ConstVoidPtr)context {
    env.objc.borrow_mut::<UIViewAnimationDelegateHostObject>(this).context = context;
}

- (())setDelegate:(id)delegate {
    retain(env, delegate);
    env.objc.borrow_mut::<UIViewAnimationDelegateHostObject>(this).delegate = delegate;
}

- (())setWillStartSelector:(SEL)selector {
    env.objc.borrow_mut::<UIViewAnimationDelegateHostObject>(this).will_start_selector = Some(selector);
}

- (())setDidStopSelector:(SEL)selector {
    env.objc.borrow_mut::<UIViewAnimationDelegateHostObject>(this).did_stop_selector = Some(selector);
}

- (())setTotalAnimationCount:(NSUInteger)count {
    env.objc.borrow_mut::<UIViewAnimationDelegateHostObject>(this).total_animation_count = count;
}

- (())dealloc {
    let UIViewAnimationDelegateHostObject {
        animation_id,
        delegate,
        ..
    } = *env.objc.borrow::<UIViewAnimationDelegateHostObject>(this);
    release(env, animation_id);
    release(env, delegate);
    env.objc.dealloc_object(this, &mut env.mem)
}

// CAAnimationDelegate protocol implementation
- (())animationDidStart:(id)animation { // CAAnimation*
    let UIViewAnimationDelegateHostObject {
        started_animation_count,
        delegate,
        will_start_selector,
        context,
        animation_id,
        ..
    } = *env.objc.borrow::<UIViewAnimationDelegateHostObject>(this);
    let new_started_animation_count = started_animation_count + 1;
    log_dbg!("[(_touchHLE_UIView_AnimationDelegate*){:?} animationDidStart:{:?}] started_animation_count {} -> {}", this, animation, started_animation_count, new_started_animation_count);
    if started_animation_count == 0 && delegate != nil && will_start_selector.is_some() {
        let will_start_selector = will_start_selector.unwrap();
        log_dbg!("Notifying delegate {:?} {:?} {} with args {:?}, {:?}", delegate, will_start_selector, will_start_selector.as_str(&env.mem), animation_id, context);
        () = msg_send(env, (delegate, will_start_selector, animation_id, context));
    }
    env.objc.borrow_mut::<UIViewAnimationDelegateHostObject>(this).started_animation_count = new_started_animation_count;
}

- (())animationDidStop:(id)animation // CAAnimation*
              finished:(bool)finished {
    assert!(finished);
    let host_object = env.objc.borrow_mut::<UIViewAnimationDelegateHostObject>(this);
    let finished_animation_count = host_object.finished_animation_count;
    let new_finished_animation_count = finished_animation_count + finished as u32;
    log_dbg!("[(_touchHLE_UIView_AnimationDelegate*){:?} animationDidStop:{:?} finished:{}] finished_animation_count {} -> {}", this, animation, finished, finished_animation_count, new_finished_animation_count);
    env.objc.borrow_mut::<UIViewAnimationDelegateHostObject>(this).finished_animation_count = new_finished_animation_count;
    let UIViewAnimationDelegateHostObject {
        total_animation_count,
        finished_animation_count,
        delegate,
        did_stop_selector,
        context,
        animation_id,
        ..
    } = *env.objc.borrow::<UIViewAnimationDelegateHostObject>(this);
    if finished_animation_count == total_animation_count && delegate != nil && did_stop_selector.is_some() {
        let did_stop_selector = did_stop_selector.unwrap();
        let finished: id = msg_class![env; NSNumber numberWithBool:finished];
        log_dbg!("Notifying delegate {:?} {:?} {} with args {:?}, {:?}, {:?}", delegate, did_stop_selector, did_stop_selector.as_str(&env.mem), animation_id, finished, context);
        () = msg_send(env, (delegate, did_stop_selector, animation_id, finished, context));
    }
}

@end

};
