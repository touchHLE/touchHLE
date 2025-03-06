/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFBoolean`.

use crate::dyld::{ConstantExports, HostConstant};

pub const kCFBooleanTrue: &str = "kCFBooleanTrue";

pub const CONSTANTS: ConstantExports =
    &[("_kCFBooleanTrue", HostConstant::NSString(kCFBooleanTrue))];
