/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The CoreFoundation Network framework.

pub mod cf_http_message;

use crate::dyld::{ConstantExports, HostConstant};

/// `NSURLAuthenticationMethod` constants (`NSURLProtectionSpace.h`).
pub const CONSTANTS: ConstantExports = &[(
    "_NSURLAuthenticationMethodServerTrust",
    HostConstant::NSString("NSURLAuthenticationMethodServerTrust"),
)];

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/System/Library/Frameworks/CFNetwork.framework/CFNetwork",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[cf_http_message::CONSTANTS, CONSTANTS],
    function_exports: &[cf_http_message::FUNCTIONS],
};
