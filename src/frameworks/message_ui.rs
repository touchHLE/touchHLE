/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Message UI framework.

use crate::dyld::HostDylib;
use crate::objc::{objc_classes, ClassExports};

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/MessageUI.framework/MessageUI",
    aliases: &[],
    class_exports: &[CLASSES],
    constant_exports: &[],
    function_exports: &[],
};

const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MFMailComposeViewController: UINavigationController

+ (bool)canSendMail {
    false
}

@end

};
