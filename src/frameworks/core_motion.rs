/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Core Motion framework.

use crate::dyld::HostDylib;
use crate::objc::{objc_classes, ClassExports};

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/CoreMotion.framework/CoreMotion",
    aliases: &[],
    class_exports: &[CLASSES],
    constant_exports: &[],
    function_exports: &[],
};

const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CMMotionManager: NSObject

- (bool)isGyroAvailable {
    // It could make sense to implement gyroscope support at least for Android.
    log!("TODO: [(CMMotionManager *){:?} isGyroAvailable] -> false", this);
    false
}
- (bool)isDeviceMotionAvailable {
    log!("TODO: [(CMMotionManager *){:?} isDeviceMotionAvailable] -> false", this);
    // According to docs, this is functionally equivalent to `isGyroAvailable`
    // method. (All devices have accelerometer, but only some do have gyro).
    false
}

@end

};
