/*
 * This Source Code Code is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `libiconv` stub.
//!
//! Some apps link `/usr/lib/libiconv.2.dylib` without actually importing any
//! of its symbols (e.g. Asphalt 6 via a bundled static library). This empty
//! dylib entry exists so the loader does not report a missing dylib.

use crate::dyld::HostDylib;

pub const DYLIB: HostDylib = HostDylib {
    path: "/usr/lib/libiconv.2.dylib",
    aliases: &["/usr/lib/libiconv.dylib"],
    class_exports: &[],
    constant_exports: &[],
    function_exports: &[],
};
