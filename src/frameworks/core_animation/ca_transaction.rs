/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CATransaction`.
use crate::{
    dyld::{ConstantExports, HostConstant},
    frameworks::core_foundation::time::CFTimeInterval,
    objc::{id, objc_classes, ClassExports},
};

pub const kCATransactionDisableActions: &str = "kCATransactionDisableActions";

pub const CONSTANTS: ConstantExports = &[(
    "_kCATransactionDisableActions",
    HostConstant::NSString(kCATransactionDisableActions),
)];

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CATransaction: NSObject

+ (())setValue:(id)value forKey:(id)key {
    // very spammy, so using log_dbg until class is actually implemented
    log_dbg!("TODO: [CATransaction setValue:{:?} forKey:{:?}]", value, key);
}

+ (())begin {
    // very spammy, so using log_once
    log_once!("TODO: [CATransaction begin]");
}

+ (())commit {
    // very spammy, so using log_once
    log_once!("TODO: [CATransaction commit]");
}

+ (())setDisableActions:(bool)flag {
    // very spammy, so using log_dbg until class is actually implemented
    log_dbg!(" TODO: [CATransaction setDisableActions:{:?}]", flag);
}

+ (())setAnimationDuration:(CFTimeInterval)duration {
    // very spammy, so using log_dbg until class is actually implemented
    log_dbg!("TODO: [CATransaction setAnimationDuration:{:?}]", duration);
}

@end

};
