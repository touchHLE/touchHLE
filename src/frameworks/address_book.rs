/*
 * This Source Code Code is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Minimal `AddressBook.framework` stub.
//!
//! Some games (e.g. Asphalt 6, via the Facebook SDK) link the AddressBook
//! framework for contact/friend features. None of that is emulated: the
//! functions below are inert stubs returning empty/error values, so callers
//! take their "no contacts" path. The `ABPropertyID` constants are small
//! placeholder integers; they are only ever passed back into the stubs.

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant, HostDylib};
use crate::objc::{id, nil};
use crate::Environment;

/// `ABPropertyID` is a plain `int32_t` variable, so the symbol is the integer
/// itself, not a pointer like the string constants.
fn property_id(env: &mut Environment, value: i32) -> crate::mem::ConstVoidPtr {
    env.mem.alloc_and_write(value).cast().cast_const()
}

pub const CONSTANTS: ConstantExports = &[
    (
        "_kABPersonFirstNameProperty",
        HostConstant::Custom(|env| property_id(env, 0)),
    ),
    (
        "_kABPersonLastNameProperty",
        HostConstant::Custom(|env| property_id(env, 1)),
    ),
    (
        "_kABPersonEmailProperty",
        HostConstant::Custom(|env| property_id(env, 2)),
    ),
    (
        "_kABPersonPhoneProperty",
        HostConstant::Custom(|env| property_id(env, 3)),
    ),
    (
        "_kABWorkLabel",
        HostConstant::NSString("_$!<Work>!$_"),
    ),
];

fn ABAddressBookCreate(_env: &mut Environment) -> id {
    // NULL: "could not open the address book", so callers bail out.
    nil
}
fn ABAddressBookCopyArrayOfAllPeople(_env: &mut Environment, _address_book: id) -> id {
    nil
}
fn ABPersonCreate(_env: &mut Environment) -> id {
    nil
}
fn ABPersonGetSortOrdering(_env: &mut Environment) -> u32 {
    // kABPersonSortByFirstName = 0
    0
}
fn ABPersonComparePeopleByName(
    _env: &mut Environment,
    _person1: id,
    _person2: id,
    _sort_order: u32,
) -> i32 {
    // kCFCompareEqualTo
    0
}
fn ABRecordCopyValue(_env: &mut Environment, _record: id, _property: u32) -> id {
    nil
}
fn ABRecordSetValue(
    _env: &mut Environment,
    _record: id,
    _property: u32,
    _value: id,
    _error: crate::mem::MutVoidPtr,
) -> u32 {
    // false
    0
}
fn ABMultiValueCreateMutable(_env: &mut Environment) -> id {
    nil
}
fn ABMultiValueGetCount(_env: &mut Environment, _multi_value: id) -> i32 {
    0
}
fn ABMultiValueCopyValueAtIndex(_env: &mut Environment, _multi_value: id, _index: i32) -> id {
    nil
}
fn ABMultiValueAddValueAndLabel(
    _env: &mut Environment,
    _multi_value: id,
    _value: id,
    _label: id,
    _identifier_out: crate::mem::MutVoidPtr,
) -> u32 {
    // false
    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(ABAddressBookCreate()),
    export_c_func!(ABAddressBookCopyArrayOfAllPeople(_)),
    export_c_func!(ABPersonCreate()),
    export_c_func!(ABPersonGetSortOrdering()),
    export_c_func!(ABPersonComparePeopleByName(_, _, _)),
    export_c_func!(ABRecordCopyValue(_, _)),
    export_c_func!(ABRecordSetValue(_, _, _, _)),
    export_c_func!(ABMultiValueCreateMutable()),
    export_c_func!(ABMultiValueGetCount(_)),
    export_c_func!(ABMultiValueCopyValueAtIndex(_, _)),
    export_c_func!(ABMultiValueAddValueAndLabel(_, _, _, _)),
];

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/AddressBook.framework/AddressBook",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[CONSTANTS],
    function_exports: &[FUNCTIONS],
};
