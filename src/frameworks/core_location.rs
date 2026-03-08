/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Core Location framework.
//!
//! Proper implementation of this framework _could_ make sense on Android,
//! but it seems like early iOS games were using it exclusively to
//! ~~spy on~~ track users without actual location-based gameplay.
//!
//! Some apps (e.g. maps) would _require_ location support to work properly,
//! but it is not the current focus of the touchHLE. The current focus is,
//! you know, **GAMES**.

use crate::dyld::HostDylib;
use crate::objc::{id, objc_classes, ClassExports};

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/CoreLocation.framework/CoreLocation",
    aliases: &[],
    class_exports: &[CLASSES],
    constant_exports: &[],
    function_exports: &[],
};

type CLLocationAccuracy = f64;

const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CLLocationManager: NSObject

+ (bool)headingAvailable {
    false
}

- (())setDelegate:(id)_delegate {
    // TODO
}

- (bool)headingAvailable {
    false
}

- (())startUpdatingHeading {
    // TODO
}
- (())stopUpdatingHeading {
    // TODO
}

- (())startUpdatingLocation {
    // TODO
}
- (())stopUpdatingLocation {
    // TODO
}

- (())setDesiredAccuracy:(CLLocationAccuracy)_acc {
    // TODO
}

@end

@implementation CLHeading: NSObject
// TODO
@end

};
