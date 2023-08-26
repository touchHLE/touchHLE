/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFDictionary` and `CFMutableDictionary`.
//!
//! These are toll-free bridged to `NSDictionary` and `NSMutableDictionary` in
//! Apple's implementation. Here they are the same types.

pub type CFDictionaryRef = super::CFTypeRef;
#[allow(dead_code)]
pub type CFMutableDictionaryRef = super::CFTypeRef;
