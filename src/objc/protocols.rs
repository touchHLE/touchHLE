/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */


use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, SafeRead};
use crate::objc::Class;
use crate::objc::methods::method_list_t;

/// The layout of a protocol list in an app binary.
///
/// The name, field names and field layout are based on what Ghidra outputs.
#[repr(C, packed)]
pub(super) struct protocol_list_t {
    count: GuestUSize,
    // entries follow the struct
}
unsafe impl SafeRead for protocol_list_t {}


/// The layout of a protocol in an app binary.
///
/// The name, field names and field layout are based on what Ghidra outputs.
#[repr(C, packed)]
struct protocol_t {
    isa: Class,
    name: ConstPtr<u8>,
    protocols: ConstPtr<protocol_list_t>,
    instance_methods: ConstPtr<method_list_t>,
    class_methods: ConstPtr<method_list_t>,
    optional_instance_methods: ConstPtr<method_list_t>,
    optional_class_methods: ConstPtr<method_list_t>,
    _properties: ConstVoidPtr, // property list (TODO)
    _unk0: u32,
    _unk1: u32
}
unsafe impl SafeRead for protocol_t {}