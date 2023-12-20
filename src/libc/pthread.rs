/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! POSIX Threads implementation.
//!
//! The pthread API often wants functions to check some precondition and return
//! an error if it isn't met. For convenience and for the sake of debugging this
//! implementation, we'll usually assert on these conditions instead, assuming
//! that the app is well-written and that it won't rely on these soft failures.
//! Cases like this will be marked with a comment saying what error should have
//! been returned, e.g. `assert!(...); // should be EINVAL`.

#![allow(non_camel_case_types)]

/// Helper macro for the common pattern of checking magic numbers and returning
/// [crate::libc::errno::EINVAL] on failure.
///
/// Usage: `check_magic!(env, some_ptr, 0xABAD1DEA);`
macro_rules! check_magic {
    ($env:ident, $object:ident, $expected:ident) => {
        let actual = $env.mem.read($object.cast::<u32>());
        if actual != $expected {
            log!("Warning: failed magic number check for pthread object at {:?}: expected {:#x}, got {:#x}", $object, $expected, actual);
            return $crate::libc::errno::EINVAL;
        }
    }
}

pub mod cond;
pub mod key;
pub mod mutex;
pub mod once;
pub mod thread;

#[derive(Default)]
pub struct State {
    pub cond: cond::State,
    key: key::State,
    thread: thread::State,
}
