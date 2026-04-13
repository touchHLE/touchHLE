/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFReadStream`.

use super::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use super::cf_url::CFURLRef;
use super::CFIndex;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::mem::MutPtr;
use crate::objc::{id, msg, msg_class, objc_classes, ClassExports, HostObject, NSZonePtr};
use crate::Environment;
use crate::fs::GuestFile;

pub type CFReadStreamRef = super::CFTypeRef;

/// Status constants matching Apple's CFStreamStatus enum.
#[allow(dead_code)]
const kCFStreamStatusNotOpen: i32 = 0;
const kCFStreamStatusOpen: i32 = 2;
const kCFStreamStatusError: i32 = 5;

/// Host object stored behind every `_touchHLE_CFReadStream` instance.
struct CFReadStreamHostObject {
    path: String,
    file: Option<GuestFile>,
}

impl HostObject for CFReadStreamHostObject {}

pub fn CFReadStreamCreateWithFile(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    file_url: CFURLRef,
) -> CFReadStreamRef {
    assert!(allocator == kCFAllocatorDefault || env.mem.read(allocator).is_system_default());

    let path_ns: id = msg![env; file_url path];
    let _keep_alive: id = msg![env; path_ns retain];
    let path = to_rust_string(env, path_ns);

    let host_object = Box::new(CFReadStreamHostObject { path: path.to_string(), file: None });

    let class = env
        .objc
        .get_known_class("_touchHLE_CFReadStream", &mut env.mem);
    let stream = env.objc.alloc_object(class, host_object, &mut env.mem);

    let result = crate::objc::retain(env, stream);
    log!("CFReadStreamCreateWithFile('{}') -> {:?}", path, result);
    result
}

fn CFReadStreamOpen(env: &mut Environment, stream: CFReadStreamRef) -> bool {
    log!("CFReadStreamOpen called, stream={:?}", stream);
    let host_obj = env.objc.borrow_mut::<CFReadStreamHostObject>(stream);
    if host_obj.file.is_some() {
        return true;
    }
    let path = host_obj.path.clone();
    let guest_path = crate::fs::GuestPath::new(&path);
    match env.fs.open(guest_path) {
        Ok(guest_file) => {
            // Extract the underlying std::fs::File from GuestFile
            match env.fs.open(guest_path) {
				Ok(guest_file) => {
				let host_obj = env.objc.borrow_mut::<CFReadStreamHostObject>(stream);
				log!("CFReadStreamOpen('{}') => success", path);
				host_obj.file = Some(guest_file);
				true
			}
				Err(()) => {
					log!("CFReadStreamOpen('{}') failed: not found in guest fs", path);
					false
				}
			}
        }
        Err(()) => {
            log!("CFReadStreamOpen('{}') failed: not found in guest fs", path);
            false
        }
    }
}

fn CFReadStreamRead(
    env: &mut Environment,
    stream: CFReadStreamRef,
    buffer: MutPtr<u8>,
    buffer_length: CFIndex,
) -> CFIndex {
    use std::io::Read;
	log!("CFReadStreamRead called, stream={:?}, length={}", stream, buffer_length);
    let buf_len: usize = buffer_length.try_into().unwrap();
    let host_obj = env.objc.borrow_mut::<CFReadStreamHostObject>(stream);

    let Some(file) = host_obj.file.as_mut() else {
        log!("CFReadStreamRead: stream is not open");
        return -1;
    };

    // Read into a host-side buffer first, then copy into guest memory.
    let mut tmp = vec![0u8; buf_len];
    match file.read(&mut tmp) {
        Ok(n) => {
			let dest = env.mem.bytes_at_mut(buffer, n.try_into().unwrap());
			dest.copy_from_slice(&tmp[..n]);
			log!("CFReadStreamRead: read {} bytes", n);
			n as CFIndex
		}
        Err(e) => {
            log!("CFReadStreamRead error: {}", e);
            -1
        }
    }
}

fn CFReadStreamClose(env: &mut Environment, stream: CFReadStreamRef) {
    let host_obj = env.objc.borrow_mut::<CFReadStreamHostObject>(stream);
    log_dbg!("CFReadStreamClose('{}')", host_obj.path);
    host_obj.file = None;
}

fn CFReadStreamGetStatus(env: &mut Environment, stream: CFReadStreamRef) -> i32 {
    let host_obj = env.objc.borrow::<CFReadStreamHostObject>(stream);
    if host_obj.file.is_some() {
        kCFStreamStatusOpen
    } else {
        kCFStreamStatusError
    }
}

fn CFReadStreamCopyProperty(
    _env: &mut Environment,
    _stream: CFReadStreamRef,
    _property_name: id,
) -> id {
    log!("TODO: CFReadStreamCopyProperty");
    id::null()
}

fn CFReadStreamGetError(
    _env: &mut Environment,
    _stream: CFReadStreamRef,
) -> id {
    log!("TODO: CFReadStreamGetError");
    id::null()
}

fn CFURLCreatePropertyFromResource(
    env: &mut Environment,
    _url: id,
    _property: id,
    _error_code: id,
) -> id {
    // Return a non-zero file size so the caller proceeds normally.
    // The actual size doesn't matter for Zenonia's usage.
    log!("TODO: CFURLCreatePropertyFromResource - returning dummy size 4096");
    let file_size: i32 = 16384;
    let ns_number: id = msg_class![env; NSNumber numberWithInt:file_size];
    msg![env; ns_number retain]
}

fn CFReadStreamHasBytesAvailable(env: &mut Environment, stream: CFReadStreamRef) -> bool {
    let host_obj = env.objc.borrow::<CFReadStreamHostObject>(stream);
    let available = host_obj.file.is_some();
    log!("CFReadStreamHasBytesAvailable({:?}) -> {}", stream, available);
    available
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFReadStreamCreateWithFile(_, _)),
    export_c_func!(CFReadStreamOpen(_)),
    export_c_func!(CFReadStreamRead(_, _, _)),
    export_c_func!(CFReadStreamClose(_)),
    export_c_func!(CFReadStreamGetStatus(_)),
    export_c_func!(CFReadStreamCopyProperty(_, _)),
    export_c_func!(CFReadStreamGetError(_)),
    export_c_func!(CFURLCreatePropertyFromResource(_, _, _)),
	export_c_func!(CFReadStreamHasBytesAvailable(_)),
];

/// The ObjC class backing CFReadStream objects.
/// We need this so that CFRelease / retain-counting works correctly
/// (CFRelease calls [obj release] under the hood in touchHLE).
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation _touchHLE_CFReadStream: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(CFReadStreamHostObject {
        path: String::new(),
        file: None,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// CFRelease will call [stream release] which calls dealloc when rc hits 0.
// The default NSObject dealloc is fine — our HostObject Drop will clean up.

@end

};