/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UITouch`.

use super::ui_event;
use super::ui_view::UIViewHostObject;
use crate::frameworks::core_graphics::{CGFloat, CGPoint};
use crate::frameworks::foundation::{NSTimeInterval, NSUInteger};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::window::Event;
use crate::Environment;

#[derive(Default)]
pub struct State {
    current_touch: Option<id>,
}

struct UITouchHostObject {
    /// Strong reference to the `UIView`
    view: id,
    /// Relative to screen
    location: CGPoint,
    /// Relative to screen
    previous_location: CGPoint,
    timestamp: NSTimeInterval,
}
impl HostObject for UITouchHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UITouch: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UITouchHostObject {
        view: nil,
        location: CGPoint { x: 0.0, y: 0.0 },
        previous_location: CGPoint { x: 0.0, y: 0.0 },
        timestamp: 0.0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    let &mut UITouchHostObject { view, .. } = env.objc.borrow_mut(this);
    release(env, view);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (CGPoint)locationInView:(id)that_view { // UIView*
    let &UITouchHostObject { location, .. } = env.objc.borrow(this);
    if that_view == nil {
        location
    } else {
        // FIXME, see below
        // Note: also change touchesForView: on UIEvent
        resolve_point_in_view(env, that_view, location).unwrap()
    }
}
- (CGPoint)previousLocationInView:(id)that_view { // UIView*
    let &UITouchHostObject { previous_location, .. } = env.objc.borrow(this);
    if that_view == nil {
        previous_location
    } else {
        // FIXME, see below
        // Note: also change touchesForView: on UIEvent
        resolve_point_in_view(env, that_view, previous_location).unwrap()
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

@end

};

pub fn resolve_point_in_view(env: &mut Environment, view: id, point: CGPoint) -> Option<CGPoint> {
    let (expected_width, expected_height) = env.window.size_unrotated_unscaled();
    let expected_width = expected_width as CGFloat;
    let expected_height = expected_height as CGFloat;

    let &UIViewHostObject { bounds, center, .. } = env.objc.borrow(view);

    if bounds.size.width != expected_width || bounds.size.height != expected_height {
        return None;
    }
    if center.x != expected_width / 2.0 || center.y != expected_height / 2.0 {
        return None;
    }

    Some(CGPoint {
        x: point.x - bounds.origin.x,
        y: point.y - bounds.origin.y,
    })
}

fn find_view_for_touch(env: &mut Environment, point: CGPoint) -> Option<id> {
    // FIXME: This is a massive hack that is only going to work for apps that
    // have a single view which handles all touch inputs. We should eventually
    // implement the proper responder chain.

    let ui_window_class = env.objc.get_known_class("UIWindow", &mut env.mem);
    // TODO: Can we avoid copying this somehow?
    let views = env.framework_state.uikit.ui_view.views.clone();
    for view in views {
        // There's no reason a UIWindow can't handle touch events, this is just
        // a hack specific to apps which don't do that.
        if msg![env; view isKindOfClass:ui_window_class] {
            continue;
        }

        // FIXME: This is an even bigger hack, it is assuming there is a single
        // view with the same size as the screen, and can't account for
        // the view hierarchy's effects on the co-ordinate system!
        if resolve_point_in_view(env, view, point).is_none() {
            continue;
        }

        log_dbg!("Picked view {:?} for touch event", view);
        return Some(view);
    }

    log!("Warning: touch event ignored, can't find appropriate view (FIXME)");
    None
}

/// [super::handle_events] will forward touch events to this function.
pub fn handle_event(env: &mut Environment, event: Event) {
    match event {
        Event::TouchDown(coords) => {
            if env.framework_state.uikit.ui_touch.current_touch.is_some() {
                log!("Warning: New touch initiated but current touch did not end yet, treating as movement.");
                return handle_event(env, Event::TouchMove(coords));
            }

            log_dbg!("Touch down: {:?}", coords);

            let location = CGPoint {
                x: coords.0,
                y: coords.1,
            };

            let Some(view) = find_view_for_touch(env, location) else {
                return;
            };

            // UIKit creates and drains autorelease pools when handling events.
            let pool: id = msg_class![env; NSAutoreleasePool new];

            // Note: if the emulator is heavily lagging, this timestamp is going
            // to be far off from the truth, since it should represent the
            // time when the event actually happened, not the time when the
            // event was dispatched. Maybe we'll need to fix this eventually.
            let timestamp: NSTimeInterval = msg_class![env; NSProcessInfo systemUptime];

            let new_touch: id = msg_class![env; UITouch alloc];
            retain(env, view);
            *env.objc.borrow_mut(new_touch) = UITouchHostObject {
                view,
                location,
                previous_location: location,
                timestamp,
            };
            autorelease(env, new_touch);

            env.framework_state.uikit.ui_touch.current_touch = Some(new_touch);
            retain(env, new_touch);

            let touches: id = msg_class![env; NSSet setWithObject:new_touch];
            let event = ui_event::new_event(env, touches, view);
            autorelease(env, event);

            log_dbg!(
                "Sending [{:?} touchesBegan:{:?} withEvent:{:?}]",
                view,
                touches,
                event
            );
            let _: () = msg![env; view touchesBegan:touches withEvent:event];

            release(env, pool);
        }
        Event::TouchMove(coords) => {
            let Some(touch) = env.framework_state.uikit.ui_touch.current_touch else {
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
        Event::TouchUp(coords) => {
            let Some(touch) = env.framework_state.uikit.ui_touch.current_touch else {
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

            let pool: id = msg_class![env; NSAutoreleasePool new];

            let touches: id = msg_class![env; NSSet setWithObject:touch];
            let event = ui_event::new_event(env, touches, view);
            autorelease(env, event);

            env.framework_state.uikit.ui_touch.current_touch = None;
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
