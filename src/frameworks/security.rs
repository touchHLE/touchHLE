/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Security framework.

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant, HostDylib};
use crate::frameworks::core_foundation::cf_dictionary::CFDictionaryRef;
use crate::frameworks::core_foundation::CFTypeRef;
use crate::mem::MutPtr;
use crate::objc::nil;
use crate::Environment;

pub const DYLIB: HostDylib = HostDylib {
    path: "/System/Library/Frameworks/Security.framework/Security",
    aliases: &[],
    class_exports: &[],
    constant_exports: &[CONSTANTS],
    function_exports: &[FUNCTIONS],
};

type OSStatus = i32;

const errSecSuccess: OSStatus = 0;
const errSecItemNotFound: OSStatus = -25300;

fn SecItemAdd(
    env: &mut Environment,
    attributes: CFDictionaryRef,
    result: MutPtr<CFTypeRef>,
) -> OSStatus {
    log_dbg!("SecItemAdd({:?}, {:?})", attributes, result);
    if !result.is_null() {
        env.mem.write(result, nil);
    }
    errSecSuccess
}

fn SecItemCopyMatching(
    env: &mut Environment,
    query: CFDictionaryRef,
    result: MutPtr<CFTypeRef>,
) -> OSStatus {
    log_dbg!("SecItemCopyMatching({:?}, {:?})", query, result);
    if !result.is_null() {
        env.mem.write(result, nil);
    }
    errSecItemNotFound
}

fn SecItemDelete(_env: &mut Environment, query: CFDictionaryRef) -> OSStatus {
    log_dbg!("SecItemDelete({:?})", query);
    errSecSuccess
}

fn SecItemUpdate(
    _env: &mut Environment,
    query: CFDictionaryRef,
    attributes_to_update: CFDictionaryRef,
) -> OSStatus {
    log_dbg!("SecItemUpdate({:?}, {:?})", query, attributes_to_update);
    errSecSuccess
}

pub const CONSTANTS: ConstantExports = &[
    ("_kSecClass", HostConstant::NSString("class")),
    ("_kSecClassGenericPassword", HostConstant::NSString("genp")),
    ("_kSecAttrAccount", HostConstant::NSString("acct")),
    ("_kSecAttrLabel", HostConstant::NSString("labl")),
    ("_kSecAttrService", HostConstant::NSString("svce")),
    (
        "_kSecReturnAttributes",
        HostConstant::NSString("r_Attributes"),
    ),
    ("_kSecReturnData", HostConstant::NSString("r_Data")),
    ("_kSecValueData", HostConstant::NSString("v_Data")),
];

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(SecItemAdd(_, _)),
    export_c_func!(SecItemCopyMatching(_, _)),
    export_c_func!(SecItemDelete(_)),
    export_c_func!(SecItemUpdate(_, _)),
];
