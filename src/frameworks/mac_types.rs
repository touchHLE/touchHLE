/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MacTypes.h`
//!
//! It's unclear if this belongs to some particular "framework", but it is
//! definitely from Carbon.

/// Status code. At least in Audio Toolbox's use, this is usually a FourCC.
/// 0 means success.
pub type OSStatus = i32;
