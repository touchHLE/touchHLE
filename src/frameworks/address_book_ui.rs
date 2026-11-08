/*
 * This Source Code Code is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Minimal `AddressBookUI.framework` stub.
//!
//! Games link this framework (transitively, via the Facebook SDK) but its
//! classes are never actually used at runtime in the paths we emulate. The
//! empty dylib entry exists so the loader does not report a missing dylib;
//! any class reference resolves through the usual unimplemented-class path.

use crate::dyld::HostDylib;

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/AddressBookUI.framework/AddressBookUI",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[],
    function_exports: &[],
};
