/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIDevice`.

use crate::frameworks::foundation::ns_string;
use crate::frameworks::foundation::NSInteger;
use crate::objc::{id, objc_classes, ClassExports, TrivialHostObject};

pub type UIDeviceOrientation = NSInteger;
#[allow(dead_code)]
pub const UIDeviceOrientationUnknown: UIDeviceOrientation = 0;
pub const UIDeviceOrientationPortrait: UIDeviceOrientation = 1;
#[allow(dead_code)]
pub const UIDeviceOrientationPortraitUpsideDown: UIDeviceOrientation = 2;
pub const UIDeviceOrientationLandscapeLeft: UIDeviceOrientation = 3;
pub const UIDeviceOrientationLandscapeRight: UIDeviceOrientation = 4;
#[allow(dead_code)]
pub const UIDeviceOrientationFaceUp: UIDeviceOrientation = 5;
#[allow(dead_code)]
pub const UIDeviceOrientationFaceDown: UIDeviceOrientation = 6;

#[derive(Default)]
pub struct State {
    current_device: Option<id>,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIDevice: NSObject

+ (id)currentDevice {
    if let Some(device) = env.framework_state.uikit.ui_device.current_device {
        device
    } else {
        let new = env.objc.alloc_static_object(
            this,
            Box::new(TrivialHostObject),
            &mut env.mem
        );
        env.framework_state.uikit.ui_device.current_device = Some(new);
        new
    }
}

// NSString
- (id)systemVersion {
    ns_string::get_static_str(env, "2.0")
}

- (id)systemName {
    ns_string::get_static_str(env, "iPhoneOS")
}

- (id)UI_USER_INTERFACE_IDIOM {
    ns_string::get_static_str(env, "UIUserInterfaceIdiom.phone")
}

- (id)systemName {
    ns_string::get_static_str(env, "iPhoneOS")
}



- (bool)isMultitaskingSupported {
    false
}

@end

};
