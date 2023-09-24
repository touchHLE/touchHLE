/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UITouch`.

use super::ui_event;
use super::ui_event::UIEventHostObject;
use crate::frameworks::core_graphics::{CGPoint, CGRect};
use crate::frameworks::foundation::{NSInteger, NSTimeInterval, NSUInteger};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::window::{Event, FingerId};
use crate::Environment;
use std::collections::HashMap;

pub type UITouchPhase = NSInteger;
pub const UITouchPhaseBegan: UITouchPhase = 0;
pub const UITouchPhaseMoved: UITouchPhase = 1;
pub const UITouchPhaseEnded: UITouchPhase = 3;

#[derive(Default)]
pub struct State {
    current_touches: HashMap<FingerId, id>,
}

pub(super) struct UITouchHostObject {
    /// Strong reference to the `UIView`
    view: id,
    /// Strong reference to the `UIWindow`, used as a reference for co-ordinate
    /// space conversion
    pub(super) window: id,
    /// Relative to the screen
    location: CGPoint,
    /// Relative to the screen
    previous_location: CGPoint,
    /// Relative to the screen, used for `touchesForView:`
    pub(super) original_location: CGPoint,
    timestamp: NSTimeInterval,
    phase: UITouchPhase,
}
impl HostObject for UITouchHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UITouch: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UITouchHostObject {
        view: nil,
        window: nil,
        location: CGPoint { x: 0.0, y: 0.0 },
        previous_location: CGPoint { x: 0.0, y: 0.0 },
        original_location: CGPoint { x: 0.0, y: 0.0 },
        timestamp: 0.0,
        phase: UITouchPhaseBegan,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    let &mut UITouchHostObject { view, window, .. } = env.objc.borrow_mut(this);
    release(env, view);
    release(env, window);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (CGPoint)locationInView:(id)that_view { // UIView*
    let &UITouchHostObject { location, window, .. } = env.objc.borrow(this);
    if that_view == nil {
        location // TODO: this should use convertPoint:fromView: too
    } else {
        msg![env; that_view convertPoint:location fromView:window]
    }
}
- (CGPoint)previousLocationInView:(id)that_view { // UIView*
    let &UITouchHostObject { previous_location, window, .. } = env.objc.borrow(this);
    if that_view == nil {
        previous_location // TODO: this should use convertPoint:fromView: too
    } else {
        msg![env; that_view convertPoint:previous_location fromView:window]
    }
}

- (id)view {
    env.objc.borrow::<UITouchHostObject>(this).view
}

- (NSTimeInterval)timestamp {
    env.objc.borrow::<UITouchHostObject>(this).timestamp
}

- (NSUInteger)tapCount {
    1 // TODO: support double-taps etc
}

- (UITouchPhase)phase {
    env.objc.borrow::<UITouchHostObject>(this).phase
}

@end

};

/// [super::handle_events] will forward touch events to this function.
pub fn handle_event(env: &mut Environment, event: Event) {
    match event {
        Event::TouchesDown(map) => {
            let finger_id = 0;
            assert!(map.len() == 1 && map.contains_key(&finger_id));
            let coords = map.get(&finger_id).unwrap();

            if env
                .framework_state
                .uikit
                .ui_touch
                .current_touches
                .contains_key(&finger_id)
            {
                log!("Warning: New touch initiated but current touch did not end yet, treating as movement.");
                return handle_event(env, Event::TouchesMove(map));
            }

            log_dbg!("Touch down: {:?}", coords);

            let location = CGPoint {
                x: coords.0,
                y: coords.1,
            };

            // UIKit creates and drains autorelease pools when handling events.
            let pool: id = msg_class![env; NSAutoreleasePool new];

            // Note: if the emulator is heavily lagging, this timestamp is going
            // to be far off from the truth, since it should represent the
            // time when the event actually happened, not the time when the
            // event was dispatched. Maybe we'll need to fix this eventually.
            let timestamp: NSTimeInterval = msg_class![env; NSProcessInfo systemUptime];

            // TODO: is this the correct state of the UITouch and UIEvent during
            //       hit testing?

            let new_touch: id = msg_class![env; UITouch alloc];
            *env.objc.borrow_mut(new_touch) = UITouchHostObject {
                view: nil,
                window: nil,
                location,
                previous_location: location,
                original_location: location,
                timestamp,
                phase: UITouchPhaseBegan,
            };
            autorelease(env, new_touch);

            let touches: id = msg_class![env; NSSet setWithObject:new_touch];
            let event = ui_event::new_event(env, touches, nil);
            autorelease(env, event);

            // FIXME: handle non-fullscreen windows in hit testing and
            //        co-ordinate space translation.

            // Assumes the last window in the list is the one on top.
            // TODO: this is not correct once we support zPosition.
            let Some(&top_window) = env
                .framework_state
                .uikit
                .ui_view
                .ui_window
                .visible_windows
                .last()
            else {
                log!("No visible window, touch event ignored");
                return;
            };

            let view: id = msg![env; top_window hitTest:location withEvent:event];
            if view == nil {
                log!(
                    "Couldn't find a view for touch at {:?} in window {:?}, discarding",
                    location,
                    top_window,
                );
                return;
            } else {
                log_dbg!(
                    "Found view {:?} with frame {:?} for touch at {:?} in window {:?}",
                    view,
                    {
                        let f: CGRect = msg![env; view frame];
                        f
                    },
                    location,
                    top_window,
                );
            }

            retain(env, view);
            retain(env, top_window);
            {
                let new_touch = env.objc.borrow_mut::<UITouchHostObject>(new_touch);
                new_touch.view = view;
                new_touch.window = top_window;
            }

            retain(env, view);
            env.objc.borrow_mut::<UIEventHostObject>(event).view = view;

            env.framework_state
                .uikit
                .ui_touch
                .current_touches
                .insert(finger_id, new_touch);
            retain(env, new_touch);

            log_dbg!(
                "Sending [{:?} touchesBegan:{:?} withEvent:{:?}]",
                view,
                touches,
                event
            );
            let _: () = msg![env; view touchesBegan:touches withEvent:event];

            release(env, pool);
        }
        Event::TouchesMove(map) => {
            let finger_id = 0;
            assert!(map.len() == 1 && map.contains_key(&finger_id));
            let coords = map.get(&finger_id).unwrap();

            let Some(&touch) = env
                .framework_state
                .uikit
                .ui_touch
                .current_touches
                .get(&finger_id)
            else {
                log!("Warning: Touch move event received but no current touch, ignoring.");
                return;
            };

            log_dbg!("Touch move: {:?}", coords);

            let location = CGPoint {
                x: coords.0,
                y: coords.1,
            };

            let timestamp: NSTimeInterval = msg_class![env; NSProcessInfo systemUptime];

            let view = env.objc.borrow::<UITouchHostObject>(touch).view;
            let host_object = env.objc.borrow_mut::<UITouchHostObject>(touch);
            host_object.previous_location = host_object.location;
            host_object.location = location;
            host_object.timestamp = timestamp;
            host_object.phase = UITouchPhaseMoved;

            let pool: id = msg_class![env; NSAutoreleasePool new];

            let touches: id = msg_class![env; NSSet setWithObject:touch];
            let event = ui_event::new_event(env, touches, view);
            autorelease(env, event);

            log_dbg!(
                "Sending [{:?} touchesMoved:{:?} withEvent:{:?}]",
                view,
                touches,
                event
            );
            let _: () = msg![env; view touchesMoved:touches withEvent:event];

            release(env, pool);
        }
        Event::TouchesUp(map) => {
            let finger_id = 0;
            assert!(map.len() == 1 && map.contains_key(&finger_id));
            let coords = map.get(&finger_id).unwrap();

            let Some(&touch) = env
                .framework_state
                .uikit
                .ui_touch
                .current_touches
                .get(&finger_id)
            else {
                log!("Warning: Touch up event received but no current touch, ignoring.");
                return;
            };

            log_dbg!("Touch up: {:?}", coords);

            let location = CGPoint {
                x: coords.0,
                y: coords.1,
            };

            let timestamp: NSTimeInterval = msg_class![env; NSProcessInfo systemUptime];

            let view = env.objc.borrow::<UITouchHostObject>(touch).view;
            let host_object = env.objc.borrow_mut::<UITouchHostObject>(touch);
            host_object.previous_location = host_object.location;
            host_object.location = location;
            host_object.timestamp = timestamp;
            host_object.phase = UITouchPhaseEnded;

            let pool: id = msg_class![env; NSAutoreleasePool new];

            let touches: id = msg_class![env; NSSet setWithObject:touch];
            let event = ui_event::new_event(env, touches, view);
            autorelease(env, event);

            env.framework_state
                .uikit
                .ui_touch
                .current_touches
                .remove(&finger_id);
            release(env, touch); // only owner now should be the NSSet

            log_dbg!(
                "Sending [{:?} touchesEnded:{:?} withEvent:{:?}]",
                view,
                touches,
                event
            );
            let _: () = msg![env; view touchesEnded:touches withEvent:event];

            release(env, pool);
        }
        _ => unreachable!(),
    }
}
