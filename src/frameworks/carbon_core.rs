/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Some things from Carbon Core headers.

/// Status code. At least in Audio Toolbox's use, this is usually a FourCC.
/// 0 means success.
///
/// This is from `MacTypes.h`, which unusually isn't part of any framework!
pub type OSStatus = i32;

/// End-of-file status code.
///
/// One of many status codes from `MacErrors.h`, which is in Carbon Core.
pub const eofErr: OSStatus = -39;

/// Status code meaning that a parameter supplied by the user was invalid.
///
/// One of many status codes from `MacErrors.h`, which is in Carbon Core.
pub const paramErr: OSStatus = -50;
