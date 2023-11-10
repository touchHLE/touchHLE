/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
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

use super::ObjC;
use crate::abi::{GuestArg, GuestRet};
use crate::mach_o::MachO;
use crate::mem::{ConstPtr, Mem, MutPtr, Ptr};
use crate::Environment;

/// Create a string literal for a selector from Objective-C message syntax
/// components. Useful for [super::objc_classes] and for [super::msg].
#[macro_export]
macro_rules! selector {
    // "foo"
    ($name:ident) => { stringify!($name) };
    // "fooWithBar:", "fooWithBar:Baz" etc
    ($_:tt; $name:ident $(, $namen:ident)*) => {
        concat!(stringify!($name), ":", $(stringify!($namen), ":"),*)
    }
}
pub use crate::selector; // #[macro_export] is weird...

/// Opaque type used for selectors.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
#[allow(clippy::upper_case_acronyms)] // silly clippit, this isn't an acronym!
pub struct SEL(ConstPtr<u8>);

impl GuestArg for SEL {
    const REG_COUNT: usize = <ConstPtr<u8> as GuestArg>::REG_COUNT;
    fn from_regs(regs: &[u32]) -> Self {
        SEL(<ConstPtr<u8> as GuestArg>::from_regs(regs))
    }
    fn to_regs(self, regs: &mut [u32]) {
        <ConstPtr<u8> as GuestArg>::to_regs(self.0, regs)
    }
}
impl GuestRet for SEL {
    fn from_regs(regs: &[u32]) -> Self {
        SEL(<ConstPtr<u8> as GuestRet>::from_regs(regs))
    }
    fn to_regs(self, regs: &mut [u32]) {
        <ConstPtr<u8> as GuestRet>::to_regs(self.0, regs)
    }
}

impl SEL {
    pub fn as_str(self, mem: &Mem) -> &str {
        // selectors are probably always UTF-8 but this hasn't been verified
        mem.cstr_at_utf8(self.0).unwrap()
    }
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl ObjC {
    pub fn lookup_selector(&self, name: &str) -> Option<SEL> {
        self.selectors.get(name).copied()
    }

    /// Register a selector using a Rust [String]. Despite the name there is no
    /// inherent "host" quality of the resulting selector, but because this
    /// function will allocate a new C string, this function is not the most
    /// efficient route if there's already a constant string in the app binary.
    pub fn register_host_selector(&mut self, name: String, mem: &mut Mem) -> SEL {
        if let Some(existing) = self.lookup_selector(&name) {
            return existing;
        }

        let sel = SEL(mem.alloc_and_write_cstr(name.as_bytes()).cast_const());
        self.selectors.insert(name, sel);
        sel
    }

    /// Register and deduplicate all the selectors of host classes.
    ///
    /// To avoid wasting guest memory, call this after calling
    /// [ObjC::register_bin_selectors], so that selector strings in the app
    /// binary can be re-used. [crate::dyld] calls both of these.
    pub fn register_host_selectors(&mut self, mem: &mut Mem) {
        for &class_list in super::CLASS_LISTS {
            for (_name, template) in class_list {
                for method_list in [template.class_methods, template.instance_methods] {
                    for &(name, _imp) in method_list {
                        if self.selectors.contains_key(name) {
                            continue;
                        }
                        let sel = SEL(mem.alloc_and_write_cstr(name.as_bytes()).cast_const());
                        self.selectors.insert(name.to_string(), sel);
                    }
                }
            }
        }
    }

    /// Register a selector from the application binary. Must be a
    /// static-lifetime constant string.
    pub(super) fn register_bin_selector(&mut self, sel_cstr: ConstPtr<u8>, mem: &Mem) -> SEL {
        let sel_str = mem.cstr_at_utf8(sel_cstr).unwrap();

        if let Some(existing_sel) = self.lookup_selector(sel_str) {
            existing_sel
        } else {
            let sel = SEL(sel_cstr);
            self.selectors.insert(sel_str.to_string(), sel);
            sel
        }
    }

    /// For use by [crate::dyld]: register and deduplicate all the selectors
    /// referenced in the application binary.
    pub fn register_bin_selectors(&mut self, bin: &MachO, mem: &mut Mem) {
        let Some(selrefs) = bin.get_section("__objc_selrefs") else {
            return;
        };

        assert!(selrefs.size % 4 == 0);
        let base: MutPtr<ConstPtr<u8>> = Ptr::from_bits(selrefs.addr);
        for i in 0..(selrefs.size / 4) {
            let selref = base + i;
            let sel_cstr = mem.read(selref);

            let sel = self.register_bin_selector(sel_cstr, mem);
            mem.write(selref, sel.0);
        }
    }
}

/// Standard Objective-C runtime function for selector registration.
pub(super) fn sel_registerName(env: &mut Environment, name: ConstPtr<u8>) -> SEL {
    let name = env.mem.cstr_at_utf8(name).unwrap();

    if let Some(existing) = env.objc.lookup_selector(name) {
        return existing;
    }

    let name = name.to_string();
    env.objc.register_host_selector(name, &mut env.mem)
}
