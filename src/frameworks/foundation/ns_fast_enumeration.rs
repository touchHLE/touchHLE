//! Helpers for the `NSFastEnumeration` protocol.
//!
//! The protocol is just:
//! ```
//! - (NSUInteger)countByEnumeratingWithState:(NSFastEnumerationState*)state
//!                                   objects:(id)stackbuf
//!                                     count:(NSUInteger)len
//! ```
//!
//! Resources:
//! - The GCC documentation's [Fast Enumeration Protocol section](https://gcc.gnu.org/onlinedocs/gcc/Fast-enumeration-protocol.html)

use crate::mem::{MutPtr, MutVoidPtr, SafeRead};
use crate::objc::id;

#[repr(C, packed)]
pub struct NSFastEnumerationState {
    pub state: u32,
    pub items_ptr: MutPtr<id>,
    pub mutations_ptr: MutVoidPtr,
    pub extra: [u32; 5],
}
unsafe impl SafeRead for NSFastEnumerationState {}
