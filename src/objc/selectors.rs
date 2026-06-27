/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Handling of Objective-C selectors.
//!
//! These are the names used to look up method implementations in
//! Objective-C. In Apple's implementation, they are always
//! null-terminated C strings, but they are meant to be treated as
//! opaque values. Selector strings should be (TODO) interned so
//! pointer comparison can be used instead of string comparison.
//!
//! Resources:
//! - Apple's [The Objective-C Programming Language](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjectiveC/Chapters/ocSelectors.html)

use std::collections::HashMap;
use std::sync::{OnceLock, Mutex};

use super::ObjC;
use crate::abi::{GuestArg, GuestRet};
use crate::mach_o::MachO;
use crate::mem::{ConstPtr, Mem, MutPtr, Ptr, SafeRead};
use crate::Environment;

/// Create a string literal for a selector from Objective-C message
/// syntax components. Useful for [super::objc_classes] and for
/// [super::msg].
#[macro_export]
macro_rules! selector {
    // "foo"
    ($name:ident) => { stringify!($name) };
    // "fooWithBar:", "fooWithBar:Baz", "fooWithBar:::" etc
    ($_:tt; $name:ident $(, $($namen:ident)?)*) => {
        concat!(stringify!($name), ":", $($(stringify!($namen),)? ":"),*)
    }
}
pub use crate::selector; // #[macro_export] is weird...

/// Opaque type used for selectors.
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq, Hash)]
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
        // Selectors are expected to be UTF-8, but if a corrupt or
        // misinterpreted pointer is passed (e.g. a class pointer used
        // as SEL), the bytes may not be valid UTF-8. Return a safe
        // fallback instead of panicking so that callers can log the
        // issue and continue execution.
        match mem.cstr_at_utf8(self.0) {
            Ok(s) => s,
            Err(_) => "<invalid-selector-utf8>",
        }
    }
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
    /// A null/sentinel SEL. Useful for places that need a "no selector"
    /// value (e.g. block-based notification observers don't carry a
    /// selector, but still flow through the same Observer struct).
    pub const fn null() -> Self {
        SEL(Ptr::null())
    }
}

unsafe impl SafeRead for SEL {}

impl ObjC {
    pub fn lookup_selector(&self, name: &str) -> Option<SEL> {
        self.selectors.get(name).copied()
    }

    /// Register a selector using a Rust [String]. Despite the name
    /// there is no inherent "host" quality of the resulting selector,
    /// but because this function will allocate a new C string, this
    /// function is not the most efficient route if there's already a
    /// constant string in the app binary.
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
    /// [ObjC::register_bin_selectors], so that selector strings in
    /// the app binary can be re-used. [crate::dyld] calls both of
    /// these.
    pub fn register_host_selectors(&mut self, mem: &mut Mem) {
        for (_name, template) in crate::dyld::DYLIB_LIST
            .iter()
            .flat_map(|dylib| dylib.class_exports)
            .copied()
            .flatten()
        {
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

    /// Register a selector from the application binary. Must be a
    /// static-lifetime constant string.
    pub(super) fn register_bin_selector(
        &mut self,
        sel_cstr: ConstPtr<u8>,
        mem: &Mem,
    ) -> Option<SEL> {
        // If the bytes at the selector pointer are not valid UTF-8
        // (e.g. a corrupted or misaligned binary section), skip the
        // entry rather than panicking.
        let sel_str = match mem.cstr_at_utf8(sel_cstr) {
            Ok(s) => s,
            Err(_) => {
                warn_non_utf8_selector_once(sel_cstr);
                return None;
            }
        };

        if let Some(existing_sel) = self.lookup_selector(sel_str) {
            Some(existing_sel)
        } else {
            let sel = SEL(sel_cstr);
            self.selectors.insert(sel_str.to_string(), sel);
            Some(sel)
        }
    }

    /// For use by [crate::dyld]: register and deduplicate all the
    /// selectors referenced in the application binary.
    pub fn register_bin_selectors(&mut self, bin: &MachO, mem: &mut Mem) {
        let Some(selrefs) = bin.get_section("__objc_selrefs") else {
            return;
        };

        assert!(selrefs.size % 4 == 0);
        let base: MutPtr<ConstPtr<u8>> = Ptr::from_bits(selrefs.addr);
        for i in 0..(selrefs.size / 4) {
            let selref = base + i;
            let sel_cstr = mem.read(selref);

            if let Some(sel) = self.register_bin_selector(sel_cstr, mem) {
                mem.write(selref, sel.0);
            }
        }
    }

    /// Dumps all selectors referenced by the binary as JSON to stdout.
    ///
    /// The JSON has the following form:
    /// ```json
    /// {
    ///     "object": "selectors",
    ///     "selectors": [
    ///         {
    ///             "selector": ((name of selector)),
    ///             "instance_implementations": [ ((names of classes)) ]
    ///                 | null,
    ///             "class_implementations": [ ((names of classes)) ]
    ///                 | null,
    ///         },
    ///         ...
    ///     ],
    /// }
    /// ```
    pub fn dump_selectors(
        &self,
        bin: &MachO,
        mem: &Mem,
        file: &mut std::fs::File,
    ) -> Result<(), std::io::Error> {
        use std::io::Write;
        let Some(selrefs) = bin.get_section("__objc_selrefs") else {
            writeln!(file, "{{ \"object\": \"selectors\", \"selectors\": [] }}")?;
            log!("No selectors in binary!");
            return Ok(());
        };
        assert!(selrefs.size % 4 == 0);
        // We manually gather selectors from the binary since it
        // represents the selectors actually used, whereas using
        // self.selectors would include all host selectors.
        let base: ConstPtr<SEL> = Ptr::from_bits(selrefs.addr);
        let bin_sels: Vec<SEL> = (0..(selrefs.size / 4))
            .map(|i| mem.read(base + i))
            .collect();

        // Gather all selectors in all linked classes. The first
        // vector is for instance methods, the second for class
        // methods.
        let mut impl_selectors: HashMap<SEL, (Vec<&str>, Vec<&str>)> = HashMap::new();
        for class in self.classes.values() {
            let class_host_object = self.get_host_object(*class).unwrap();
            let Some(super::ClassHostObject { name, methods, .. }) =
                class_host_object.as_any().downcast_ref()
            else {
                continue;
            };
            for sel in methods.keys() {
                let entry = impl_selectors.entry(*sel);
                entry.or_default().0.push(name.as_str());
            }
            let metaclass = Self::read_isa(*class, mem);
            // Also get class methods:
            let metaclass_host_object = self.get_host_object(metaclass).unwrap();
            let super::ClassHostObject { methods, .. } =
                metaclass_host_object.as_any().downcast_ref().unwrap();
            for sel in methods.keys() {
                let entry = impl_selectors.entry(*sel);
                entry.or_default().1.push(name.as_str());
            }
        }

        // Also check unlinked host classes: just because the binary
        // doesn't link them in directly doesn't mean it won't use
        // them.
        for (class_name, template) in crate::dyld::DYLIB_LIST
            .iter()
            .flat_map(|dylib| dylib.class_exports)
            .copied()
            .flatten()
        {
            if self.classes.contains_key(*class_name) {
                continue;
            }

            for &(sel_name, _) in template.instance_methods {
                let sel = self.lookup_selector(sel_name).unwrap();
                let entry = impl_selectors.entry(sel);
                entry.or_default().0.push(class_name);
            }

            for &(sel_name, _) in template.class_methods {
                let sel = self.lookup_selector(sel_name).unwrap();
                let entry = impl_selectors.entry(sel);
                entry.or_default().1.push(class_name);
            }
        }

        write!(
            file,
            "{{\n    \"object\": \"selectors\",\n    \
             \"selectors\": [ "
        )?;
        for (i, sel) in bin_sels.iter().enumerate() {
            // Why doesn't json allow trailing commas...
            let comma = if i == bin_sels.len() - 1 { "" } else { "," };

            let name = sel.as_str(mem);
            write!(file, "        {{ \"selector\": \"{name}\"")?;
            if let Some((instance_impls, class_impls)) = impl_selectors.get(sel) {
                if !instance_impls.is_empty() {
                    write!(file, ", \"instance_implementations\": [ ")?;
                    for (j, class) in instance_impls.iter().enumerate() {
                        let comma = if j == instance_impls.len() - 1 {
                            ""
                        } else {
                            ","
                        };
                        write!(file, "\"{class}\"{comma} ")?;
                    }
                    write!(file, "]")?;
                }
                if !class_impls.is_empty() {
                    write!(file, ", \"class_implementations\": [ ")?;
                    for (j, class) in class_impls.iter().enumerate() {
                        let comma = if j == class_impls.len() - 1 { "" } else { "," };
                        write!(file, "\"{class}\"{comma} ")?;
                    }
                    write!(file, "]")?;
                }
            }
            writeln!(file, "}}{comma}")?;
        }
        write!(file, "    ]\n}}")
    }
}

/// Standard Objective-C runtime function for selector registration.
pub(super) fn sel_registerName(env: &mut Environment, name: ConstPtr<u8>) -> SEL {
    // Guard against a null or invalid pointer being passed as the
    // selector name; return a null SEL rather than panicking.
    if name.is_null() {
        log!("Warning: sel_registerName called with null pointer");
        return SEL(Ptr::null());
    }

    let name_str = match env.mem.cstr_at_utf8(name) {
        Ok(s) => s,
        Err(_) => {
            log!(
                "Warning: sel_registerName: name at {:?} is not \
                 valid UTF-8; returning null SEL",
                name
            );
            return SEL(Ptr::null());
        }
    };

    if let Some(existing) = env.objc.lookup_selector(name_str) {
        return existing;
    }

    let name_str = name_str.to_string();
    env.objc.register_host_selector(name_str, &mut env.mem)
}

/// `SEL sel_getUid(const char *str)` — per Apple's runtime, this is
/// functionally identical to `sel_registerName`: it registers a method
/// name with the runtime and returns the corresponding selector.
/// Reference: <https://developer.apple.com/documentation/objectivec/sel_getuid(_:)>
pub(super) fn sel_getUid(env: &mut Environment, name: ConstPtr<u8>) -> SEL {
    sel_registerName(env, name)
}

/// `const char *sel_getName(SEL sel)` — returns the C string name of a
/// selector. In our runtime a [SEL] already wraps a pointer to its
/// null-terminated name string, so we simply return that pointer.
/// A null selector maps to the C string "<null selector>" in Apple's
/// implementation; we return a null pointer, which callers treat as an
/// empty/absent name.
/// Reference: <https://developer.apple.com/documentation/objectivec/sel_getname(_:)>
pub(super) fn sel_getName(_env: &mut Environment, sel: SEL) -> ConstPtr<u8> {
    sel.0
}

/// `BOOL sel_isEqual(SEL lhs, SEL rhs)` — returns whether two selectors
/// are equal. Selectors are registered/deduplicated, so a pointer
/// comparison matches Apple's behavior. As a fallback (e.g. for an
/// unregistered binary selector pointer), compare the underlying name
/// strings too.
/// Reference: <https://developer.apple.com/documentation/objectivec/sel_isequal(_:_:)>
pub(super) fn sel_isEqual(env: &mut Environment, lhs: SEL, rhs: SEL) -> bool {
    if lhs.0 == rhs.0 {
        return true;
    }
    if lhs.is_null() || rhs.is_null() {
        return false;
    }
    lhs.as_str(&env.mem) == rhs.as_str(&env.mem)
}

fn warn_non_utf8_selector_once(sel_cstr: ConstPtr<u8>) {
    use std::collections::HashSet;
    static SEEN: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let addr = sel_cstr.to_bits();
    let mut guard = seen.lock().unwrap();
    if guard.insert(addr) {
        log!(
            "Warning: skipping bin selector at {:?}: not valid UTF-8 \
             (further occurrences at this address will be silenced)",
            sel_cstr
        );
    }
}
