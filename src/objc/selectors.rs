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
use crate::mach_o::MachO;
use crate::memory::{ConstPtr, Memory, MutPtr, Ptr};

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

impl super::ObjC {
    /// For use by [crate::dyld]: register and deduplicate all the selectors
    /// referenced in the application binary.
    pub fn register_selectors(&mut self, bin: &MachO, mem: &mut Memory) {
        let Some(selrefs) = bin.get_section("__objc_selrefs") else { return; };

        assert!(selrefs.size % 4 == 0);
        let base: MutPtr<ConstPtr<u8>> = Ptr::from_bits(selrefs.addr);
        for i in 0..(selrefs.size / 4) {
            let selref = base + i;
            let sel_cstr = mem.read(selref);
            let sel_str = mem.cstr_at_utf8(sel_cstr);

            match self.selectors.get(sel_str) {
                Some(&existing_sel) => {
                    if sel_cstr != existing_sel.0 {
                        mem.write(selref, existing_sel.0);
                    }
                }
                None => {
                    let sel = SEL(sel_cstr);
                    self.selectors.insert(sel_str.to_string(), sel);
                }
            }
        }
    }
}
