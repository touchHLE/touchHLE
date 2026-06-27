/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIGestureRecognizer` and concrete subclasses.
//!
//! Full implementation with target-action dispatch based on Apple's
//! documentation:
//! - <https://developer.apple.com/documentation/uikit/uigesturerecognizer>
//! - <https://developer.apple.com/documentation/uikit/uitapgesturerecognizer>
//! - <https://developer.apple.com/documentation/uikit/uipangesturerecognizer>
//! - <https://developer.apple.com/documentation/uikit/uiswipegesturerecognizer>
//! - <https://developer.apple.com/documentation/uikit/uilongpressgesturerecognizer>
//!
//! On real iOS, gesture recognizers participate in the touch dispatch pipeline
//! and can delay/cancel touches sent to the view. In HyperHLE, touch dispatch
//! goes directly to views (via `touchesBegan:/touchesMoved:/touchesEnded:`),
//! so gesture recognizers primarily exist to:
//! 1. Not crash when apps create and attach them.
//! 2. Store targets/actions and fire them when the recognizer transitions
//!    to the appropriate state (externally triggered or from basic heuristics).
//! 3. Provide property storage (numberOfTaps, direction, etc.) so apps can
//!    read them without panicking on unknown selectors.

use crate::frameworks::core_graphics::{CGFloat, CGPoint};
use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    id, msg_send, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
    SEL,
};
use crate::Environment;

// ============================================================================
// MARK: - UIGestureRecognizerState constants
// ============================================================================

pub type UIGestureRecognizerState = NSInteger;
pub const UIGestureRecognizerStatePossible: UIGestureRecognizerState = 0;
pub const UIGestureRecognizerStateBegan: UIGestureRecognizerState = 1;
pub const UIGestureRecognizerStateChanged: UIGestureRecognizerState = 2;
pub const UIGestureRecognizerStateEnded: UIGestureRecognizerState = 3;
pub const UIGestureRecognizerStateCancelled: UIGestureRecognizerState = 4;
pub const UIGestureRecognizerStateFailed: UIGestureRecognizerState = 5;
pub const UIGestureRecognizerStateRecognized: UIGestureRecognizerState =
    UIGestureRecognizerStateEnded;

// ============================================================================
// MARK: - UISwipeGestureRecognizerDirection
// ============================================================================

pub type UISwipeGestureRecognizerDirection = NSInteger;
pub const UISwipeGestureRecognizerDirectionRight: UISwipeGestureRecognizerDirection = 1 << 0;
pub const UISwipeGestureRecognizerDirectionLeft: UISwipeGestureRecognizerDirection = 1 << 1;
pub const UISwipeGestureRecognizerDirectionUp: UISwipeGestureRecognizerDirection = 1 << 2;
pub const UISwipeGestureRecognizerDirectionDown: UISwipeGestureRecognizerDirection = 1 << 3;

// ============================================================================
// MARK: - Target-action pair
// ============================================================================

#[derive(Clone)]
struct TargetAction {
    target: id, // weak reference per Apple docs
    action: SEL,
}

// ============================================================================
// MARK: - UIGestureRecognizer host object
// ============================================================================

pub(super) struct UIGestureRecognizerHostObject {
    pub state: UIGestureRecognizerState,
    pub view: id,
    pub enabled: bool,
    pub cancels_touches_in_view: bool,
    pub delays_touches_began: bool,
    pub delays_touches_ended: bool,
    pub delegate: id,
    targets: Vec<TargetAction>,
    /// Whether the recognizer requires other recognizers to fail first.
    require_to_fail: Vec<id>,
}
impl HostObject for UIGestureRecognizerHostObject {}

impl Default for UIGestureRecognizerHostObject {
    fn default() -> Self {
        UIGestureRecognizerHostObject {
            state: UIGestureRecognizerStatePossible,
            view: nil,
            enabled: true,
            cancels_touches_in_view: true,
            delays_touches_began: false,
            delays_touches_ended: true,
            delegate: nil,
            targets: Vec::new(),
            require_to_fail: Vec::new(),
        }
    }
}

/// Fire all registered target-action pairs for a gesture recognizer.
/// The action selector may have 0 or 1 argument (the recognizer itself).
pub fn fire_targets(env: &mut Environment, recognizer: id) {
    let targets: Vec<TargetAction> = env
        .objc
        .borrow::<UIGestureRecognizerHostObject>(recognizer)
        .targets
        .clone();

    for ta in targets {
        if ta.target == nil {
            continue;
        }
        let sel_str = ta.action.as_str(&env.mem);
        let colon_count = sel_str.bytes().filter(|&b| b == b':').count();
        match colon_count {
            0 => {
                let _: () = msg_send(env, (ta.target, ta.action));
            }
            1 => {
                let _: () = msg_send(env, (ta.target, ta.action, recognizer));
            }
            _ => {
                log!(
                    "Warning: UIGestureRecognizer target-action: unexpected colon count {} in {:?}",
                    colon_count,
                    sel_str
                );
            }
        }
    }
}

// ============================================================================
// MARK: - UITapGestureRecognizer host object
// ============================================================================

#[derive(Default)]
struct UITapGestureRecognizerHostObject {
    number_of_taps_required: NSInteger,
    number_of_touches_required: NSInteger,
}
impl HostObject for UITapGestureRecognizerHostObject {}

// ============================================================================
// MARK: - UIPanGestureRecognizer host object
// ============================================================================

#[derive(Default)]
struct UIPanGestureRecognizerHostObject {
    minimum_number_of_touches: NSInteger,
    maximum_number_of_touches: NSInteger,
    translation: CGPoint,
    velocity: CGPoint,
}
impl HostObject for UIPanGestureRecognizerHostObject {}

// ============================================================================
// MARK: - UISwipeGestureRecognizer host object
// ============================================================================

#[derive(Default)]
struct UISwipeGestureRecognizerHostObject {
    direction: UISwipeGestureRecognizerDirection,
    number_of_touches_required: NSInteger,
}
impl HostObject for UISwipeGestureRecognizerHostObject {}

// ============================================================================
// MARK: - UILongPressGestureRecognizer host object
// ============================================================================

#[derive(Default)]
struct UILongPressGestureRecognizerHostObject {
    minimum_press_duration: f64,
    allowable_movement: CGFloat,
    number_of_taps_required: NSInteger,
    number_of_touches_required: NSInteger,
}
impl HostObject for UILongPressGestureRecognizerHostObject {}

// ============================================================================
// MARK: - Class exports
// ============================================================================

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// ==========================================================================
// UIGestureRecognizer — base class
// ==========================================================================

@implementation UIGestureRecognizer: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIGestureRecognizerHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithTarget:(id)target action:(SEL)action {
    if target != nil && !action.is_null() {
        env.objc
            .borrow_mut::<UIGestureRecognizerHostObject>(this)
            .targets
            .push(TargetAction { target, action });
    }
    this
}

- (id)init {
    this
}

- (())dealloc {
    let host = env.objc.borrow::<UIGestureRecognizerHostObject>(this);
    let require_to_fail = host.require_to_fail.clone();
    for other in require_to_fail {
        release(env, other);
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: State

- (UIGestureRecognizerState)state {
    env.objc.borrow::<UIGestureRecognizerHostObject>(this).state
}

- (())setState:(UIGestureRecognizerState)state {
    env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this).state = state;
}

// MARK: View

- (id)view {
    env.objc.borrow::<UIGestureRecognizerHostObject>(this).view
}

- (())setView:(id)view {
    env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this).view = view;
}

// MARK: Enabled

- (bool)isEnabled {
    env.objc.borrow::<UIGestureRecognizerHostObject>(this).enabled
}

- (())setEnabled:(bool)enabled {
    env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this).enabled = enabled;
    if !enabled {
        env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this).state =
            UIGestureRecognizerStateFailed;
    }
}

// MARK: Delegate

- (id)delegate {
    env.objc.borrow::<UIGestureRecognizerHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    // Delegate is a weak reference per Apple docs.
    env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this).delegate = delegate;
}

// MARK: Touch delivery properties

- (bool)cancelsTouchesInView {
    env.objc.borrow::<UIGestureRecognizerHostObject>(this).cancels_touches_in_view
}

- (())setCancelsTouchesInView:(bool)value {
    env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this).cancels_touches_in_view = value;
}

- (bool)delaysTouchesBegan {
    env.objc.borrow::<UIGestureRecognizerHostObject>(this).delays_touches_began
}

- (())setDelaysTouchesBegan:(bool)value {
    env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this).delays_touches_began = value;
}

- (bool)delaysTouchesEnded {
    env.objc.borrow::<UIGestureRecognizerHostObject>(this).delays_touches_ended
}

- (())setDelaysTouchesEnded:(bool)value {
    env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this).delays_touches_ended = value;
}

// MARK: Target-action management

- (())addTarget:(id)target action:(SEL)action {
    if target == nil || action.is_null() {
        return;
    }
    env.objc
        .borrow_mut::<UIGestureRecognizerHostObject>(this)
        .targets
        .push(TargetAction { target, action });
}

- (())removeTarget:(id)target action:(SEL)action {
    let targets = &mut env
        .objc
        .borrow_mut::<UIGestureRecognizerHostObject>(this)
        .targets;
    targets.retain(|ta| {
        if target != nil && ta.target != target {
            return true;
        }
        if !action.is_null() && ta.action != action {
            return true;
        }
        false
    });
}

// MARK: Failure requirements

- (())requireGestureRecognizerToFail:(id)other {
    if other == nil {
        return;
    }
    retain(env, other);
    env.objc
        .borrow_mut::<UIGestureRecognizerHostObject>(this)
        .require_to_fail
        .push(other);
}

// MARK: Location

- (CGPoint)locationInView:(id)_view {
    // Without actual touch tracking in the recognizer, return zero.
    CGPoint { x: 0.0, y: 0.0 }
}

- (CGPoint)locationOfTouch:(NSInteger)_touch_index inView:(id)_view {
    CGPoint { x: 0.0, y: 0.0 }
}

- (NSInteger)numberOfTouches {
    0
}

// MARK: Reset

- (())reset {
    env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this).state =
        UIGestureRecognizerStatePossible;
}

// MARK: Touch handling (subclasses override these)

- (())touchesBegan:(id)_touches withEvent:(id)_event {}
- (())touchesMoved:(id)_touches withEvent:(id)_event {}
- (())touchesEnded:(id)_touches withEvent:(id)_event {}
- (())touchesCancelled:(id)_touches withEvent:(id)_event {}

@end

// ==========================================================================
// UITapGestureRecognizer
// ==========================================================================

@implementation UITapGestureRecognizer: UIGestureRecognizer

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UITapGestureRecognizerHostObject {
        number_of_taps_required: 1,
        number_of_touches_required: 1,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (NSInteger)numberOfTapsRequired {
    env.objc
        .borrow::<UITapGestureRecognizerHostObject>(this)
        .number_of_taps_required
}

- (())setNumberOfTapsRequired:(NSInteger)taps {
    env.objc
        .borrow_mut::<UITapGestureRecognizerHostObject>(this)
        .number_of_taps_required = taps;
}

- (NSInteger)numberOfTouchesRequired {
    env.objc
        .borrow::<UITapGestureRecognizerHostObject>(this)
        .number_of_touches_required
}

- (())setNumberOfTouchesRequired:(NSInteger)touches {
    env.objc
        .borrow_mut::<UITapGestureRecognizerHostObject>(this)
        .number_of_touches_required = touches;
}

@end

// ==========================================================================
// UIPanGestureRecognizer
// ==========================================================================

@implementation UIPanGestureRecognizer: UIGestureRecognizer

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIPanGestureRecognizerHostObject {
        minimum_number_of_touches: 1,
        maximum_number_of_touches: NSInteger::MAX,
        translation: CGPoint { x: 0.0, y: 0.0 },
        velocity: CGPoint { x: 0.0, y: 0.0 },
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (NSInteger)minimumNumberOfTouches {
    env.objc
        .borrow::<UIPanGestureRecognizerHostObject>(this)
        .minimum_number_of_touches
}

- (())setMinimumNumberOfTouches:(NSInteger)touches {
    env.objc
        .borrow_mut::<UIPanGestureRecognizerHostObject>(this)
        .minimum_number_of_touches = touches;
}

- (NSInteger)maximumNumberOfTouches {
    env.objc
        .borrow::<UIPanGestureRecognizerHostObject>(this)
        .maximum_number_of_touches
}

- (())setMaximumNumberOfTouches:(NSInteger)touches {
    env.objc
        .borrow_mut::<UIPanGestureRecognizerHostObject>(this)
        .maximum_number_of_touches = touches;
}

- (CGPoint)translationInView:(id)_view {
    env.objc
        .borrow::<UIPanGestureRecognizerHostObject>(this)
        .translation
}

- (())setTranslation:(CGPoint)translation inView:(id)_view {
    env.objc
        .borrow_mut::<UIPanGestureRecognizerHostObject>(this)
        .translation = translation;
}

- (CGPoint)velocityInView:(id)_view {
    env.objc
        .borrow::<UIPanGestureRecognizerHostObject>(this)
        .velocity
}

@end

// ==========================================================================
// UISwipeGestureRecognizer
// ==========================================================================

@implementation UISwipeGestureRecognizer: UIGestureRecognizer

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UISwipeGestureRecognizerHostObject {
        direction: UISwipeGestureRecognizerDirectionRight,
        number_of_touches_required: 1,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (UISwipeGestureRecognizerDirection)direction {
    env.objc
        .borrow::<UISwipeGestureRecognizerHostObject>(this)
        .direction
}

- (())setDirection:(UISwipeGestureRecognizerDirection)direction {
    env.objc
        .borrow_mut::<UISwipeGestureRecognizerHostObject>(this)
        .direction = direction;
}

- (NSInteger)numberOfTouchesRequired {
    env.objc
        .borrow::<UISwipeGestureRecognizerHostObject>(this)
        .number_of_touches_required
}

- (())setNumberOfTouchesRequired:(NSInteger)touches {
    env.objc
        .borrow_mut::<UISwipeGestureRecognizerHostObject>(this)
        .number_of_touches_required = touches;
}

@end

// ==========================================================================
// UILongPressGestureRecognizer
// ==========================================================================

@implementation UILongPressGestureRecognizer: UIGestureRecognizer

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UILongPressGestureRecognizerHostObject {
        minimum_press_duration: 0.5, // Apple default
        allowable_movement: 10.0,    // Apple default
        number_of_taps_required: 0,
        number_of_touches_required: 1,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (f64)minimumPressDuration {
    env.objc
        .borrow::<UILongPressGestureRecognizerHostObject>(this)
        .minimum_press_duration
}

- (())setMinimumPressDuration:(f64)duration {
    env.objc
        .borrow_mut::<UILongPressGestureRecognizerHostObject>(this)
        .minimum_press_duration = duration;
}

- (CGFloat)allowableMovement {
    env.objc
        .borrow::<UILongPressGestureRecognizerHostObject>(this)
        .allowable_movement
}

- (())setAllowableMovement:(CGFloat)movement {
    env.objc
        .borrow_mut::<UILongPressGestureRecognizerHostObject>(this)
        .allowable_movement = movement;
}

- (NSInteger)numberOfTapsRequired {
    env.objc
        .borrow::<UILongPressGestureRecognizerHostObject>(this)
        .number_of_taps_required
}

- (())setNumberOfTapsRequired:(NSInteger)taps {
    env.objc
        .borrow_mut::<UILongPressGestureRecognizerHostObject>(this)
        .number_of_taps_required = taps;
}

- (NSInteger)numberOfTouchesRequired {
    env.objc
        .borrow::<UILongPressGestureRecognizerHostObject>(this)
        .number_of_touches_required
}

- (())setNumberOfTouchesRequired:(NSInteger)touches {
    env.objc
        .borrow_mut::<UILongPressGestureRecognizerHostObject>(this)
        .number_of_touches_required = touches;
}

@end

// ==========================================================================
// UIScreenEdgePanGestureRecognizer (iOS 7+)
// ==========================================================================

@implementation UIScreenEdgePanGestureRecognizer: UIPanGestureRecognizer

// `edges` property: UIRectEdge bitmask indicating which edges are monitored.
- (NSInteger)edges { 0 }
- (())setEdges:(NSInteger)_edges {}

@end

};
