/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `Definition of a Mach port`

use crate::libc::mach::core_types::natural_t;

type mach_port_name_t = natural_t;
pub type mach_port_t = mach_port_name_t;

/// MACH_PORT_NULL indicates the absence of any port or port rights.
pub const MACH_PORT_NULL: mach_port_name_t = 0;
/// MACH_PORT_DEAD indicates that a port right was present, but it died.
pub const MACH_PORT_DEAD: mach_port_name_t = !0;
