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
use crate::frameworks::core_foundation::cf_data::{CFDataCreate, CFDataRef};
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::foundation::NSUInteger;
use crate::mem::{ConstVoidPtr, GuestUSize, MutVoidPtr};
use crate::objc::{id, msg, msg_class, objc_classes, ClassExports, HostObject};
use crate::Environment;

pub type CGDataProviderRef = CFTypeRef;

/// `(*void)(void *info, const void *data, size_t size)`
type CGDataProviderReleaseDataCallback = GuestFunction;

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
            if release_callback.addr_with_thumb_bit() != 0 {
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
pub(super) fn borrow_bytes(env: &Environment, provider: CGDataProviderRef) -> &[u8] {
    match *env.objc.borrow(provider) {
        CGDataProviderHostObject::DataWithSize { data, size, .. } => {
            env.mem.bytes_at(data.cast(), size)
        }
        CGDataProviderHostObject::CGImage(cg_image) => {
            cg_image::borrow_image(&env.objc, cg_image).pixels()
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
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGDataProviderRetain(_)),
    export_c_func!(CGDataProviderRelease(_)),
    export_c_func!(CGDataProviderCreateWithData(_, _, _, _)),
    export_c_func!(CGDataProviderCopyData(_)),
];
