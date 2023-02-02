/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFString`.
//!
//! This is toll-free bridged to `CFURL` in Apple's implementation. Here it is
//! the same type.

pub type CFStringRef = super::CFTypeRef;
