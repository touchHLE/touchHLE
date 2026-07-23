/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIGestureRecognizer` and the tap/swipe recognizers available in iOS 3.2.

use crate::frameworks::core_graphics::CGPoint;
use crate::frameworks::foundation::{NSInteger, NSUInteger};
use crate::objc::{id, msg, msg_send, nil, objc_classes, ClassExports, HostObject, NSZonePtr, SEL};
use crate::Environment;

pub type UIGestureRecognizerState = NSInteger;
pub const UIGestureRecognizerStatePossible: UIGestureRecognizerState = 0;
pub const UIGestureRecognizerStateRecognized: UIGestureRecognizerState = 3;
pub const UIGestureRecognizerStateFailed: UIGestureRecognizerState = 5;

pub type UISwipeGestureRecognizerDirection = NSUInteger;
pub const UISwipeGestureRecognizerDirectionRight: UISwipeGestureRecognizerDirection = 1 << 0;
pub const UISwipeGestureRecognizerDirectionLeft: UISwipeGestureRecognizerDirection = 1 << 1;
pub const UISwipeGestureRecognizerDirectionUp: UISwipeGestureRecognizerDirection = 1 << 2;
pub const UISwipeGestureRecognizerDirectionDown: UISwipeGestureRecognizerDirection = 1 << 3;

#[derive(Clone, Copy, PartialEq, Eq)]
enum GestureKind {
    Generic,
    Tap,
    Swipe,
}

pub(super) struct UIGestureRecognizerHostObject {
    // UIKit does not retain targets, delegates or the associated view.
    target: id,
    action: Option<SEL>,
    delegate: id,
    view: id,
    kind: GestureKind,
    state: UIGestureRecognizerState,
    enabled: bool,
    cancels_touches_in_view: bool,
    delays_touches_began: bool,
    delays_touches_ended: bool,
    number_of_taps_required: NSUInteger,
    number_of_touches_required: NSUInteger,
    direction: UISwipeGestureRecognizerDirection,
    initial_location: CGPoint,
    current_location: CGPoint,
    tracking: bool,
}
impl HostObject for UIGestureRecognizerHostObject {}

impl UIGestureRecognizerHostObject {
    fn new(kind: GestureKind) -> Self {
        Self {
            target: nil,
            action: None,
            delegate: nil,
            view: nil,
            kind,
            state: UIGestureRecognizerStatePossible,
            enabled: true,
            cancels_touches_in_view: true,
            delays_touches_began: false,
            delays_touches_ended: true,
            number_of_taps_required: 1,
            number_of_touches_required: 1,
            direction: UISwipeGestureRecognizerDirectionRight,
            initial_location: CGPoint { x: 0.0, y: 0.0 },
            current_location: CGPoint { x: 0.0, y: 0.0 },
            tracking: false,
        }
    }
}

fn init_with_target(env: &mut Environment, this: id, target: id, action: SEL) -> id {
    let recognizer = env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this);
    recognizer.target = target;
    recognizer.action = Some(action);
    this
}

fn send_action(env: &mut Environment, recognizer: id) {
    let (target, action) = {
        let host = env.objc.borrow::<UIGestureRecognizerHostObject>(recognizer);
        (host.target, host.action)
    };
    let Some(action) = action else { return };
    if target == nil {
        log!("TODO: UIGestureRecognizer target:nil responder-chain dispatch is unsupported");
        return;
    }

    let colon_count = action
        .as_str(&env.mem)
        .bytes()
        .filter(|&b| b == b':')
        .count();
    match colon_count {
        0 => () = msg_send(env, (target, action)),
        1 => () = msg_send(env, (target, action, recognizer)),
        _ => panic!("Unexpected gesture recognizer action {:?}", action),
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIGestureRecognizer: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIGestureRecognizerHostObject::new(GestureKind::Generic));
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithTarget:(id)target action:(SEL)action {
    init_with_target(env, this, target, action)
}

- (id)delegate { env.objc.borrow::<UIGestureRecognizerHostObject>(this).delegate }
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this).delegate = delegate;
}
- (id)view { env.objc.borrow::<UIGestureRecognizerHostObject>(this).view }
- (UIGestureRecognizerState)state {
    env.objc.borrow::<UIGestureRecognizerHostObject>(this).state
}
- (bool)isEnabled { env.objc.borrow::<UIGestureRecognizerHostObject>(this).enabled }
- (())setEnabled:(bool)enabled {
    let host = env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this);
    host.enabled = enabled;
    host.tracking = false;
    host.state = UIGestureRecognizerStatePossible;
}
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
- (CGPoint)locationInView:(id)view {
    let (current_location, own_view) = {
        let host = env.objc.borrow::<UIGestureRecognizerHostObject>(this);
        (host.current_location, host.view)
    };
    if view == nil || view == own_view {
        current_location
    } else {
        msg![env; view convertPoint:current_location fromView:own_view]
    }
}

@end

@implementation UITapGestureRecognizer: UIGestureRecognizer

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIGestureRecognizerHostObject::new(GestureKind::Tap));
    env.objc.alloc_object(this, host_object, &mut env.mem)
}
- (NSUInteger)numberOfTapsRequired {
    env.objc.borrow::<UIGestureRecognizerHostObject>(this).number_of_taps_required
}
- (())setNumberOfTapsRequired:(NSUInteger)value {
    env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this).number_of_taps_required = value;
}
- (NSUInteger)numberOfTouchesRequired {
    env.objc.borrow::<UIGestureRecognizerHostObject>(this).number_of_touches_required
}
- (())setNumberOfTouchesRequired:(NSUInteger)value {
    env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this).number_of_touches_required = value;
}

@end


@implementation UISwipeGestureRecognizer: UIGestureRecognizer

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIGestureRecognizerHostObject::new(GestureKind::Swipe));
    env.objc.alloc_object(this, host_object, &mut env.mem)
}
- (UISwipeGestureRecognizerDirection)direction {
    env.objc.borrow::<UIGestureRecognizerHostObject>(this).direction
}
- (())setDirection:(UISwipeGestureRecognizerDirection)value {
    env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this).direction = value;
}
- (NSUInteger)numberOfTouchesRequired {
    env.objc.borrow::<UIGestureRecognizerHostObject>(this).number_of_touches_required
}
- (())setNumberOfTouchesRequired:(NSUInteger)value {
    env.objc.borrow_mut::<UIGestureRecognizerHostObject>(this).number_of_touches_required = value;
}

@end

};

pub(super) fn set_view(env: &mut Environment, recognizer: id, view: id) {
    env.objc
        .borrow_mut::<UIGestureRecognizerHostObject>(recognizer)
        .view = view;
}

fn delegate_allows_touch(env: &mut Environment, recognizer: id, touch: id) -> bool {
    let delegate = env
        .objc
        .borrow::<UIGestureRecognizerHostObject>(recognizer)
        .delegate;
    if delegate == nil {
        return true;
    }
    let Some(selector) = env
        .objc
        .lookup_selector("gestureRecognizer:shouldReceiveTouch:")
    else {
        return true;
    };
    if msg![env; delegate respondsToSelector:selector] {
        msg_send(env, (delegate, selector, recognizer, touch))
    } else {
        true
    }
}

fn delegate_allows_begin(env: &mut Environment, recognizer: id) -> bool {
    let delegate = env
        .objc
        .borrow::<UIGestureRecognizerHostObject>(recognizer)
        .delegate;
    if delegate == nil {
        return true;
    }
    let Some(selector) = env.objc.lookup_selector("gestureRecognizerShouldBegin:") else {
        return true;
    };
    if msg![env; delegate respondsToSelector:selector] {
        msg_send(env, (delegate, selector, recognizer))
    } else {
        true
    }
}

pub(super) fn touches_began(env: &mut Environment, view: id, touches: id) {
    if view == nil {
        return;
    }
    let touch: id = msg![env; touches anyObject];
    if touch == nil {
        return;
    }
    let recognizers = super::ui_view::gesture_recognizers(env, view);
    for recognizer in recognizers {
        let (enabled, recognizer_view) = {
            let host = env.objc.borrow::<UIGestureRecognizerHostObject>(recognizer);
            (host.enabled, host.view)
        };
        if !enabled || !delegate_allows_touch(env, recognizer, touch) {
            continue;
        }
        let location: CGPoint = msg![env; touch locationInView:recognizer_view];
        let host = env
            .objc
            .borrow_mut::<UIGestureRecognizerHostObject>(recognizer);
        host.initial_location = location;
        host.current_location = location;
        host.state = UIGestureRecognizerStatePossible;
        host.tracking = true;
    }
}

pub(super) fn touches_moved(env: &mut Environment, view: id, touches: id) {
    if view == nil {
        return;
    }
    let touch: id = msg![env; touches anyObject];
    if touch == nil {
        return;
    }
    let recognizers = super::ui_view::gesture_recognizers(env, view);
    for recognizer in recognizers {
        let (tracking, recognizer_view) = {
            let host = env.objc.borrow::<UIGestureRecognizerHostObject>(recognizer);
            (host.tracking, host.view)
        };
        if tracking {
            let location: CGPoint = msg![env; touch locationInView:recognizer_view];
            env.objc
                .borrow_mut::<UIGestureRecognizerHostObject>(recognizer)
                .current_location = location;
        }
    }
}

pub(super) fn touches_ended(env: &mut Environment, view: id, touches: id) {
    if view == nil {
        return;
    }
    let touch: id = msg![env; touches anyObject];
    if touch == nil {
        return;
    }
    let recognizers = super::ui_view::gesture_recognizers(env, view);
    for recognizer in recognizers {
        let (tracking, recognizer_view) = {
            let host = env.objc.borrow::<UIGestureRecognizerHostObject>(recognizer);
            (host.tracking, host.view)
        };
        if !tracking {
            continue;
        }
        let location: CGPoint = msg![env; touch locationInView:recognizer_view];
        {
            let host = env
                .objc
                .borrow_mut::<UIGestureRecognizerHostObject>(recognizer);
            host.current_location = location;
            host.tracking = false;
        }

        let recognized = {
            let host = env.objc.borrow::<UIGestureRecognizerHostObject>(recognizer);
            let dx = host.current_location.x - host.initial_location.x;
            let dy = host.current_location.y - host.initial_location.y;
            match host.kind {
                GestureKind::Tap => host.number_of_taps_required == 1 && dx * dx + dy * dy <= 100.0,
                GestureKind::Swipe => {
                    let (direction, distance, cross_distance) = if dx.abs() >= dy.abs() {
                        (
                            if dx >= 0.0 {
                                UISwipeGestureRecognizerDirectionRight
                            } else {
                                UISwipeGestureRecognizerDirectionLeft
                            },
                            dx.abs(),
                            dy.abs(),
                        )
                    } else {
                        (
                            if dy >= 0.0 {
                                UISwipeGestureRecognizerDirectionDown
                            } else {
                                UISwipeGestureRecognizerDirectionUp
                            },
                            dy.abs(),
                            dx.abs(),
                        )
                    };
                    distance >= 24.0
                        && cross_distance <= distance
                        && host.direction & direction != 0
                }
                GestureKind::Generic => false,
            }
        };

        if recognized && delegate_allows_begin(env, recognizer) {
            env.objc
                .borrow_mut::<UIGestureRecognizerHostObject>(recognizer)
                .state = UIGestureRecognizerStateRecognized;
            send_action(env, recognizer);
        } else {
            env.objc
                .borrow_mut::<UIGestureRecognizerHostObject>(recognizer)
                .state = UIGestureRecognizerStateFailed;
        }
    }
}
