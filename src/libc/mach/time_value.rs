/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Time value struct

use crate::libc::mach::core_types::integer_t;
use crate::mem::SafeRead;

#[repr(C, packed)]
pub struct time_value_t {
    pub seconds: integer_t,
    pub microseconds: integer_t,
}
unsafe impl SafeRead for time_value_t {}
