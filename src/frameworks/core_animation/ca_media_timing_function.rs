/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CAMediaTimingFunction`

use crate::{
    dyld::{ConstantExports, HostConstant},
    frameworks::foundation::ns_string::to_rust_string,
    msg,
    objc::{autorelease, id, objc_classes, ClassExports},
};

pub type CAMediaTimingFunctionName = id; // NSString*

pub const kCAMediaTimingFunctionDefault: &str = "kCAMediaTimingFunctionDefault";
pub const kCAMediaTimingFunctionEaseIn: &str = "kCAMediaTimingFunctionEaseIn";
pub const kCAMediaTimingFunctionEaseInEaseOut: &str = "kCAMediaTimingFunctionEaseInEaseOut";
pub const kCAMediaTimingFunctionEaseOut: &str = "kCAMediaTimingFunctionEaseOut";
pub const kCAMediaTimingFunctionLinear: &str = "kCAMediaTimingFunctionLinear";

pub const CONSTANTS: ConstantExports = &[
    (
        "_kCAMediaTimingFunctionDefault",
        HostConstant::NSString(kCAMediaTimingFunctionDefault),
    ),
    (
        "_kCAMediaTimingFunctionEaseIn",
        HostConstant::NSString(kCAMediaTimingFunctionEaseIn),
    ),
    (
        "_kCAMediaTimingFunctionEaseInEaseOut",
        HostConstant::NSString(kCAMediaTimingFunctionEaseInEaseOut),
    ),
    (
        "_kCAMediaTimingFunctionEaseOut",
        HostConstant::NSString(kCAMediaTimingFunctionEaseOut),
    ),
    (
        "_kCAMediaTimingFunctionLinear",
        HostConstant::NSString(kCAMediaTimingFunctionLinear),
    ),
];

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation CAMediaTimingFunction: NSObject

+ (id)functionWithName:(CAMediaTimingFunctionName)name {
    let object = msg![env; this alloc];
    let object = msg![env; object init];
    log!("TODO: [CAMediaTimingFunction functionWithName:{:?} ({:?})] -> {:?}", name, to_rust_string(env, name), object);
    autorelease(env, object)
}

@end

};
