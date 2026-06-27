/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Core Motion framework.
//!
//! Per Apple's CoreMotion Framework Reference (iOS 4.0+, available since 2.0
//! as UIAccelerometer):
//!
//! - CMMotionManager: The gateway object for accelerometer, gyroscope, and
//!   device-motion services.
//! - CMAccelerometerData: Contains a single accelerometer reading
//!   (CMAcceleration with x, y, z in G-force units).
//! - CMGyroData: Contains a single gyroscope reading (CMRotationRate with
//!   x, y, z in radians/second).
//! - CMDeviceMotion: Contains processed device-motion data combining
//!   accelerometer + gyroscope: attitude, rotationRate, gravity,
//!   userAcceleration.
//! - CMAcceleration: struct { x: f64, y: f64, z: f64 }
//! - CMRotationRate: struct { x: f64, y: f64, z: f64 }
//!
//! This implementation integrates with the SDL sensor subsystem (via the
//! window's accelerometer reading) to provide real accelerometer data when
//! available, and falls back to simulated gravity (0, 0, -1) otherwise.

use crate::abi::{impl_GuestRet_for_large_struct, GuestArg};
use crate::dyld::HostDylib;
use crate::objc::{
    autorelease, id, msg_class, nil, objc_classes, ClassExports, HostObject, NSZonePtr,
};
use crate::Environment;
use std::time::Instant;

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/CoreMotion.framework/CoreMotion",
    aliases: &[],
    class_exports: &[CLASSES],
    constant_exports: &[],
    function_exports: &[],
};

// =============================================================================
// Host object types
// =============================================================================

/// Per Apple docs: CMAcceleration is a structure with x, y, z fields
/// representing acceleration in G-force units along each axis.
/// iPhone coordinate system:
///   x: lateral (positive = right)
///   y: longitudinal (positive = up toward top of device)
///   z: perpendicular to screen (positive = toward user)
/// When device is flat on table face-up: x=0, y=0, z=-1 (gravity pulling down)
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C, packed)]
struct CMAcceleration {
    x: f64,
    y: f64,
    z: f64,
}
unsafe impl crate::mem::SafeRead for CMAcceleration {}
impl_GuestRet_for_large_struct!(CMAcceleration);
impl GuestArg for CMAcceleration {
    const REG_COUNT: usize = 6;
    fn from_regs(regs: &[u32]) -> Self {
        CMAcceleration {
            x: f64::from_regs(&regs[0..2]),
            y: f64::from_regs(&regs[2..4]),
            z: f64::from_regs(&regs[4..6]),
        }
    }
    fn to_regs(self, regs: &mut [u32]) {
        let (x, y, z) = (self.x, self.y, self.z);
        x.to_regs(&mut regs[0..2]);
        y.to_regs(&mut regs[2..4]);
        z.to_regs(&mut regs[4..6]);
    }
}

/// Per Apple docs: CMRotationRate in radians per second around each axis.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C, packed)]
struct CMRotationRate {
    x: f64,
    y: f64,
    z: f64,
}
unsafe impl crate::mem::SafeRead for CMRotationRate {}
impl_GuestRet_for_large_struct!(CMRotationRate);
impl GuestArg for CMRotationRate {
    const REG_COUNT: usize = 6;
    fn from_regs(regs: &[u32]) -> Self {
        CMRotationRate {
            x: f64::from_regs(&regs[0..2]),
            y: f64::from_regs(&regs[2..4]),
            z: f64::from_regs(&regs[4..6]),
        }
    }
    fn to_regs(self, regs: &mut [u32]) {
        let (x, y, z) = (self.x, self.y, self.z);
        x.to_regs(&mut regs[0..2]);
        y.to_regs(&mut regs[2..4]);
        z.to_regs(&mut regs[4..6]);
    }
}

#[derive(Default)]
struct CMMotionManagerHostObject {
    accelerometer_update_interval: f64,
    gyro_update_interval: f64,
    device_motion_update_interval: f64,
    accelerometer_active: bool,
    gyro_active: bool,
    device_motion_active: bool,
    /// Cached last accelerometer reading
    last_acceleration: CMAcceleration,
    /// Timestamp of last accelerometer update
    last_accel_timestamp: f64,
    /// Reference time for computing timestamps. `None` only for the phantom
    /// fallback created via `Default::default()`; real motion managers
    /// allocated through `+alloc` always populate this with `Instant::now()`.
    start_time: Option<Instant>,
}
impl HostObject for CMMotionManagerHostObject {}

#[derive(Default)]
struct CMAccelerometerDataHostObject {
    acceleration: CMAcceleration,
    timestamp: f64,
}
impl HostObject for CMAccelerometerDataHostObject {}

#[derive(Default)]
struct CMGyroDataHostObject {
    rotation_rate: CMRotationRate,
    timestamp: f64,
}
impl HostObject for CMGyroDataHostObject {}

#[derive(Default)]
struct CMDeviceMotionHostObject {
    /// Gravity component of acceleration
    gravity: CMAcceleration,
    /// User-generated acceleration (total minus gravity)
    user_acceleration: CMAcceleration,
    /// Rotation rate
    rotation_rate: CMRotationRate,
    timestamp: f64,
}
impl HostObject for CMDeviceMotionHostObject {}

#[derive(Clone, Copy, Debug, Default)]
struct CMAttitude {
    roll: f64,
    pitch: f64,
    yaw: f64,
}

#[derive(Default)]
struct CMAttitudeHostObject {
    attitude: CMAttitude,
}
impl HostObject for CMAttitudeHostObject {}

// =============================================================================
// Helper: read real accelerometer data from SDL sensor via window
// =============================================================================

/// Attempts to read the current accelerometer from the SDL sensor subsystem.
/// Returns (x, y, z) in G-force units matching iOS coordinate convention,
/// or None if no sensor is available.
fn read_sdl_accelerometer(env: &Environment) -> Option<CMAcceleration> {
    // `Window::get_acceleration` already returns the real or simulated
    // accelerometer reading in iOS G-force units (it converts SDL's m/s^2 and
    // flips the sign internally), so we just forward those values.
    let window = env.window.as_ref()?;
    let (x, y, z) = window.get_acceleration(&env.options);
    Some(CMAcceleration {
        x: x as f64,
        y: y as f64,
        z: z as f64,
    })
}

const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =============================================================================
// CMAccelerometerData
// Per Apple: Encapsulates a single accelerometer sample.
// =============================================================================

@implementation CMAccelerometerData: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(CMAccelerometerDataHostObject {
        acceleration: CMAcceleration { x: 0.0, y: 0.0, z: -1.0 },
        timestamp: 0.0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (CMAcceleration)acceleration {
    env.objc.borrow::<CMAccelerometerDataHostObject>(this).acceleration
}

- (f64)timestamp {
    env.objc.borrow::<CMAccelerometerDataHostObject>(this).timestamp
}

- (f64)_accelerationX {
    env.objc.borrow::<CMAccelerometerDataHostObject>(this).acceleration.x
}
- (f64)_accelerationY {
    env.objc.borrow::<CMAccelerometerDataHostObject>(this).acceleration.y
}
- (f64)_accelerationZ {
    env.objc.borrow::<CMAccelerometerDataHostObject>(this).acceleration.z
}

- (id)description {
    let host = env.objc.borrow::<CMAccelerometerDataHostObject>(this);
    let acceleration = host.acceleration;
    let (x, y, z) = (acceleration.x, acceleration.y, acceleration.z);
    let s = format!(
        "<CMAccelerometerData: timestamp={:.4} x={:.4} y={:.4} z={:.4}>",
        host.timestamp, x, y, z
    );
    let cstr = env.mem.alloc_and_write_cstr(s.as_bytes());
    msg_class![env; NSString stringWithUTF8String:cstr]
}

@end

// =============================================================================
// CMGyroData
// =============================================================================

@implementation CMGyroData: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(CMGyroDataHostObject {
        rotation_rate: CMRotationRate { x: 0.0, y: 0.0, z: 0.0 },
        timestamp: 0.0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (CMRotationRate)rotationRate {
    env.objc.borrow::<CMGyroDataHostObject>(this).rotation_rate
}

- (f64)timestamp {
    env.objc.borrow::<CMGyroDataHostObject>(this).timestamp
}

- (f64)_rotationRateX {
    env.objc.borrow::<CMGyroDataHostObject>(this).rotation_rate.x
}
- (f64)_rotationRateY {
    env.objc.borrow::<CMGyroDataHostObject>(this).rotation_rate.y
}
- (f64)_rotationRateZ {
    env.objc.borrow::<CMGyroDataHostObject>(this).rotation_rate.z
}

@end

// =============================================================================
// CMAttitude
// Per Apple: describes the orientation of the device as roll/pitch/yaw
// (radians), plus a rotation matrix and quaternion. With no real gyroscope on
// desktop/Android hosts, we derive roll/pitch from the gravity vector and
// leave yaw at zero. This is enough for games that only read the property to
// avoid the generic "does not respond to selector" path (which floods the log
// and slows the whole emulator down).
// =============================================================================

@implementation CMAttitude: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(CMAttitudeHostObject::default());
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (f64)roll {
    env.objc.borrow::<CMAttitudeHostObject>(this).attitude.roll
}

- (f64)pitch {
    env.objc.borrow::<CMAttitudeHostObject>(this).attitude.pitch
}

- (f64)yaw {
    env.objc.borrow::<CMAttitudeHostObject>(this).attitude.yaw
}

@end

// =============================================================================
// CMDeviceMotion
// =============================================================================

@implementation CMDeviceMotion: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(CMDeviceMotionHostObject {
        gravity: CMAcceleration { x: 0.0, y: 0.0, z: -1.0 },
        user_acceleration: CMAcceleration { x: 0.0, y: 0.0, z: 0.0 },
        rotation_rate: CMRotationRate { x: 0.0, y: 0.0, z: 0.0 },
        timestamp: 0.0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)attitude {
    let gravity = env.objc.borrow::<CMDeviceMotionHostObject>(this).gravity;
    let (gx, gy, gz) = (gravity.x, gravity.y, gravity.z);
    let pitch = gx.atan2((gy * gy + gz * gz).sqrt());
    let roll = gy.atan2((gx * gx + gz * gz).sqrt());
    let attitude: id = msg_class![env; CMAttitude new];
    {
        let attitude_host = env.objc.borrow_mut::<CMAttitudeHostObject>(attitude);
        attitude_host.attitude = CMAttitude {
            roll,
            pitch,
            yaw: 0.0,
        };
    }
    autorelease(env, attitude)
}

- (CMAcceleration)gravity {
    env.objc.borrow::<CMDeviceMotionHostObject>(this).gravity
}

- (CMAcceleration)userAcceleration {
    env.objc.borrow::<CMDeviceMotionHostObject>(this).user_acceleration
}

- (CMRotationRate)rotationRate {
    env.objc.borrow::<CMDeviceMotionHostObject>(this).rotation_rate
}

- (f64)timestamp {
    env.objc.borrow::<CMDeviceMotionHostObject>(this).timestamp
}

- (f64)_gravityX {
    env.objc.borrow::<CMDeviceMotionHostObject>(this).gravity.x
}
- (f64)_gravityY {
    env.objc.borrow::<CMDeviceMotionHostObject>(this).gravity.y
}
- (f64)_gravityZ {
    env.objc.borrow::<CMDeviceMotionHostObject>(this).gravity.z
}

- (f64)_userAccelerationX {
    env.objc.borrow::<CMDeviceMotionHostObject>(this).user_acceleration.x
}
- (f64)_userAccelerationY {
    env.objc.borrow::<CMDeviceMotionHostObject>(this).user_acceleration.y
}
- (f64)_userAccelerationZ {
    env.objc.borrow::<CMDeviceMotionHostObject>(this).user_acceleration.z
}

- (f64)_rotationRateX {
    env.objc.borrow::<CMDeviceMotionHostObject>(this).rotation_rate.x
}
- (f64)_rotationRateY {
    env.objc.borrow::<CMDeviceMotionHostObject>(this).rotation_rate.y
}
- (f64)_rotationRateZ {
    env.objc.borrow::<CMDeviceMotionHostObject>(this).rotation_rate.z
}

@end

// =============================================================================
// CMMotionManager
// Per Apple: The gateway object for the device's accelerometer, gyroscope,
// and device-motion services. Your app creates an instance of this class and
// uses its properties and methods to:
//   - Determine which sensors are available (isAccelerometerAvailable, etc.)
//   - Set the update interval
//   - Start/stop updates
//   - Retrieve the most recent data (accelerometerData, gyroData, deviceMotion)
// =============================================================================

@implementation CMMotionManager: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(CMMotionManagerHostObject {
        accelerometer_update_interval: 1.0 / 60.0, // 60Hz default per Apple docs
        gyro_update_interval: 1.0 / 60.0,
        device_motion_update_interval: 1.0 / 60.0,
        accelerometer_active: false,
        gyro_active: false,
        device_motion_active: false,
        last_acceleration: CMAcceleration { x: 0.0, y: 0.0, z: -1.0 },
        last_accel_timestamp: 0.0,
        start_time: Some(Instant::now()),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// =========================================================================
// Availability checks
// Per Apple: These indicate whether the hardware sensor is available.
// On the emulator: accelerometer can come from SDL; gyro/magnetometer cannot.
// =========================================================================

- (bool)isAccelerometerAvailable {
    // Accelerometer is available if we have an SDL sensor or if user has a
    // mouse (virtual accelerometer via right-click).
    true
}

- (bool)isGyroAvailable {
    // No gyroscope emulation available on desktop hosts.
    false
}

- (bool)isDeviceMotionAvailable {
    // Device motion requires both accelerometer + gyro for full fusion.
    // We can provide gravity-only device motion from accelerometer alone.
    true
}

- (bool)isMagnetometerAvailable {
    false
}
- (bool)isAccelerometerAvailable {
    // According to https://developer.apple.com/documentation/coremotion/getting-raw-accelerometer-events?language=objc,
    // every iOS device has an accelerometer, but on real hardware this method
    // can still return false if the device isn't ready to produce data yet.
    // Here we always return true since we don't model that readiness state.
    true
}

// =========================================================================
// Update intervals
// Per Apple: The interval, in seconds, for providing accelerometer updates.
// A value of 0 means updates come as fast as possible.
// =========================================================================

- (())setAccelerometerUpdateInterval:(f64)interval {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).accelerometer_update_interval =
        if interval <= 0.0 { 1.0 / 100.0 } else { interval };
}

- (f64)accelerometerUpdateInterval {
    env.objc.borrow::<CMMotionManagerHostObject>(this).accelerometer_update_interval
}

- (())setGyroUpdateInterval:(f64)interval {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).gyro_update_interval =
        if interval <= 0.0 { 1.0 / 100.0 } else { interval };
}

- (f64)gyroUpdateInterval {
    env.objc.borrow::<CMMotionManagerHostObject>(this).gyro_update_interval
}

- (())setDeviceMotionUpdateInterval:(f64)interval {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).device_motion_update_interval =
        if interval <= 0.0 { 1.0 / 100.0 } else { interval };
}

- (f64)deviceMotionUpdateInterval {
    env.objc.borrow::<CMMotionManagerHostObject>(this).device_motion_update_interval
}

// =========================================================================
// Start/Stop updates (pull mode — app reads accelerometerData on demand)
// =========================================================================

- (())startAccelerometerUpdates {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).accelerometer_active = true;
}

- (())stopAccelerometerUpdates {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).accelerometer_active = false;
}

- (())startGyroUpdates {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).gyro_active = true;
}

- (())stopGyroUpdates {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).gyro_active = false;
}

- (())startDeviceMotionUpdates {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).device_motion_active = true;
}

- (())stopDeviceMotionUpdates {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).device_motion_active = false;
}

// Push mode (handler-based) — we activate the sensor but do NOT call
// handlers since that would require implementing NSOperationQueue dispatch.
// Games that use pull-mode (polling accelerometerData) still work perfectly.
- (())startAccelerometerUpdatesToQueue:(id)_queue withHandler:(id)_handler {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).accelerometer_active = true;
}

- (())startGyroUpdatesToQueue:(id)_queue withHandler:(id)_handler {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).gyro_active = true;
}

- (())startDeviceMotionUpdatesToQueue:(id)_queue withHandler:(id)_handler {
    env.objc.borrow_mut::<CMMotionManagerHostObject>(this).device_motion_active = true;
}

// =========================================================================
// Active state queries
// =========================================================================

- (bool)isAccelerometerActive {
    env.objc.borrow::<CMMotionManagerHostObject>(this).accelerometer_active
}

- (bool)isGyroActive {
    env.objc.borrow::<CMMotionManagerHostObject>(this).gyro_active
}

- (bool)isDeviceMotionActive {
    env.objc.borrow::<CMMotionManagerHostObject>(this).device_motion_active
}

// `-[CMMotionManager isMagnetometerActive]` — per
// <https://developer.apple.com/documentation/coremotion/cmmotionmanager/1616080-magnetometeractive>.
// touchHLE never enables the magnetometer (we report `isMagnetometerAvailable`
// as `NO`), so this always returns `NO`. Apple's documentation requires
// the property be queryable even when the hardware is absent.
- (bool)isMagnetometerActive {
    false
}

// Companion symmetric APIs for the magnetometer-pull-mode API. Apple
// documents `startMagnetometerUpdates` / `stopMagnetometerUpdates` and
// the `magnetometerData` accessor in
// <https://developer.apple.com/documentation/coremotion/cmmotionmanager>.
// With no hardware backing we still acknowledge the calls so that apps
// guarding magnetometer use behind `isMagnetometerAvailable` continue to
// run without crashing if they call these unconditionally.
- (())startMagnetometerUpdates {
}
- (())stopMagnetometerUpdates {
}
- (())startMagnetometerUpdatesToQueue:(id)_queue withHandler:(id)_handler {
}
- (id)magnetometerData {
    nil
}
- (())setMagnetometerUpdateInterval:(f64)_interval {
}
- (f64)magnetometerUpdateInterval {
    0.0
}

// =========================================================================
// Data accessors (pull mode)
// Per Apple: Returns the latest sample of accelerometer/gyro/motion data,
// or nil if updates have not been started.
// =========================================================================

- (id)accelerometerData {
    let active = env.objc.borrow::<CMMotionManagerHostObject>(this).accelerometer_active;
    if !active {
        return nil;
    }

    // Read real accelerometer data from SDL sensor, or fall back to simulated
    let accel = read_sdl_accelerometer(env)
        .unwrap_or(CMAcceleration { x: 0.0, y: 0.0, z: -1.0 });

    let timestamp = env.objc.borrow::<CMMotionManagerHostObject>(this)
        .start_time.unwrap_or_else(std::time::Instant::now).elapsed().as_secs_f64();

    // Update cached value
    {
        let host = env.objc.borrow_mut::<CMMotionManagerHostObject>(this);
        host.last_acceleration = accel;
        host.last_accel_timestamp = timestamp;
    }

    // Create and return a fresh CMAccelerometerData object
    let data: id = msg_class![env; CMAccelerometerData new];
    {
        let data_host = env.objc.borrow_mut::<CMAccelerometerDataHostObject>(data);
        data_host.acceleration = accel;
        data_host.timestamp = timestamp;
    }
    autorelease(env, data)
}

- (id)gyroData {
    let active = env.objc.borrow::<CMMotionManagerHostObject>(this).gyro_active;
    if !active {
        return nil;
    }

    // No real gyroscope data available from desktop hosts.
    // Return zero rotation rate (device is stationary).
    let timestamp = env.objc.borrow::<CMMotionManagerHostObject>(this)
        .start_time.unwrap_or_else(std::time::Instant::now).elapsed().as_secs_f64();

    let data: id = msg_class![env; CMGyroData new];
    {
        let data_host = env.objc.borrow_mut::<CMGyroDataHostObject>(data);
        data_host.rotation_rate = CMRotationRate { x: 0.0, y: 0.0, z: 0.0 };
        data_host.timestamp = timestamp;
    }
    autorelease(env, data)
}

- (id)deviceMotion {
    let active = env.objc.borrow::<CMMotionManagerHostObject>(this).device_motion_active;
    if !active {
        return nil;
    }

    // Without a real gyroscope we cannot do sensor fusion. We approximate:
    // - gravity = raw accelerometer reading (accurate when device is still)
    // - userAcceleration = zero (can't separate without gyro)
    // - rotationRate = zero
    let accel = read_sdl_accelerometer(env)
        .unwrap_or(CMAcceleration { x: 0.0, y: 0.0, z: -1.0 });
    let timestamp = env.objc.borrow::<CMMotionManagerHostObject>(this)
        .start_time.unwrap_or_else(std::time::Instant::now).elapsed().as_secs_f64();

    let data: id = msg_class![env; CMDeviceMotion new];
    {
        let data_host = env.objc.borrow_mut::<CMDeviceMotionHostObject>(data);
        data_host.gravity = accel;
        data_host.user_acceleration = CMAcceleration { x: 0.0, y: 0.0, z: 0.0 };
        data_host.rotation_rate = CMRotationRate { x: 0.0, y: 0.0, z: 0.0 };
        data_host.timestamp = timestamp;
    }
    autorelease(env, data)
}

@end

};
