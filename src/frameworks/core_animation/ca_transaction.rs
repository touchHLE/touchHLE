/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CATransaction`.
use crate::{
    dyld::{ConstantExports, HostConstant},
    frameworks::core_foundation::time::CFTimeInterval,
    objc::{id, objc_classes, ClassExports, HostObject},
};

type NSZonePtr = id;

pub const kCATransactionDisableActionsDefault: &str = "kCATransactionDisableActions";

pub const CONSTANTS: ConstantExports = &[(
    "_kCATransactionDisableActions",
    HostConstant::NSString(kCATransactionDisableActionsDefault),
)];

#[derive(Default)]
struct CATransactionHostObject {
    disable_actions: bool,
    animation_duration: CFTimeInterval,
}
impl HostObject for CATransactionHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CATransaction: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<CATransactionHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// Handle setting values via key-value coding (KVC)
+ (())setValue:(id)value forKey:(id)key {
    log_dbg!("[CATransaction setValue:{:?} forKey:{:?}]", value, key);
    // TODO: Implement setValue function
    // Only used to set animation Duration
    // Seems like there is a default!?
}

+ (())begin {
    log_dbg!("[CATransaction begin]");
    // TODO: Implement transaction stack handling
}

+ (())commit {
    log_dbg!("[CATransaction commit]");
    // TODO: Commit the current transaction
}

+ (())setDisableActions:(bool)flag {
    log_dbg!("[CATransaction setDisableActions:{:?}]", flag);
    env.objc.borrow_mut::<CATransactionHostObject>(this).disable_actions = flag;
}

+ (bool)disableActions {
    env.objc.borrow::<CATransactionHostObject>(this).disable_actions
}

+ (())setAnimationDuration:(CFTimeInterval)duration {
    log_dbg!("[CATransaction setAnimationDuration:{:?}]", duration);
    env.objc.borrow_mut::<CATransactionHostObject>(this).animation_duration = duration;
}

+ (CFTimeInterval)animationDuration {
    env.objc.borrow::<CATransactionHostObject>(this).animation_duration
}

@end

};
