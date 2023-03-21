/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! touchHLE is a high-level emulator (HLE) for iPhone OS applications.
//!
//! In various places, the terms "guest" and "host" are used to distinguish
//! between the emulated application (the "guest") and the emulator itself (the
//! "host"), and more generally, their different environments.
//! For example:
//! - The guest is a 32-bit application, so a "guest pointer" is 32 bits.
//! - The host is a 64-bit application, so a "host pointer" is 64 bits.
//! - The guest can only directly access "guest memory".
//! - The host can access both "guest memory" and "host memory".
//! - A "guest function" is emulated Arm code, usually from the app binary.
//! - A "host function" is a Rust function that is part of this emulator.

// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]

fn main() -> Result<(), String> {
    println!("touchHLE {} — https://touchhle.org/", touchHLE::VERSION);
    println!();

    let mut args = std::env::args();
    let _ = args.next().unwrap(); // skip argv[0]

    touchHLE::_main(args.collect())
}
