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

use crate::mem::{Mem, MutPtr, MutVoidPtr, SafeRead};
use crate::objc::{id, objc_classes, ClassExports};

use super::NSUInteger;

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

pub fn fast_enumeration_helper(
    mem: &mut Mem,
    this: id,
    iterator: &mut impl Iterator<Item = id>,
    state: MutPtr<NSFastEnumerationState>,
    stackbuf: MutPtr<id>,
    len: NSUInteger,
) -> NSUInteger {
    let NSFastEnumerationState {
        state: start_index, ..
    } = mem.read(state);

    if start_index >= 1 {
        // FIXME: linear time complexity
        _ = iterator.nth((start_index - 1).try_into().unwrap());
    }

    let mut batch_count = 0;
    while batch_count < len {
        if let Some(object) = iterator.next() {
            mem.write(stackbuf + batch_count, object);
            batch_count += 1;
        } else {
            break;
        }
    }
    mem.write(
        state,
        NSFastEnumerationState {
            state: start_index + batch_count,
            items_ptr: stackbuf,
            // can be anything as long as it's dereferenceable and the same
            // each iteration
            // Note: stackbuf can be different each time, it's better to return
            // self pointer
            mutations_ptr: this.cast(),
            extra: Default::default(),
        },
    );
    batch_count
}
