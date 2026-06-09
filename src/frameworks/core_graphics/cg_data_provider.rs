/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGDataProvider.h`

use super::cg_image::{self, CGImageRef, CGImageRelease, CGImageRetain};
use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::frameworks::core_foundation::cf_allocator::kCFAllocatorDefault;
use crate::frameworks::core_foundation::cf_data::{
    CFDataCreate, CFDataGetBytePtr, CFDataGetLength, CFDataRef,
};
use crate::frameworks::core_foundation::cf_url::CFURLRef;
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::foundation::ns_string::to_rust_string;
use crate::frameworks::foundation::NSUInteger;
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, SafeRead};
use crate::objc::{id, msg, msg_class, objc_classes, ClassExports, HostObject};
use crate::Environment;

pub type CGDataProviderRef = CFTypeRef;

/// `(*void)(void *info, const void *data, size_t size)`
type CGDataProviderReleaseDataCallback = GuestFunction;

///  `(*size_t)(void *info, void *buffer, size_t count)`
type CGDataProviderGetBytesCallback = GuestFunction;
///  `(*off_t)(void *info, off_t count)`
type CGDataProviderSkipForwardCallback = GuestFunction;
///  `(*void)(void *info)`
type CGDataProviderRewindCallback = GuestFunction;
///  `(*void)(void *info)`
type CGDataProviderReleaseInfoCallback = GuestFunction;

#[repr(C, packed)]
struct CGDataProviderSequentialCallbacks {
    version: u32,
    get_bytes: CGDataProviderGetBytesCallback,
    skip_forward: CGDataProviderSkipForwardCallback,
    rewind: CGDataProviderRewindCallback,
    release_info: CGDataProviderReleaseInfoCallback,
}
unsafe impl SafeRead for CGDataProviderSequentialCallbacks {}

// A CGDataProvider is supposed to be a collection of callbacks used for
// accessing data, but at least for now, we instead only support some specific
// use-cases.

enum CGDataProviderHostObject {
    DataWithSize {
        data: ConstVoidPtr,
        size: GuestUSize,
        /// User-provided pointer passed to release callback.
        info: MutVoidPtr,
        release_callback: CGDataProviderReleaseDataCallback,
    },
    // TODO: Maybe we should store image data in guest memory so we don't
    // need a special variant for this.
    CGImage(CGImageRef),
    CFData(CFDataRef),
}
impl HostObject for CGDataProviderHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// CGDataProvider is a CFType-based type, but in our implementation those
// are just Objective-C types, so we need a class for it, but its name is not
// visible anywhere.
@implementation _touchHLE_CGDataProvider: NSObject

- (())dealloc {
    match *env.objc.borrow(this) {
        CGDataProviderHostObject::DataWithSize {
            info,
            data,
            size,
            release_callback,
        } => {
            if !release_callback.to_ptr().is_null() {
                let args: (MutVoidPtr, ConstVoidPtr, GuestUSize) = (info, data, size);
                log_dbg!(
                    "Freeing {:?}, calling release callback {:?} with {:?}",
                    this,
                    release_callback,
                    args,
                );
                () = release_callback.call_from_host(env, args);
            }
        },
        CGDataProviderHostObject::CGImage(cg_image) => CGImageRelease(env, cg_image),
        CGDataProviderHostObject::CFData(cf_data) => CFRelease(env, cf_data),
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

pub fn CGDataProviderRelease(env: &mut Environment, c: CGDataProviderRef) {
    if !c.is_null() {
        CFRelease(env, c);
    }
}
pub fn CGDataProviderRetain(env: &mut Environment, c: CGDataProviderRef) -> CGDataProviderRef {
    if !c.is_null() {
        CFRetain(env, c)
    } else {
        c
    }
}

fn CGDataProviderCreateSequential(
    env: &mut Environment,
    info: MutVoidPtr,
    callbacks: ConstPtr<CGDataProviderSequentialCallbacks>,
) -> CGDataProviderRef {
    let callbacks = env.mem.read(callbacks);
    let version = callbacks.version;
    assert_eq!(version, 0);

    // TODO: use rewind callback
    let get_bytes_callback = callbacks.get_bytes;
    let release_info = callbacks.release_info;

    // We are reading all data at once in chunks
    // TODO: implement proper sequential provider
    let chunk_size = 1024;
    let chunk = env.mem.alloc(chunk_size);
    let mut bytes: Vec<u8> = Vec::new();
    loop {
        let read: GuestUSize = get_bytes_callback.call_from_host(env, (info, chunk, chunk_size));
        if read == 0 {
            break;
        }
        assert!(read <= chunk_size); // safeguard
        bytes.extend_from_slice(env.mem.bytes_at(chunk.cast(), read));
    }
    env.mem.free(chunk);

    // TODO: Technically, we should release at dealloc.
    // Does it really matter?
    if !release_info.to_ptr().is_null() {
        () = release_info.call_from_host(env, (info,));
    }

    let total_size: GuestUSize = bytes.len().try_into().unwrap();
    let data: MutPtr<u8> = env.mem.alloc(total_size).cast();
    env.mem
        .bytes_at_mut(data, total_size)
        .copy_from_slice(&bytes);

    let cf_data = CFDataCreate(
        env,
        kCFAllocatorDefault,
        data.cast().cast_const(),
        total_size.try_into().unwrap(),
    );
    env.mem.free(data.cast());
    CGDataProviderCreateWithCFData(env, cf_data)
}

fn CGDataProviderCreateWithData(
    env: &mut Environment,
    info: MutVoidPtr,
    data: ConstVoidPtr,
    size: GuestUSize,
    release_callback: CGDataProviderReleaseDataCallback,
) -> CGDataProviderRef {
    let class = env
        .objc
        .get_known_class("_touchHLE_CGDataProvider", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CGDataProviderHostObject::DataWithSize {
            info,
            data,
            size,
            release_callback,
        }),
        &mut env.mem,
    )
}

#[allow(rustdoc::broken_intra_doc_links)] // https://github.com/rust-lang/rust/issues/83049
/// This is for use by [super::cg_image::CGImageGetDataProvider].
pub(super) fn from_cg_image(env: &mut Environment, cg_image: CGImageRef) -> CGDataProviderRef {
    CGImageRetain(env, cg_image);
    let class = env
        .objc
        .get_known_class("_touchHLE_CGDataProvider", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CGDataProviderHostObject::CGImage(cg_image)),
        &mut env.mem,
    )
}

/// Generic interface for host code.
pub(super) fn borrow_bytes(env: &mut Environment, provider: CGDataProviderRef) -> &[u8] {
    match *env.objc.borrow(provider) {
        CGDataProviderHostObject::DataWithSize { data, size, .. } => {
            env.mem.bytes_at(data.cast(), size)
        }
        CGDataProviderHostObject::CGImage(cg_image) => {
            cg_image::borrow_image(&env.objc, cg_image).pixels()
        }
        CGDataProviderHostObject::CFData(cf_data) => {
            let data = CFDataGetBytePtr(env, cf_data);
            let size = CFDataGetLength(env, cf_data);
            env.mem.bytes_at(data, size.try_into().unwrap())
        }
    }
}

fn CGDataProviderCopyData(env: &mut Environment, provider: CGDataProviderRef) -> CFDataRef {
    match *env.objc.borrow(provider) {
        CGDataProviderHostObject::DataWithSize { data, size, .. } => CFDataCreate(
            env,
            kCFAllocatorDefault,
            data.cast(),
            size.try_into().unwrap(),
        ),
        CGDataProviderHostObject::CGImage(cg_image) => {
            let bytes = cg_image::borrow_image(&env.objc, cg_image).pixels();

            let len: NSUInteger = bytes.len().try_into().unwrap();
            let alloc = env.mem.alloc(len);
            env.mem
                .bytes_at_mut(alloc.cast(), len)
                .copy_from_slice(bytes);

            // TODO: it would be cleaner to use CFDataCreateWithBytesNoCopy, but
            // that's a bit more tricky.
            let ns_data: id = msg_class![env; NSData alloc];
            msg![env; ns_data initWithBytesNoCopy:alloc length:len]
        }
        CGDataProviderHostObject::CFData(cf_data) => {
            let data = CFDataGetBytePtr(env, cf_data);
            let size = CFDataGetLength(env, cf_data);
            CFDataCreate(env, kCFAllocatorDefault, data.cast(), size)
        }
    }
}

fn CGDataProviderCreateWithFilename(
    env: &mut Environment,
    filename: ConstPtr<u8>,
) -> CGDataProviderRef {
    log_dbg!(
        "CGDataProviderCreateWithFilename('{:?}')",
        env.mem.cstr_at_utf8(filename)
    );
    let path: id = msg_class![env; NSString stringWithCString:filename];
    let data: id = msg_class![env; NSData dataWithContentsOfFile:path];
    CGDataProviderCreateWithCFData(env, data)
}

fn CGDataProviderCreateWithURL(env: &mut Environment, url: CFURLRef) -> CGDataProviderRef {
    assert!(msg![env; url isFileURL]); // TODO
    let path: id = msg![env; url path];
    log_dbg!(
        "CGDataProviderCreateWithURL url path {}",
        to_rust_string(env, path)
    );
    let data: id = msg_class![env; NSData dataWithContentsOfFile:path];
    CGDataProviderCreateWithCFData(env, data)
}

fn CGDataProviderCreateWithCFData(env: &mut Environment, data: CFDataRef) -> CGDataProviderRef {
    CFRetain(env, data);
    let class = env
        .objc
        .get_known_class("_touchHLE_CGDataProvider", &mut env.mem);
    env.objc.alloc_object(
        class,
        Box::new(CGDataProviderHostObject::CFData(data)),
        &mut env.mem,
    )
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGDataProviderRetain(_)),
    export_c_func!(CGDataProviderRelease(_)),
    export_c_func!(CGDataProviderCreateSequential(_, _)),
    export_c_func!(CGDataProviderCreateWithData(_, _, _, _)),
    export_c_func!(CGDataProviderCopyData(_)),
    export_c_func!(CGDataProviderCreateWithFilename(_)),
    export_c_func!(CGDataProviderCreateWithURL(_)),
    export_c_func!(CGDataProviderCreateWithCFData(_)),
];
