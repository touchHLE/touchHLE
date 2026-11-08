/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Minimal `Accounts.framework` stub.
//!
//! Provides the `ACAccountType` identifier constants that apps reference as
//! global data symbols (e.g. MikuFlick2 links `_ACAccountTypeIdentifierTwitter`
//! for its Twitter posting feature). No `ACAccountStore` functionality is
//! implemented.

use crate::dyld::{ConstantExports, HostConstant, HostDylib};

pub const CONSTANTS: ConstantExports = &[(
    "_ACAccountTypeIdentifierTwitter",
    // Literal value from Apple's Accounts framework headers.
    HostConstant::NSString("com.apple.twitter"),
)];

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/Accounts.framework/Accounts",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[CONSTANTS],
    function_exports: &[],
};
