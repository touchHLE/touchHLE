//! `UIAccelerometer`.
//!
//! Useful resources:
//! - [Apple's documentation for UIAcceleration](https://developer.apple.com/documentation/uikit/uiacceleration) has a really nice diagram of how the accelerometer axes relate to an iPhone.

use crate::frameworks::foundation::NSTimeInterval;
use crate::mem::MutVoidPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, ClassExports, HostObject,
    TrivialHostObject,
};
use crate::window::Matrix;
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
        log!("This app uses the accelerometer.");
        if env.window.have_controllers() {
            log!("Please connect a controller with an analog stick for accelerometer simulation.");
        } else {
            log!("Your connected controller's analog stick will be used for accelerometer simulation.");
        }
    }
}

- (NSTimeInterval)updateInterval {
    // TODO: return some reasonable default value
    env.framework_state.uikit.ui_accelerometer.update_interval.unwrap()
}
- (())setUpdateInterval:(NSTimeInterval)interval {
    env.framework_state.uikit.ui_accelerometer.update_interval = Some(interval);
}

@end

@implementation UIAcceleration: NSObject

+ (id)allocWithZone:(MutVoidPtr)_zone {
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
pub(super) fn handle_accelerometer(env: &mut Environment) {
    let state = &mut env.framework_state.uikit.ui_accelerometer;

    let Some(delegate) = state.delegate else {
        return;
    };

    // TODO: use some reasonable default value
    let ns_interval = state.update_interval.unwrap();
    let rust_interval = Duration::from_secs_f64(ns_interval);

    let now = Instant::now();
    if let Some(due_by) = state.due_by {
        if due_by > now {
            return;
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
            log!("Warning: Accelerometer is lagging. It is overdue by {}s and has missed {} interval(s)!", overdue_by.as_secs_f64(), advance_by - 1);
        }
        let advance_by = rust_interval.checked_mul(advance_by).unwrap();
        state.due_by = Some(due_by.checked_add(advance_by).unwrap());
    } else {
        state.due_by = Some(now.checked_add(rust_interval).unwrap());
    }

    // Get analog stick inputs. The range is [-1, 1] on each axis.
    let (controller_x, controller_y) = env.window.get_controller_sticks();

    // Let's simulate tilting the device based on the analog stick inputs.
    //
    // If an iPhone is lying flat on its back, level with the ground, and it is
    // on Earth, the accelerometer will report approximately (0, 0, -1).
    // The acceleration x and y axes are aligned with the screen's x and y axes.
    // +x points to the right of the screen, +y points to the top of the
    // of the screen, and +z points away from the screen. In the example
    // scenario, the z axis is parallel to gravity.

    let gravity: [f32; 3] = [0.0, 0.0, -1.0];

    let neutral_x = env.options.x_tilt_offset.to_radians();
    let neutral_y = env.options.y_tilt_offset.to_radians();
    let x_rotation_range = env.options.x_tilt_range.to_radians() / 2.0;
    let y_rotation_range = env.options.y_tilt_range.to_radians() / 2.0;
    // (x, y) are swapped and inverted because the controller Y axis usually
    // corresponds to forward/backward movement, but rotating about the Y axis
    // means tilting the device left/right, and gravity points in the opposite
    // direction of the device's tilt.
    let x_rotation = neutral_x - x_rotation_range * controller_y;
    let y_rotation = neutral_y - y_rotation_range * controller_x;

    let matrix = Matrix::<3>::y_rotation(y_rotation).multiply(&Matrix::<3>::x_rotation(x_rotation));
    let rotated_gravity = matrix.transform(gravity);

    // UIKit creates and drains autorelease pools when handling events.
    let pool: id = msg_class![env; NSAutoreleasePool new];

    let timestamp: NSTimeInterval = msg_class![env; NSProcessInfo systemUptime];
    let acceleration: id = msg_class![env; UIAcceleration alloc];
    *env.objc.borrow_mut(acceleration) = UIAccelerationHostObject {
        x: rotated_gravity[0].into(),
        y: rotated_gravity[1].into(),
        z: rotated_gravity[2].into(),
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
    let _: () = msg![env; delegate accelerometer:accelerometer
                                   didAccelerate:acceleration];

    release(env, pool);
}
