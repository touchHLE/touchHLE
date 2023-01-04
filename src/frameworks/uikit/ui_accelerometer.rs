//! `UIAccelerometer`.

use crate::frameworks::foundation::NSTimeInterval;
use crate::objc::{id, nil, objc_classes, ClassExports, TrivialHostObject};

#[derive(Default)]
pub struct State {
    /// [UIAccelerometer sharedAccelerometer]
    shared_accelerometer: Option<id>,
    /// Something implementing UIAccelerometerDelegate, weak reference
    delegate: Option<id>,
    update_interval: Option<NSTimeInterval>,
}

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
        log!("TODO: Send accelerometer events to delegate {:?}", delegate);
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

};
