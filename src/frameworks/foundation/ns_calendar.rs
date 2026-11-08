/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Calendar identifier constants (`NSCalendar.h`).
//!
//! These are exported as global data symbols. Apps such as MikuFlick2 link
//! `_NSGregorianCalendar` and pass it to calendar/date-formatter APIs; the
//! identifiers act as opaque tokens here since no calendar math is
//! implemented.

use crate::dyld::{ConstantExports, HostConstant};

const NSGregorianCalendar: &str = "NSGregorianCalendar";

pub const CONSTANTS: ConstantExports = &[(
    "_NSGregorianCalendar",
    HostConstant::NSString(NSGregorianCalendar),
)];
