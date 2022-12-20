//! Handling of Objective-C selectors.
//!
//! These are the names used to look up method implementations in Objective-C.
//! In Apple's implementation, they are always null-terminated C strings, but
//! they are meant to be treated as opaque values. Selector strings should be
//! (TODO) interned so pointer comparison can be used instead of string
//! comparison.
//!
//! Resources:
//! - Apple's [The Objective-C Programming Language](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjectiveC/Chapters/ocSelectors.html)

use crate::abi::GuestArg;
use crate::memory::{ConstPtr, Memory};

/// Opaque type used for selectors.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
#[allow(clippy::upper_case_acronyms)] // silly clippit, this isn't an acronym!
pub struct SEL(ConstPtr<u8>);

impl GuestArg for SEL {
    const REG_COUNT: usize = <ConstPtr<u8> as GuestArg>::REG_COUNT;
    fn from_regs(regs: &[u32]) -> Self {
        SEL(<ConstPtr<u8> as GuestArg>::from_regs(regs))
    }
}

impl SEL {
    pub fn as_str(self, mem: &Memory) -> &str {
        // selectors are probably always UTF-8 but this hasn't been verified
        mem.cstr_at_utf8(self.0)
    }
}
