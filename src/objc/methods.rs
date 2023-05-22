/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Handling of Objective-C methods.
//!
//! Resources:
//! - [Apple's documentation of `class_addMethod`](https://developer.apple.com/documentation/objectivec/1418901-class_addmethod?language=objc)

use super::{id, nil, Class, ClassHostObject, ObjC, SEL};
use crate::abi::{CallFromGuest, DotDotDot, GuestArg, GuestFunction, GuestRet};
use crate::mem::{guest_size_of, ConstPtr, GuestUSize, Mem, Ptr, SafeRead};
use crate::Environment;

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
pub trait HostIMP: CallFromGuest {}

impl<R> HostIMP for fn(&mut Environment, id, SEL) -> R where R: GuestRet {}
impl<R, P1> HostIMP for fn(&mut Environment, id, SEL, P1) -> R
where
    R: GuestRet,
    P1: GuestArg,
{
}
impl<R, P1> HostIMP for fn(&mut Environment, id, SEL, P1, DotDotDot) -> R
where
    R: GuestRet,
    P1: GuestArg,
{
}
impl<R, P1, P2> HostIMP for fn(&mut Environment, id, SEL, P1, P2) -> R
where
    R: GuestRet,
    P1: GuestArg,
    P2: GuestArg,
{
}
impl<R, P1, P2> HostIMP for fn(&mut Environment, id, SEL, P1, P2, DotDotDot) -> R
where
    R: GuestRet,
    P1: GuestArg,
    P2: GuestArg,
{
}
impl<R, P1, P2, P3> HostIMP for fn(&mut Environment, id, SEL, P1, P2, P3) -> R
where
    R: GuestRet,
    P1: GuestArg,
    P2: GuestArg,
    P3: GuestArg,
{
}
impl<R, P1, P2, P3> HostIMP for fn(&mut Environment, id, SEL, P1, P2, P3, DotDotDot) -> R
where
    R: GuestRet,
    P1: GuestArg,
    P2: GuestArg,
    P3: GuestArg,
{
}
impl<R, P1, P2, P3, P4> HostIMP for fn(&mut Environment, id, SEL, P1, P2, P3, P4) -> R
where
    R: GuestRet,
    P1: GuestArg,
    P2: GuestArg,
    P3: GuestArg,
    P4: GuestArg,
{
}
impl<R, P1, P2, P3, P4> HostIMP for fn(&mut Environment, id, SEL, P1, P2, P3, P4, DotDotDot) -> R
where
    R: GuestRet,
    P1: GuestArg,
    P2: GuestArg,
    P3: GuestArg,
    P4: GuestArg,
{
}
impl<R, P1, P2, P3, P4, P5> HostIMP for fn(&mut Environment, id, SEL, P1, P2, P3, P4, P5) -> R
where
    R: GuestRet,
    P1: GuestArg,
    P2: GuestArg,
    P3: GuestArg,
    P4: GuestArg,
    P5: GuestArg,
{
}
impl<R, P1, P2, P3, P4, P5> HostIMP
    for fn(&mut Environment, id, SEL, P1, P2, P3, P4, P5, DotDotDot) -> R
where
    R: GuestRet,
    P1: GuestArg,
    P2: GuestArg,
    P3: GuestArg,
    P4: GuestArg,
    P5: GuestArg,
{
}

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
    /// For use by NSObject's getter/setter search methods.
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
}
