/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UITouch`.

use super::ui_event;
use super::ui_gesture_recognizer::{
    fire_targets, UIGestureRecognizerHostObject, UIGestureRecognizerStatePossible,
    UIGestureRecognizerStateRecognized, UISwipeGestureRecognizerDirectionDown,
    UISwipeGestureRecognizerDirectionLeft, UISwipeGestureRecognizerDirectionRight,
    UISwipeGestureRecognizerDirectionUp,
};
use crate::frameworks::core_graphics::{CGPoint, CGRect};
use crate::frameworks::foundation::{NSInteger, NSTimeInterval, NSUInteger};
use crate::mem::MutVoidPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, msg_send_no_type_checking, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::window::{Coords, Event, FingerId};
use crate::Environment;
use std::collections::hash_map::{Entry, HashMap};
use std::collections::HashSet;

pub type UITouchPhase = NSInteger;
pub const UITouchPhaseBegan: UITouchPhase = 0;
pub const UITouchPhaseMoved: UITouchPhase = 1;
pub const UITouchPhaseStationary: UITouchPhase = 2;
pub const UITouchPhaseEnded: UITouchPhase = 3;

#[derive(Default)]
pub struct State {
    pub current_touches: HashMap<FingerId, id>,
}

#[derive(Default)]
pub(super) struct UITouchHostObject {
    pub(super) view: id,
    pub(super) window: id,
    location: CGPoint,
    previous_location: CGPoint,
    /// Where this touch first landed (in window/screen coordinates). Used to
    /// compute the total displacement for swipe gesture recognition.
    start_location: CGPoint,
    timestamp: NSTimeInterval,
    phase: UITouchPhase,
}
impl HostObject for UITouchHostObject {}

fn touchhle_should_use_landscape_touch_remap(env: &Environment) -> bool {
    match env.bundle.bundle_identifier() {
        // Confirmed landscape Source/Cocos games.
        "at.source.veggie1" | "at.source.potato3D" | "at.source.potpan" => true,

        // TomatoZombie is native portrait.
        "at.source.tomzom" => false,

        // Manual override for testing.
        _ => std::env::var_os("TOUCHHLE_TOUCH_LOCATION_PORTRAIT_TO_LANDSCAPE").is_some(),
    }
}


fn should_remap_touch_location_for_view(env: &mut Environment, view: id) -> bool {
    match env.bundle.bundle_identifier() {
        // Confirmed/wip landscape Source/Cocos games.
        "at.source.veggie1" | "at.source.potato3D" | "at.source.potpan" => return true,

        // TomatoZombie is native portrait.
        "at.source.tomzom" => return false,

        _ => {}
    }

    if std::env::var_os("TOUCHHLE_TOUCH_LOCATION_PORTRAIT_TO_LANDSCAPE").is_some() {
        return true;
    }

    if view == nil {
        return false;
    }

    let view_class: crate::objc::Class = msg![env; view class];
    let class_name = env.objc.get_class_name(view_class);

    matches!(
        class_name,
        "CCGLView" | "EAGLView" | "CCEAGLView" | "GLKView"
    )
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UITouch: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UITouchHostObject {
        view: nil,
        window: nil,
        location: CGPoint { x: 0.0, y: 0.0 },
        previous_location: CGPoint { x: 0.0, y: 0.0 },
        start_location: CGPoint { x: 0.0, y: 0.0 },
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

- (CGPoint)locationInView:(id)that_view {
    let &UITouchHostObject { location, window, .. } = env.objc.borrow(this);
    let location_in_window: CGPoint = msg![env; window
        convertPoint:location fromWindow:nil];
    let mut result: CGPoint = if that_view == nil {
        location_in_window
    } else {
        msg![env;
        that_view convertPoint:location_in_window fromView:window]
    };

    if touchhle_should_use_landscape_touch_remap(env) {
        // Important: this happens AFTER UIKit hit-testing. The touch can still
        // hit the 320x480 EAGLView, but the game receives a landscape-style
        // 480x320 point from locationInView:, which is what PotatoGold's
        // custom OpenGL menu widgets appear to expect.
        let old_x = result.x;
        let old_y = result.y;

        // CaptainTomato / Cocos2D landscape fix.
        // UIKit hit-testing sees a 320x480 portrait CCGLView, but the game's
        // Cocos/OpenGL menu expects 480x320 landscape coordinates.
        //
        // Default to LandscapeRight. Set TOUCHHLE_TOUCH_LANDSCAPE_LEFT=1 if
        // taps are mirrored to the wrong side.
        // CaptainTomato fix:
        // Do NOT rotate. UIKit gives 320x480-ish touch coords, while the game
        // wants 480x320-ish coords. The image is already visually rotated by
        // the emulator/window path, so a 90-degree touch rotation makes the
        // speaker/pause button hit the play button.
        let mode = std::env::var("TOUCHHLE_TOUCH_MODE").unwrap_or_else(|_| {
            match env.bundle.bundle_identifier() {
                // CaptainTomato's confirmed working mode.
                "at.source.veggie1" => "scale".to_string(),

                // Potato Story / Potato Panic use the same scale-only touch
                // behavior class as CaptainTomato. Do not rotate; rotation
                // makes buttons miss entirely.
                "at.source.potato3D" | "at.source.potpan" => "scale".to_string(),

                // Other remapped apps default to the old safe behavior.
                _ => "scale".to_string(),
            }
        });

        let (mut new_x, mut new_y) = match mode.as_str() {
            // LandscapeRight: portrait window -> 480x320 landscape coords.
            "right" => (old_y, 320.0 - old_x),

            // Same rotation as right, but mirrored horizontally.
            "right-flip-x" => (480.0 - old_y, 320.0 - old_x),

            // LandscapeLeft, kept for quick testing if right is mirrored.
            "left" => (480.0 - old_y, old_x),

            // Confirmed CaptainTomato behavior.
            "scale" | _ => (
                old_x * (480.0 / 320.0),
                old_y * (320.0 / 480.0),
            ),
        };

        if let Ok(offset) = std::env::var("TOUCHHLE_TOUCH_LOCATION_X_OFFSET") {
            if let Ok(offset) = offset.parse::<f32>() {
                new_x += offset;
            }
        }
        if let Ok(offset) = std::env::var("TOUCHHLE_TOUCH_LOCATION_Y_OFFSET") {
            if let Ok(offset) = offset.parse::<f32>() {
                new_y += offset;
            }
        }

        log!(
            "UITouch landscape remap: ({:.1}, {:.1}) -> ({:.1}, {:.1})",
            old_x,
            old_y,
            new_x,
            new_y
        );

        result = CGPoint {
            x: new_x.clamp(0.0, 479.0),
            y: new_y.clamp(0.0, 319.0),
        };
    }

    result
}

- (CGPoint)previousLocationInView:(id)that_view {
    let &UITouchHostObject { previous_location, window, .. } = env.objc.borrow(this);
    let location_in_window: CGPoint = msg![env; window
        convertPoint:previous_location fromWindow:nil];
    let mut result: CGPoint = if that_view == nil {
        location_in_window
    } else {
        msg![env;
        that_view convertPoint:location_in_window fromView:window]
    };

    if touchhle_should_use_landscape_touch_remap(env) {
        let old_x = result.x;
        let old_y = result.y;

        // CaptainTomato / Cocos2D landscape fix.
        // UIKit hit-testing sees a 320x480 portrait CCGLView, but the game's
        // Cocos/OpenGL menu expects 480x320 landscape coordinates.
        //
        // Default to LandscapeRight. Set TOUCHHLE_TOUCH_LANDSCAPE_LEFT=1 if
        // taps are mirrored to the wrong side.
        // CaptainTomato fix:
        // Do NOT rotate. UIKit gives 320x480-ish touch coords, while the game
        // wants 480x320-ish coords. The image is already visually rotated by
        // the emulator/window path, so a 90-degree touch rotation makes the
        // speaker/pause button hit the play button.
        let mut new_x = old_x * (480.0 / 320.0);
        let mut new_y = old_y * (320.0 / 480.0);

        if let Ok(offset) = std::env::var("TOUCHHLE_TOUCH_LOCATION_X_OFFSET") {
            if let Ok(offset) = offset.parse::<f32>() {
                new_x += offset;
            }
        }
        if let Ok(offset) = std::env::var("TOUCHHLE_TOUCH_LOCATION_Y_OFFSET") {
            if let Ok(offset) = offset.parse::<f32>() {
                new_y += offset;
            }
        }

        log!(
            "UITouch landscape remap: ({:.1}, {:.1}) -> ({:.1}, {:.1})",
            old_x,
            old_y,
            new_x,
            new_y
        );

        result = CGPoint {
            x: new_x.clamp(0.0, 479.0),
            y: new_y.clamp(0.0, 319.0),
        };
    }

    result
}

- (id)view {
    env.objc.borrow::<UITouchHostObject>(this).view
}

- (id)window {
    env.objc.borrow::<UITouchHostObject>(this).window
}

- (NSTimeInterval)timestamp {
    env.objc.borrow::<UITouchHostObject>(this).timestamp
}

- (NSUInteger)tapCount {
    1
}

- (UITouchPhase)phase {
    env.objc.borrow::<UITouchHostObject>(this).phase
}

@end

};

pub fn handle_event(env: &mut Environment, event: Event) {
    let touch_ids: Vec<id> = env
        .framework_state
        .uikit
        .ui_touch
        .current_touches
        .values()
        .cloned()
        .collect();
    for touch in touch_ids {
        env.objc.borrow_mut::<UITouchHostObject>(touch).phase = UITouchPhaseStationary;
    }
    match event {
        Event::TouchesDown(map) => handle_touches_down(env, map),
        Event::TouchesMove(map) => handle_touches_move(env, map),
        Event::TouchesUp(map) => handle_touches_up(env, map),
        other => {
            // ui_touch::handle_event only ever wants touch events; non-touch
            // events are filtered out before getting here. Log instead of
            // panicking the host if that contract is ever violated.
            log!(
                "Warning: ui_touch::handle_event: unsupported event {:?}; ignored.",
                other
            );
        }
    }
}

fn handle_touches_down(env: &mut Environment, map: HashMap<FingerId, Coords>) {
    let pool: id = msg_class![env;
        NSAutoreleasePool new];

    let timestamp: NSTimeInterval = {
        let process_info = msg_class![env; NSProcessInfo processInfo];
        msg![env; process_info systemUptime]
    };

    let touches: id = msg_class![env;
        NSMutableSet
        allocWithZone:(MutVoidPtr::null())];

    for (finger_id, coords) in map {
        if env
            .framework_state
            .uikit
            .ui_touch
            .current_touches
            .contains_key(&finger_id)
        {
            log!(
                "Warning: New touch {:?} initiated but old one exists.",
                finger_id
            );
            return handle_touches_move(env, HashMap::from([(finger_id, coords)]));
        }

        let location = CGPoint {
            x: coords.0,
            y: coords.1,
        };
        let new_touch: id = msg_class![env; UITouch alloc];
        *env.objc.borrow_mut(new_touch) = UITouchHostObject {
            view: nil,
            window: nil,
            location,
            previous_location: location,
            start_location: location,
            timestamp,
            phase: UITouchPhaseBegan,
        };
        autorelease(env, new_touch);

        let _: () = msg![env; touches addObject:new_touch];
        env.framework_state
            .uikit
            .ui_touch
            .current_touches
            .insert(finger_id, new_touch);
        retain(env, new_touch);
    }

    let all_touches_set: id = msg_class![env; NSMutableSet
        allocWithZone:(MutVoidPtr::null())];
    let existing_touches: Vec<id> = env
        .framework_state
        .uikit
        .ui_touch
        .current_touches
        .values()
        .cloned()
        .collect();
    for touch in existing_touches {
        let _: () = msg![env; all_touches_set addObject:touch];
    }

    let event = ui_event::new_event(env, all_touches_set);
    autorelease(env, event);
    let views_with_existing_touches: HashSet<id> = env
        .framework_state
        .uikit
        .ui_touch
        .current_touches
        .values()
        .map(|&touch| env.objc.borrow::<UITouchHostObject>(touch).view)
        .collect();
    let mut view_touches: HashMap<id, id> = HashMap::new();
    let touches_arr: id = msg![env; touches allObjects];
    let touches_count: NSUInteger = msg![env;
        touches_arr count];

    for i in 0..touches_count {
        let touch: id = msg![env;
            touches_arr objectAtIndex:i];
        let &UITouchHostObject { location, .. } = env.objc.borrow(touch);

        let windows = env.framework_state.uikit.ui_view.ui_window.windows.clone();
        let found_window = windows.iter().rev().find_map(|&window| {
            let location_in_window: CGPoint = msg![env; window
                convertPoint:location fromWindow:nil];
            if msg![env; window pointInside:location_in_window withEvent:event] {
                Some((window, location_in_window))
            } else {
                None
            }
        });
        // SUPER HACK: Если окно отвергло касание, силой отправляем его в
        // главное окно!
        let Some((window, location_in_window)) = found_window.or_else(|| {
            windows.last().map(|&window| {
                let lx = location.x;
                let ly = location.y;
                log_dbg!(
                    "SUPER HACK: Forcing rejected touch at ({}, {}) into window",
                    lx,
                    ly
                );
                let loc: CGPoint = msg![env; window convertPoint:location fromWindow:nil];
                (window, loc)
            })
        }) else {
            let lx = location.x;
            let ly = location.y;
            log!(
                "Couldn't find ANY window for touch at ({}, {}), discarding",
                lx,
                ly
            );
            continue;
        };
        let mut view: id = msg![env; window hitTest:location_in_window withEvent:event];

        if view != nil {
            let view_class: crate::objc::Class = msg![env; view class];
            let class_name = env.objc.get_class_name(view_class).to_owned();

            if class_name == "MBProgressHUD" {
                log!("Touch hit MBProgressHUD; keeping HUD as touch target");
            }
        }

        if view == nil {
            log_dbg!("SUPER HACK: hitTest failed, forcing touch directly into the window");
            view = window;
        } else {
            let f: CGRect = msg![env;
                view frame];
            let view_class: crate::objc::Class = msg![env; view class];
            let class_name = env.objc.get_class_name(view_class).to_owned();
            let lx = location_in_window.x;
            let ly = location_in_window.y;
            log_dbg!(
                "Touch at ({}, {}) hit {} {:?} with frame {:?}",
                lx,
                ly,
                class_name,
                view,
                f,
            );
        }

        let is_multi_touch_enabled: bool = msg![env; view isMultipleTouchEnabled];
        if !is_multi_touch_enabled
            && (view_touches.contains_key(&view) || views_with_existing_touches.contains(&view))
        {
            let stuck: Vec<FingerId> = env
                .framework_state
                .uikit
                .ui_touch
                .current_touches
                .iter()
                .filter(|(_, &t)| {
                    env.objc.borrow::<UITouchHostObject>(t).view == view && t != touch
                })
                .map(|(&fid, _)| fid)
                .collect();
            if !stuck.is_empty() {
                for fid in stuck {
                    if let Some(t) = env
                        .framework_state
                        .uikit
                        .ui_touch
                        .current_touches
                        .remove(&fid)
                    {
                        release(env, t);
                    }
                }
            } else {
                continue;
            }
        }

        if let Entry::Vacant(e) = view_touches.entry(view) {
            let s: id = msg_class![env;
                NSMutableSet
                allocWithZone:(MutVoidPtr::null())];
            e.insert(s);
        }
        let v_set: id = *view_touches.get(&view).unwrap();
        let _: () = msg![env; v_set addObject:touch];
        retain(env, view);
        retain(env, window);
        {
            let t_obj = env.objc.borrow_mut::<UITouchHostObject>(touch);
            t_obj.view = view;
            t_obj.window = window;
            t_obj.location = location;
        }
    }

    for (view, v_set) in view_touches {
        let _: () = msg![env;
            view touchesBegan:v_set withEvent:event];
    }
    release(env, pool);
}

fn handle_touches_move(env: &mut Environment, map: HashMap<FingerId, Coords>) {
    let pool: id = msg_class![env;
        NSAutoreleasePool new];
    let timestamp: NSTimeInterval = {
        let pi = msg_class![env; NSProcessInfo processInfo];
        msg![env; pi systemUptime]
    };

    let mut view_touches: HashMap<id, id> = HashMap::new();
    for (finger_id, coords) in map {
        let Some(&touch) = env
            .framework_state
            .uikit
            .ui_touch
            .current_touches
            .get(&finger_id)
        else {
            continue;
        };
        let location = CGPoint {
            x: coords.0,
            y: coords.1,
        };
        let view = env.objc.borrow::<UITouchHostObject>(touch).view;
        let host = env.objc.borrow_mut::<UITouchHostObject>(touch);
        if host.location == location {
            continue;
        }
        host.previous_location = host.location;
        host.location = location;
        host.timestamp = timestamp;
        host.phase = UITouchPhaseMoved;

        if let Entry::Vacant(e) = view_touches.entry(view) {
            let s: id = msg_class![env;
                NSMutableSet
                allocWithZone:(MutVoidPtr::null())];
            e.insert(s);
        }
        let v_set: id = *view_touches.get(&view).unwrap();
        let _: () = msg![env; v_set addObject:touch];
    }

    let all_touches_set: id = msg_class![env; NSMutableSet
        allocWithZone:(MutVoidPtr::null())];
    let existing: Vec<id> = env
        .framework_state
        .uikit
        .ui_touch
        .current_touches
        .values()
        .cloned()
        .collect();
    for t in existing {
        let _: () = msg![env; all_touches_set addObject:t];
    }

    let event = ui_event::new_event(env, all_touches_set);
    autorelease(env, event);
    for (view, v_set) in view_touches {
        let _: () = msg![env;
            view touchesMoved:v_set withEvent:event];
    }
    release(env, pool);
}


fn ultrahle_minionjump_drain_pending_callback(env: &mut Environment, select_only: bool) {
    if !matches!(
        env.bundle.bundle_identifier(),
        "com.apprisetec9.minionjump" | "com.risinghighapps.kingdomprincepro"
    ) {
        return;
    }

    let Some(target_raw) = std::env::var("ULTRAHLE_MINIONJUMP_PENDING_TARGET")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
    else {
        return;
    };

    let Some(sel_raw) = std::env::var("ULTRAHLE_MINIONJUMP_PENDING_SEL")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
    else {
        return;
    };

    let sender_raw = std::env::var("ULTRAHLE_MINIONJUMP_PENDING_SENDER")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0);

    let callback_name = std::env::var("ULTRAHLE_MINIONJUMP_PENDING_CALLBACK")
        .unwrap_or_else(|_| "<unknown>".to_string());

    let stage =
        std::env::var("ULTRAHLE_MINIONJUMP_PENDING_STAGE").unwrap_or_else(|_| "0".to_string());

    let is_level_select = callback_name == "selectLVAction:";
    if select_only != is_level_select {
        return;
    }

    std::env::remove_var("ULTRAHLE_MINIONJUMP_PENDING_TARGET");
    std::env::remove_var("ULTRAHLE_MINIONJUMP_PENDING_SEL");
    std::env::remove_var("ULTRAHLE_MINIONJUMP_PENDING_SENDER");
    std::env::remove_var("ULTRAHLE_MINIONJUMP_PENDING_CALLBACK");
    std::env::remove_var("ULTRAHLE_MINIONJUMP_PENDING_STAGE");

    if target_raw == 0 || sel_raw == 0 {
        return;
    }

    let target_id = id::from_bits(target_raw);
    let sender = id::from_bits(sender_raw);
    let callback_sel_ptr = crate::mem::ConstPtr::<u8>::from_bits(sel_raw);
    let callback_sel: crate::objc::SEL = unsafe { std::mem::transmute(callback_sel_ptr) };

    log!(
        "UltraHLE MinionJump: draining pending callback selector={} target={:?} sender={:?} stage={} select_only={}",
        callback_name,
        target_id,
        sender,
        stage,
        select_only
    );

    if callback_name.ends_with(':') {
        let _: () = msg_send_no_type_checking(env, (target_id, callback_sel, sender));
    } else {
        let _: () = msg_send_no_type_checking(env, (target_id, callback_sel));
    }
}

fn handle_touches_up(env: &mut Environment, map: HashMap<FingerId, Coords>) {
    let pool: id = msg_class![env;
        NSAutoreleasePool new];
    let timestamp: NSTimeInterval = {
        let pi = msg_class![env; NSProcessInfo processInfo];
        msg![env; pi systemUptime]
    };

    let touches: id = msg_class![env;
        NSMutableSet
        allocWithZone:(MutVoidPtr::null())];
    let all_touches_set: id = msg_class![env;
        NSMutableSet
        allocWithZone:(MutVoidPtr::null())];
    let existing: Vec<id> = env
        .framework_state
        .uikit
        .ui_touch
        .current_touches
        .values()
        .cloned()
        .collect();
    for t in existing {
        let _: () = msg![env; all_touches_set addObject:t];
    }

    let mut view_touches: HashMap<id, id> = HashMap::new();
    // (view, start_location, end_location) for swipe gesture detection.
    let mut swipe_candidates: Vec<(id, CGPoint, CGPoint)> = Vec::new();
    for (finger_id, coords) in map {
        let Some(&touch) = env
            .framework_state
            .uikit
            .ui_touch
            .current_touches
            .get(&finger_id)
        else {
            continue;
        };
        let location = CGPoint {
            x: coords.0,
            y: coords.1,
        };
        let view = env.objc.borrow::<UITouchHostObject>(touch).view;
        let start_location = env.objc.borrow::<UITouchHostObject>(touch).start_location;
        {
            let host = env.objc.borrow_mut::<UITouchHostObject>(touch);
            host.previous_location = host.location;
            host.location = location;
            host.timestamp = timestamp;
            host.phase = UITouchPhaseEnded;
        }

        if view != nil {
            swipe_candidates.push((view, start_location, location));
        }

        let _: () = msg![env;
            touches addObject:touch];

        if let Entry::Vacant(e) = view_touches.entry(view) {
            let s: id = msg_class![env;
                NSMutableSet
                allocWithZone:(MutVoidPtr::null())];
            e.insert(s);
        }
        let v_set: id = *view_touches.get(&view).unwrap();
        let _: () = msg![env; v_set addObject:touch];
        env.framework_state
            .uikit
            .ui_touch
            .current_touches
            .remove(&finger_id);
        release(env, touch);
    }

    let event = ui_event::new_event(env, all_touches_set);
    autorelease(env, event);
    for (view, v_set) in view_touches {
        let _: () = msg![env;
            view touchesEnded:v_set withEvent:event];
    }
    for (view, start, end) in swipe_candidates {
        recognize_swipes(env, view, start, end);
    }

    // ULTRAHLE_MINIONJUMP_DRAIN_SELECT_BEGIN
    ultrahle_minionjump_drain_pending_callback(env, true);
    // ULTRAHLE_MINIONJUMP_DRAIN_SELECT_END
    // ULTRAHLE_MINIONJUMP_DRAIN_POST_BEGIN
    ultrahle_minionjump_drain_pending_callback(env, false);
    // ULTRAHLE_MINIONJUMP_DRAIN_POST_END

    release(env, pool);
}

/// Minimal `UISwipeGestureRecognizer` support.
///
/// HyperHLE delivers raw touches directly to views and does not run the full
/// iOS gesture-recognition state machine. Many apps, however, attach a
/// `UISwipeGestureRecognizer` to a view and rely on it firing — without this
/// the swipe simply never happens (raw `touchesMoved:` still works, which is
/// why plain taps/drags were fine but swipes were dead).
///
/// When a touch ends we compute its straight-line delta from where it began.
/// If it moved far enough, fast/clean enough to count as a swipe, we look at
/// the view's attached recognizers and fire any `UISwipeGestureRecognizer`
/// whose `direction` mask matches the dominant axis of the movement.
fn recognize_swipes(env: &mut Environment, view: id, start: CGPoint, end: CGPoint) {
    // Apple's UIKit uses a swipe threshold in the tens of points; a touch that
    // moved less than this is a tap, not a swipe.
    const MIN_SWIPE_DISTANCE: f32 = 24.0;
    // The motion must be reasonably axis-aligned to be a swipe (otherwise it's
    // a free-form drag / pan). Require the dominant axis to dominate by 2x.
    const AXIS_DOMINANCE: f32 = 2.0;

    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let adx = dx.abs();
    let ady = dy.abs();

    if adx < MIN_SWIPE_DISTANCE && ady < MIN_SWIPE_DISTANCE {
        return;
    }

    // Determine the swipe direction from the dominant axis.
    let detected_direction: NSInteger = if adx >= ady {
        if adx < ady * AXIS_DOMINANCE && ady >= MIN_SWIPE_DISTANCE {
            // Too diagonal to be a clean swipe.
            return;
        }
        if dx > 0.0 {
            UISwipeGestureRecognizerDirectionRight
        } else {
            UISwipeGestureRecognizerDirectionLeft
        }
    } else {
        if ady < adx * AXIS_DOMINANCE && adx >= MIN_SWIPE_DISTANCE {
            return;
        }
        if dy > 0.0 {
            UISwipeGestureRecognizerDirectionDown
        } else {
            UISwipeGestureRecognizerDirectionUp
        }
    };

    // In real UIKit a gesture recognizer attached to *any* view in the hit
    // view's ancestor chain can recognize the gesture, not just the leaf view
    // the touch landed on. Games very commonly attach a
    // `UISwipeGestureRecognizer` to a container/superview (or the window's
    // root view) rather than the innermost `EAGLView` that actually gets hit.
    // Only checking the leaf view meant those swipes silently never fired.
    // Walk up the superview chain and fire matching recognizers on each view.
    let mut current = view;
    while current != nil {
        fire_matching_swipes(env, current, detected_direction);
        current = msg![env; current superview];
    }
}

/// Fire any enabled `UISwipeGestureRecognizer` attached to `view` whose
/// `direction` mask matches `detected_direction`.
fn fire_matching_swipes(env: &mut Environment, view: id, detected_direction: NSInteger) {
    // `gestureRecognizers` returns an autoreleased NSArray of recognizer ids.
    let recognizers: id = msg![env; view gestureRecognizers];
    if recognizers == nil {
        return;
    }
    let count: NSUInteger = msg![env; recognizers count];
    let swipe_class = env
        .objc
        .get_known_class("UISwipeGestureRecognizer", &mut env.mem);
    for i in 0..count {
        let recognizer: id = msg![env; recognizers objectAtIndex:i];
        if recognizer == nil {
            continue;
        }
        let cls: crate::objc::Class = msg![env; recognizer class];
        if !env.objc.class_is_subclass_of(cls, swipe_class) {
            continue;
        }
        let enabled: bool = msg![env; recognizer isEnabled];
        if !enabled {
            continue;
        }
        let mask: NSInteger = msg![env; recognizer direction];
        // `direction` is a bitmask; the recognizer fires if our detected
        // direction is one of the directions it is configured for.
        if mask & detected_direction == 0 {
            continue;
        }
        // Transition the recognizer to the recognized state and fire its
        // target-action pairs, mirroring real UIKit behaviour.
        {
            let host = env
                .objc
                .borrow_mut::<UIGestureRecognizerHostObject>(recognizer);
            host.state = UIGestureRecognizerStateRecognized;
        }
        fire_targets(env, recognizer);
        // Reset back to possible for the next gesture.
        let host = env
            .objc
            .borrow_mut::<UIGestureRecognizerHostObject>(recognizer);
        host.state = UIGestureRecognizerStatePossible;
    }
}
