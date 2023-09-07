/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Handling of Objective-C methods.
//!
//! Resources:
//! - [Apple's documentation of `class_addMethod`](https://developer.apple.com/documentation/objectivec/1418901-class_addmethod?language=objc)

use super::{
    id, nil, objc_super, Class, ClassHostObject, MsgSendSignature, MsgSendSuperSignature, ObjC, SEL,
};
use crate::abi::{CallFromGuest, DotDotDot, GuestArg, GuestFunction, GuestRet};
use crate::mem::{guest_size_of, ConstPtr, GuestUSize, Mem, Ptr, SafeRead};
use crate::Environment;
use std::any::TypeId;

/// Type for any function implementating a method.
///
/// The name is standard Objective-C.
///
/// In our implementation, we have both "host methods" (Rust functions) and
/// "guest methods" (functions in the guest app). Either way, the function needs
/// to conform to the same ABI: [id] and [SEL] must be its first two parameters.
#[allow(clippy::upper_case_acronyms)]
pub enum IMP {
    Host(&'static dyn HostIMP),
    Guest(GuestIMP),
}

/// Type for any host function implementing a method (see also [IMP]).
pub trait HostIMP: CallFromGuest {
    /// See [MsgSendSignature::type_info].
    fn type_info(&self) -> (TypeId, &'static str);
}

macro_rules! impl_HostIMP {
    ( $($P:ident),* ) => {
        impl<R, $($P,)*> HostIMP for fn(&mut Environment, id, SEL, $($P,)*) -> R
        where
            R: GuestRet + 'static,
            $($P: GuestArg + 'static,)*
        {
            fn type_info(&self) -> (TypeId, &'static str) {
                <(R, (id, SEL, $($P,)*)) as MsgSendSignature>::type_info()
            }
        }
        impl<R, $($P,)*> HostIMP for fn(&mut Environment, id, SEL, $($P,)* DotDotDot) -> R
        where
            R: GuestRet + 'static,
            $($P: GuestArg + 'static,)*
        {
            fn type_info(&self) -> (TypeId, &'static str) {
                todo!("host-to-host message calls with var-args"); // TODO
            }
        }

        // Currently there is a one-to-one mapping between valid host IMP
        // parameters and valid host message send arguments, so the traits for
        // the latter are also implemented here for convenience.

        impl<R, $($P,)*> MsgSendSignature for (R, (id, SEL, $($P,)*))
        where
            R: GuestRet + 'static,
            $($P: GuestArg + 'static,)*
        {
        }
        impl<R, $($P,)*> MsgSendSuperSignature for (R, (ConstPtr<objc_super>, SEL, $($P,)*))
        where
            R: GuestRet + 'static,
            $($P: GuestArg + 'static,)*
        {
            type WithoutSuper = (R, (id, SEL, $($P,)*));
        }
    }
}

impl_HostIMP!();
impl_HostIMP!(P1);
impl_HostIMP!(P1, P2);
impl_HostIMP!(P1, P2, P3);
impl_HostIMP!(P1, P2, P3, P4);
impl_HostIMP!(P1, P2, P3, P4, P5);

/// Type for a guest function implementing a method. See [GuestFunction].
pub type GuestIMP = GuestFunction;

/// The layout of a method list in an app binary.
///
/// The name, field names and field layout are based on what Ghidra outputs.
#[repr(C, packed)]
pub(super) struct method_list_t {
    entsize: GuestUSize,
    count: GuestUSize,
    // entries follow the struct
}
unsafe impl SafeRead for method_list_t {}

/// The layout of a method in an app binary.
///
/// The name, field names and field layout are based on what Ghidra outputs.
#[repr(C, packed)]
struct method_t {
    name: ConstPtr<u8>,
    types: ConstPtr<u8>,
    imp: GuestIMP,
}
unsafe impl SafeRead for method_t {}

impl ClassHostObject {
    // See classes.rs for host method parsing

    pub(super) fn add_methods_from_bin(
        &mut self,
        method_list_ptr: ConstPtr<method_list_t>,
        mem: &Mem,
        objc: &mut ObjC,
    ) {
        let method_list_t { entsize, count } = mem.read(method_list_ptr);
        assert!(entsize >= guest_size_of::<method_t>());

        let methods_base_ptr: ConstPtr<method_t> = (method_list_ptr + 1).cast();

        for i in 0..count {
            let method_ptr: ConstPtr<method_t> =
                Ptr::from_bits(methods_base_ptr.to_bits() + i * entsize);

            // TODO: support type strings
            let method_t {
                name,
                types: _,
                imp,
            } = mem.read(method_ptr);

            // There is no guarantee this string is unique or known.
            // We must deduplicate it like any other.
            let sel = objc.register_bin_selector(name, mem);
            self.methods.insert(sel, IMP::Guest(imp));
        }
    }
}

impl ObjC {
    /// Checks if the provided class has a method in its class chain (that is
    /// to say, objects of the given class respond to a selector).
    pub fn class_has_method(&self, class: Class, sel: SEL) -> bool {
        let mut class = class;
        loop {
            let &ClassHostObject {
                superclass,
                ref methods,
                ..
            } = self.borrow(class);
            if methods.contains_key(&sel) {
                return true;
            } else if superclass == nil {
                return false;
            } else {
                class = superclass;
            }
        }
    }

    /// Same as [Self::class_has_method], but using a named selector (rather
    /// than a pointer).
    #[allow(dead_code)]
    pub fn class_has_method_named(&self, class: Class, sel_name: &str) -> bool {
        if let Some(sel) = self.lookup_selector(sel_name) {
            self.class_has_method(class, sel)
        } else {
            false
        }
    }

    /// Checks if a given object has a method (responds to a selector).
    pub fn object_has_method(&self, mem: &Mem, obj: id, sel: SEL) -> bool {
        self.class_has_method(ObjC::read_isa(obj, mem), sel)
    }

    /// Same as [Self::object_has_method], but using a named selector (rather
    /// than a pointer).
    pub fn object_has_method_named(&self, mem: &Mem, obj: id, sel_name: &str) -> bool {
        if let Some(sel) = self.lookup_selector(sel_name) {
            self.object_has_method(mem, obj, sel)
        } else {
            false
        }
    }

    /// Checks if a class overrides a method provided by its superclass.
    ///
    /// This looks through a superclass chain looking for the selector, stopping
    /// when the superclass is hit (and panicking if it never is). It does not
    /// check whether the selector is actually a method on the superclass.
    pub fn class_overrides_method_of_superclass(
        &self,
        class: Class,
        sel: SEL,
        superclass: Class,
    ) -> bool {
        let mut class = class;
        loop {
            if class == superclass {
                return false;
            }

            let &ClassHostObject {
                superclass,
                ref methods,
                ..
            } = self.borrow(class);
            if methods.contains_key(&sel) {
                return true;
            } else if superclass == nil {
                panic!();
            } else {
                class = superclass;
            }
        }
    }

    pub fn debug_all_class_selectors_as_strings(&self, mem: &Mem, class: Class) -> Vec<String> {
        let mut class = class;
        let mut selector_strings = Vec::new();
        loop {
            let &ClassHostObject {
                superclass,
                ref methods,
                ..
            } = self.borrow(class);
            let mut class_selector_strings = methods
                .keys()
                .map(|sel| sel.as_str(mem).to_string())
                .collect();
            selector_strings.append(&mut class_selector_strings);
            if superclass == nil {
                break;
            } else {
                class = superclass;
            }
        }
        selector_strings
    }
}
