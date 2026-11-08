/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Minimal `Security.framework` stub.
//!
//! Provides the trust-related string constants apps reference as global data
//! symbols (e.g. MikuFlick2 links `_kSecTrustInfoExtendedValidationKey`).
//! No keychain or TLS functionality is implemented.

use crate::dyld::{ConstantExports, HostConstant, HostDylib};

pub const CONSTANTS: ConstantExports = &[(
    "_kSecTrustInfoExtendedValidationKey",
    HostConstant::NSString("kSecTrustInfoExtendedValidationKey"),
)];

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/Security.framework/Security",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[CONSTANTS],
    function_exports: &[],
};
