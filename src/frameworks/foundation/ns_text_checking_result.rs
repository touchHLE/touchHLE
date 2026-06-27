/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSTextCheckingResult`
//!
//! Per Apple's documentation, `NSTextCheckingResult` is a class that describes
//! items located by text checking (such as regex matches). Each result has a
//! `range` and zero or more capture group ranges accessible via
//! `rangeAtIndex:`.
//!
//! Reference: <https://developer.apple.com/documentation/foundation/nstextcheckingresult>

use super::{NSRange, NSUInteger};
use crate::objc::{
    autorelease, id, objc_classes, ClassExports, HostObject, NSZonePtr,
};

/// The NSNotFound sentinel value used in NSRange when a capture group didn't
/// participate in the match.
const NS_NOT_FOUND: i32 = 0x7fffffff;

/// Host object storing match results.
#[derive(Default)]
pub(super) struct NSTextCheckingResultHostObject {
    /// The overall match range (index 0) plus any capture group ranges.
    /// ranges[0] is the full match, ranges[1..] are capture groups.
    pub ranges: Vec<NSRange>,
}
impl HostObject for NSTextCheckingResultHostObject {}

/// Helper: create an autoreleased NSTextCheckingResult from a vector of ranges.
pub fn from_ranges(env: &mut crate::Environment, ranges: Vec<NSRange>) -> id {
    let host = Box::new(NSTextCheckingResultHostObject { ranges });
    let class = env
        .objc
        .get_known_class("NSTextCheckingResult", &mut env.mem);
    let obj = env.objc.alloc_object(class, host, &mut env.mem);
    autorelease(env, obj)
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSTextCheckingResult: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSTextCheckingResultHostObject {
        ranges: Vec::new(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// - (NSRange)range
// Returns the overall range of the match (capture group 0).
- (NSRange)range {
    let host = env.objc.borrow::<NSTextCheckingResultHostObject>(this);
    if let Some(r) = host.ranges.first() {
        NSRange { location: r.location, length: r.length }
    } else {
        NSRange { location: NS_NOT_FOUND as u32, length: 0 }
    }
}

// - (NSRange)rangeAtIndex:(NSUInteger)idx
// Returns the range of the capture group at `idx`. Index 0 is the full match.
// If the capture group did not participate, returns {NSNotFound, 0}.
- (NSRange)rangeAtIndex:(NSUInteger)idx {
    let host = env.objc.borrow::<NSTextCheckingResultHostObject>(this);
    if let Some(r) = host.ranges.get(idx as usize) {
        NSRange { location: r.location, length: r.length }
    } else {
        NSRange { location: NS_NOT_FOUND as u32, length: 0 }
    }
}

// - (NSUInteger)numberOfRanges
- (NSUInteger)numberOfRanges {
    let host = env.objc.borrow::<NSTextCheckingResultHostObject>(this);
    host.ranges.len() as NSUInteger
}

// - (NSUInteger)resultType
// NSTextCheckingTypeRegularExpression = 1 << 10 = 1024
- (NSUInteger)resultType {
    1024
}

@end

};
