/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIAccelerometer`.
//!
//! Useful resources:
//! - [Apple's documentation for UIAcceleration](https://developer.apple.com/documentation/uikit/uiacceleration) has a really nice diagram of how the accelerometer axes relate to an iPhone.

use crate::frameworks::foundation::NSTimeInterval;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, ClassExports, HostObject,
    NSZonePtr, TrivialHostObject, SEL,
};
use crate::Environment;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct State {
    /// [UIAccelerometer sharedAccelerometer]
    shared_accelerometer: Option<id>,
    /// Something implementing UIAccelerometerDelegate, weak reference
    delegate: Option<id>,
    update_interval: Option<NSTimeInterval>,
    due_by: Option<Instant>,
}

type UIAccelerationValue = f64;

const DEFAULT_UPDATE_INTERVAL: f64 = 1.0 / 60.0;

struct UIAccelerationHostObject {
    x: UIAccelerationValue,
    y: UIAccelerationValue,
    z: UIAccelerationValue,
    timestamp: NSTimeInterval,
}
impl HostObject for UIAccelerationHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// This is a singleton.
@implementation UIAccelerometer: NSObject

+ (id)sharedAccelerometer {
    if let Some(accelerometer) =
        env.framework_state.uikit.ui_accelerometer.shared_accelerometer {
        accelerometer
    } else {
        let new = env.objc.alloc_static_object(
            this,
            Box::new(TrivialHostObject),
            &mut env.mem
        );
        env.framework_state.uikit.ui_accelerometer.shared_accelerometer = Some(new);
        new
   }
}
- (id)retain { this }
- (())release {}
- (id)autorelease { this }

// TODO: more accessors

- (id)delegate {
    env.framework_state.uikit.ui_accelerometer.delegate.unwrap_or(nil)
}
- (())setDelegate:(id)delegate {
    if delegate == nil {
        env.framework_state.uikit.ui_accelerometer.delegate = None;
    } else {
        env.framework_state.uikit.ui_accelerometer.delegate = Some(delegate);
        env.window().print_accelerometer_notice();
    }
}

- (NSTimeInterval)updateInterval {
    env.framework_state.uikit.ui_accelerometer.update_interval.unwrap_or(DEFAULT_UPDATE_INTERVAL)
}
- (())setUpdateInterval:(NSTimeInterval)interval {
    // The system can limit this value, and must (some apps pass 0 and this can
    // cause a division-by-zero. 60Hz has been chosen here to match 60fps.
    let interval = interval.max(1.0 / 60.0);
    env.framework_state.uikit.ui_accelerometer.update_interval = Some(interval);
}

@end

@implementation UIAcceleration: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIAccelerationHostObject {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        timestamp: 0.0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (UIAccelerationValue)x {
    env.objc.borrow::<UIAccelerationHostObject>(this).x
}
- (UIAccelerationValue)y {
    env.objc.borrow::<UIAccelerationHostObject>(this).y
}
- (UIAccelerationValue)z {
    env.objc.borrow::<UIAccelerationHostObject>(this).z
}
- (NSTimeInterval)timestamp {
    env.objc.borrow::<UIAccelerationHostObject>(this).timestamp
}

@end

};

/// For use by `NSRunLoop` via [super::handle_events]: check if an accelerometer
/// update is due and send one if appropriate.
///
/// Returns the time an accelerometer update is due, if any.
pub(super) fn handle_accelerometer(env: &mut Environment) -> Option<Instant> {
    let state = &mut env.framework_state.uikit.ui_accelerometer;

    let delegate = state.delegate?;

    let ns_interval = state.update_interval.unwrap_or(DEFAULT_UPDATE_INTERVAL);
    let rust_interval = Duration::from_secs_f64(ns_interval);

    let now = Instant::now();
    let new_due_by = if let Some(due_by) = state.due_by {
        if due_by > now {
            return Some(due_by);
        }

        // See NSTimer implementation for a discussion of what this does.
        // I don't know if iPhone OS uses this approach for accelerometer
        // updates, but there's no obvious reason not to.
        let overdue_by = now.duration_since(due_by);
        // TODO: Use `.div_duration_f64()` once that is stabilized.
        let advance_by = (overdue_by.as_secs_f64() / ns_interval).max(1.0).ceil();
        assert!(advance_by == (advance_by as u32) as f64);
        let advance_by = advance_by as u32;
        if advance_by > 1 {
            log_dbg!("Warning: Accelerometer is lagging. It is overdue by {}s and has missed {} interval(s)!", overdue_by.as_secs_f64(), advance_by - 1);
        }
        let advance_by = rust_interval.checked_mul(advance_by).unwrap();
        Some(due_by.checked_add(advance_by).unwrap())
    } else {
        // In Resident Evil 4 the delegate is set before it fully initializes.
        // If the first message is sent immediately, it crashes.
        // This change prevents it by not sending the first message until the
        // time interval first passes
        let due_by = now.checked_add(rust_interval).unwrap();
        let new_due_by = Some(due_by);
        if due_by > now {
            return new_due_by;
        }
        new_due_by
    };
    state.due_by = new_due_by;

    // UIKit creates and drains autorelease pools when handling events.
    let pool: id = msg_class![env; NSAutoreleasePool new];

    let (x, y, z) = env.window().get_acceleration(&env.options);
    let timestamp: NSTimeInterval = msg_class![env; NSProcessInfo systemUptime];
    let acceleration: id = msg_class![env; UIAcceleration alloc];
    *env.objc.borrow_mut(acceleration) = UIAccelerationHostObject {
        x: x.into(),
        y: y.into(),
        z: z.into(),
        timestamp,
    };
    autorelease(env, acceleration);

    let accelerometer: id = msg_class![env; UIAccelerometer sharedAccelerometer];

    log_dbg!(
        "Sending [{:?} accelerometer:{:?} didAccelerate:{:?}]",
        delegate,
        accelerometer,
        acceleration,
    );
    let sel: SEL = env
        .objc
        .register_host_selector("accelerometer:didAccelerate:".to_string(), &mut env.mem);
    let responds: bool = msg![env; delegate respondsToSelector:sel];
    if responds {
        let _: () = msg![env; delegate accelerometer:accelerometer
                                       didAccelerate:acceleration];
    }

    release(env, pool);

    new_due_by
}
