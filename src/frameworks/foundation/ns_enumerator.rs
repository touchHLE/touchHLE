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
use crate::objc::{id, msg, nil, objc_classes, ClassExports};
use crate::Environment;

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

// Abstract class. Subclass must implement:
// - (id)nextObject;
@implementation NSEnumerator: NSObject

// NSFastEnumeration (convenience) implementation
- (NSUInteger)countByEnumeratingWithState:(MutPtr<NSFastEnumerationState>)state
                                  objects:(MutPtr<id>)stackbuf
                                    count:(NSUInteger)len {
    fast_enumeration_helper(env, this, |env, _| {
        msg![env; this nextObject]
    }, state, stackbuf, len)
}

@end

};

pub fn fast_enumeration_helper<F: FnMut(&mut Environment, u32) -> id>(
    env: &mut Environment,
    this: id,
    mut iter_fn: F,
    state: MutPtr<NSFastEnumerationState>,
    stackbuf: MutPtr<id>,
    len: NSUInteger,
) -> NSUInteger {
    let NSFastEnumerationState {
        state: start_index, ..
    } = env.mem.read(state);

    let mut batch_count = 0;
    while batch_count < len {
        let object = iter_fn(env, start_index + batch_count);
        if object != nil {
            env.mem.write(stackbuf + batch_count, object);
            batch_count += 1;
        } else {
            break;
        };
    }
    env.mem.write(
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
