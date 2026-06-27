/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Stub for `CoreAudio.framework/CoreAudio`.
//!
//! On iOS, CoreAudio exposes no public C API that games call directly.
//! The audio session and audio queue APIs live in AudioToolbox, not here.
//! This stub exists solely so that apps whose Mach-O dependency list
//! includes CoreAudio (e.g. those built with older SDKs or certain
//! third-party libraries) do not trigger "unimplemented dylib" warnings.

use crate::dyld::FunctionExports;

pub const FUNCTIONS: FunctionExports = &[];
