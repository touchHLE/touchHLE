/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Core Foundation framework.
//!
//! In Apple's implementation, this is a layer independent of, or below,
//! Foundation, and there is "Toll-Free Bridging" that lets some Foundation
//! types be used as if they were the corresponding Core Foundation types and
//! vice-versa. But in this implementation we will cheat and implement things
//! backwards (Core Foundation on top of Foundation) where we can get away with
//! it.
//!
//! Useful resources:
//! - Apple's [Core Foundation Design Concepts](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFDesignConcepts/CFDesignConcepts.html)
//! - Apple's [Memory Management Programming Guide for Core Foundation](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFMemoryMgmt/CFMemoryMgmt.html)

pub mod cf_allocator;
pub mod cf_array;
pub mod cf_bundle;
pub mod cf_data;
pub mod cf_dictionary;
pub mod cf_locale;
pub mod cf_run_loop;
pub mod cf_run_loop_timer;
pub mod cf_string;
pub mod cf_type;
pub mod cf_url;
pub mod time;

pub use cf_type::{CFRelease, CFRetain, CFTypeRef};

pub type CFIndex = i32;
pub type CFOptionFlags = u32;

use crate::abi::GuestArg;
use crate::impl_GuestRet_for_large_struct;
use crate::mem::SafeRead;

pub const kCFNotFound: CFIndex = -1;

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct CFRange {
    pub location: CFIndex,
    pub length: CFIndex,
}

unsafe impl SafeRead for CFRange {}
impl_GuestRet_for_large_struct!(CFRange);
impl GuestArg for CFRange {
    const REG_COUNT: usize = 2;

    fn from_regs(regs: &[u32]) -> Self {
        CFRange {
            location: GuestArg::from_regs(&regs[0..1]),
            length: GuestArg::from_regs(&regs[1..2]),
        }
    }
    fn to_regs(self, regs: &mut [u32]) {
        self.location.to_regs(&mut regs[0..1]);
        self.length.to_regs(&mut regs[1..2]);
    }
}
