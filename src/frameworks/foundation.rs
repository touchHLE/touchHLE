/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Foundation framework.
//!
//! A concept that Foundation really likes is "class clusters": abstract classes
//! with private concrete implementations. Apple has their own explanation of it
//! in [Cocoa Core Competencies](https://developer.apple.com/library/archive/documentation/General/Conceptual/DevPedia-CocoaCore/ClassCluster.html).
//! Being aware of this concept will make common types like `NSArray` and
//! `NSString` easier to understand.

use crate::dyld::{export_c_func, FunctionExports};
use crate::objc::id;
use crate::Environment;

pub mod ns_array;
pub mod ns_autorelease_pool;
pub mod ns_bundle;
pub mod ns_character_set;
pub mod ns_coder;
pub mod ns_data;
pub mod ns_date;
pub mod ns_date_formatter;
pub mod ns_dictionary;
pub mod ns_enumerator;
pub mod ns_error;
pub mod ns_exception;
pub mod ns_file_handle;
pub mod ns_file_manager;
pub mod ns_keyed_unarchiver;
pub mod ns_locale;
pub mod ns_lock;
pub mod ns_log;
pub mod ns_notification;
pub mod ns_notification_center;
pub mod ns_null;
pub mod ns_objc_runtime;
pub mod ns_object;
pub mod ns_process_info;
pub mod ns_property_list_serialization;
pub mod ns_run_loop;
pub mod ns_set;
pub mod ns_string;
pub mod ns_thread;
pub mod ns_time_zone;
pub mod ns_timer;
pub mod ns_url;
pub mod ns_url_connection;
pub mod ns_url_request;
pub mod ns_user_defaults;
pub mod ns_value;
pub mod ns_xml_parser;

#[derive(Default)]
pub struct State {
    ns_autorelease_pool: ns_autorelease_pool::State,
    ns_bundle: ns_bundle::State,
    ns_file_manager: ns_file_manager::State,
    ns_locale: ns_locale::State,
    ns_notification_center: ns_notification_center::State,
    ns_null: ns_null::State,
    ns_run_loop: ns_run_loop::State,
    ns_string: ns_string::State,
    ns_user_defaults: ns_user_defaults::State,
}

pub type NSInteger = i32;
pub type NSUInteger = u32;

// this should be equal to NSIntegerMax
pub const NSNotFound: i32 = 0x7fffffff;

#[derive(Debug)]
#[repr(C, packed)]
pub struct NSRange {
    pub location: NSUInteger,
    pub length: NSUInteger,
}
unsafe impl crate::mem::SafeRead for NSRange {}
crate::abi::impl_GuestRet_for_large_struct!(NSRange);
impl crate::abi::GuestArg for NSRange {
    const REG_COUNT: usize = 2;

    fn from_regs(regs: &[u32]) -> Self {
        NSRange {
            location: crate::abi::GuestArg::from_regs(&regs[0..1]),
            length: crate::abi::GuestArg::from_regs(&regs[1..2]),
        }
    }
    fn to_regs(self, regs: &mut [u32]) {
        self.location.to_regs(&mut regs[0..1]);
        self.length.to_regs(&mut regs[1..2]);
    }
}

fn NSStringFromRange(env: &mut Environment, range: NSRange) -> id {
    let loc = range.location;
    let len = range.length;
    let string = format!("{{{}, {}}}", loc, len);
    ns_string::from_rust_string(env, string)
}

pub type NSComparisonResult = NSInteger;
pub const NSOrderedAscending: NSComparisonResult = -1;
pub const NSOrderedSame: NSComparisonResult = 0;
pub const NSOrderedDescending: NSComparisonResult = 1;

/// Number of seconds.
pub type NSTimeInterval = f64;

/// UTF-16 code unit.
#[allow(non_camel_case_types)]
pub type unichar = u16;

/// Utility to help with implementing the `hash` method, which various classes
/// in Foundation have to do.
fn hash_helper<T: std::hash::Hash>(hashable: &T) -> NSUInteger {
    use std::hash::Hasher;

    // Rust documentation says DefaultHasher::new() should always return the
    // same instance, so this should give consistent hashes.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hashable.hash(&mut hasher);
    let hash_u64: u64 = hasher.finish();
    (hash_u64 as u32) ^ ((hash_u64 >> 32) as u32)
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(NSStringFromRange(_))];
