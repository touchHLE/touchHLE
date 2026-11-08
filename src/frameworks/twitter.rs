/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for `Twitter.framework/Twitter`.
//!
//! Twitter.framework (iOS 5–8) only exists so that legacy apps which list it
//! in their Mach-O load commands (e.g. MikuFlick2) can be dylib-resolved.
//! The actual social-account symbols those apps use live in Accounts and
//! Social; nothing is exported here.

use crate::dyld::HostDylib;

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/Twitter.framework/Twitter",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[],
    function_exports: &[],
};
