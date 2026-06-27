/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSMethodSignature`.
//!
//! Resources:
//! - [Type encodings](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjCRuntimeGuide/Articles/ocrtTypeEncodings.html#//apple_ref/doc/uid/TP40008048-CH100)
//! - [Method signatures](https://gcc.gnu.org/onlinedocs/gcc-4.9.0/gcc/Method-signatures.html)

use crate::environment::Environment;
use crate::frameworks::foundation::NSUInteger;
use crate::libc::string::strncpy;
use crate::mem::{ConstPtr, GuestUSize};
use crate::objc::{autorelease, id, objc_classes, ClassExports, HostObject};

struct NSMethodSignatureHostObject {
    return_type: ConstPtr<u8>,
    arg_types: Vec<(ConstPtr<u8>, GuestUSize)>,
}
impl HostObject for NSMethodSignatureHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSMethodSignature: NSObject

+ (id)signatureWithObjCTypes:(ConstPtr<u8>)types {
    let (return_type, arg_types) = parse_signature(env, types);
    let host_object = Box::new(NSMethodSignatureHostObject {
        return_type,
        arg_types,
    });
    let res = env.objc.alloc_object(this, host_object, &mut env.mem);
    autorelease(env, res)
}

- (NSUInteger)numberOfArguments {
    env.objc.borrow::<NSMethodSignatureHostObject>(this).arg_types.len().try_into().unwrap()
}

- (ConstPtr<u8>)getArgumentTypeAtIndex:(NSUInteger)idx {
    env.objc.borrow::<NSMethodSignatureHostObject>(this).arg_types[idx as usize].0
}

- (ConstPtr<u8>)methodReturnType {
    env.objc.borrow::<NSMethodSignatureHostObject>(this).return_type
}

- (())dealloc {
    let host_obj = env.objc.borrow::<NSMethodSignatureHostObject>(this);
    env.mem.free(host_obj.return_type.cast_mut().cast());
    for (arg_type, _) in &host_obj.arg_types {
        env.mem.free((*arg_type).cast_mut().cast());
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

fn parse_signature(
    env: &mut Environment,
    sig: ConstPtr<u8>,
) -> (ConstPtr<u8>, Vec<(ConstPtr<u8>, GuestUSize)>) {
    // first parse return type and total size in bytes
    let (scanned, return_type_idx, total_size) = parse_signature_inner(env, sig);
    let return_type_str: ConstPtr<u8> = env.mem.calloc(return_type_idx + 1).cast().cast_const();
    _ = strncpy(env, return_type_str.cast_mut(), sig, return_type_idx);
    log_dbg!(
        "Return type: '{}'; Total size {} bytes",
        env.mem.cstr_at_utf8(return_type_str).unwrap(),
        total_size
    );
    let mut scan_idx = scanned;
    let mut res = Vec::new();
    loop {
        let c = env.mem.read(sig + scan_idx);
        if c == b'\0' {
            break;
        }
        let (scanned, arg_type_idx, offset) = parse_signature_inner(env, sig + scan_idx);
        let arg_type_str: ConstPtr<u8> = env.mem.calloc(arg_type_idx + 1).cast().cast_const();
        _ = strncpy(env, arg_type_str.cast_mut(), sig + scan_idx, arg_type_idx);
        log_dbg!(
            "Arg {} type '{}' at offset {}",
            res.len(),
            env.mem.cstr_at_utf8(arg_type_str).unwrap(),
            offset
        );
        res.push((arg_type_str, offset));
        scan_idx += scanned;
    }
    (return_type_str, res)
}

fn parse_signature_inner(env: &mut Environment, curr: ConstPtr<u8>) -> (GuestUSize, u32, u32) {
    let mut idx = 0;
    let c = env.mem.read(curr);
    match c {
        b'^' => {
            // pointer
            let (scanned, read, size) = parse_signature_inner(env, curr + 1);
            (scanned + 1, read + 1, size)
        }
        b'v' | b'@' | b':' | b'f' | b'c' | b'*' | b'i' => {
            idx += 1;
            let mut size = 0;
            while let cc @ b'0'..=b'9' = env.mem.read(curr + idx) {
                size = size * 10 + (cc - b'0') as u32;
                idx += 1;
            }
            (idx, 1, size)
        }
        _ => unimplemented!("parse_signature_inner: {}", c as char),
    }
}
