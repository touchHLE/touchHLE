/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for `CoreMedia.framework/CoreMedia`.
//!
//! CoreMedia is referenced by apps (e.g. MikuFlick2, transitively via
//! AVFoundation) through its Mach-O load commands. Without a matching
//! [crate::dyld::HostDylib] entry the dylib resolver prints a warning at
//! startup. No `CMTime`/`CMSampleBuffer` functionality is implemented here.

use crate::dyld::HostDylib;

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/CoreMedia.framework/CoreMedia",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[],
    function_exports: &[],
};
