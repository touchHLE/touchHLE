/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */


use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, Mem, SafeRead};
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
pub(super) struct protocol_t {
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

pub(super) fn collect_protocols_from_bin(
    protocol_list_ptr: ConstPtr<protocol_list_t>,
    mem: &Mem,
) -> Vec<ConstPtr<protocol_t>> {
    let protocol_list_t {count} = mem.read(protocol_list_ptr);
    let proto_ptr_ptr = (protocol_list_ptr + 1).cast::<ConstPtr<protocol_t>>();
    let mut protos = Vec::new();
    for i in 0..count {
        let proto_ptr = mem.read(proto_ptr_ptr + i);
        let proto = mem.read(proto_ptr);
        protos.push(proto_ptr);
        if !proto.protocols.is_null() {
            protos.extend_from_slice(&collect_protocols_from_bin(proto.protocols, mem));
        }
    }
    protos
}