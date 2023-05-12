/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSEnumerator` and helpers for the `NSFastEnumeration` protocol.
//!
//! The protocol is just:
//! ```objc
//! - (NSUInteger)countByEnumeratingWithState:(NSFastEnumerationState*)state
//!                                   objects:(id)stackbuf
//!                                     count:(NSUInteger)len;
//! ```
//!
//! Resources:
//! - The GCC documentation's [Fast Enumeration Protocol section](https://gcc.gnu.org/onlinedocs/gcc/Fast-enumeration-protocol.html)

use crate::mem::{MutPtr, MutVoidPtr, SafeRead};
use crate::objc::{id, objc_classes, ClassExports};

#[repr(C, packed)]
pub struct NSFastEnumerationState {
    pub state: u32,
    pub items_ptr: MutPtr<id>,
    pub mutations_ptr: MutVoidPtr,
    pub extra: [u32; 5],
}
unsafe impl SafeRead for NSFastEnumerationState {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSEnumerator: NSObject
// Abstract class. Subclass must implement:
// - (id)nextObject;
// TODO: Provide NSFastEnumeration convenience implementation.
@end

};
