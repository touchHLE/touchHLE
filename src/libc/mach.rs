/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `mach`
//!
//! Resources:
//! - [The GNU Mach Reference Manual](https://www.gnu.org/software/hurd/gnumach-doc/mach.pdf).
//!   While being `the GNU Mach microkernel` manual, it serves as a good
//!   general reference doc explaining various undocumented Mach
//!   interfaces.
//! - MIT's [Mach IPC Interface](https://web.mit.edu/darwin/src/modules/xnu/osfmk/man/).
//!   Another online manual with short description for each function.
//!   The main downside is that argument names are not following Apple's
//!   conventions, so you should double-check the real definitions.

#![allow(non_camel_case_types)]

pub mod arm;
pub mod core_types;
pub mod host;
pub mod init;
pub mod mach_port;
pub mod message;
pub mod policy;
pub mod port;
pub mod semaphore;
pub mod thread_info;
pub mod time;
pub mod time_value;
pub mod vm_map;
