/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGDataProvider.h`

use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::frameworks::core_foundation::cf_data::CFDataRef;
use crate::frameworks::core_foundation::{CFRelease, CFRetain, CFTypeRef};
use crate::frameworks::foundation::NSUInteger;
use crate::mem::{ConstVoidPtr, GuestUSize, MutVoidPtr};
use crate::objc::{id, msg, msg_class, objc_classes, ClassExports, HostObject};
use crate::Environment;

pub type CGDataProviderRef = CFTypeRef;

/// `(*void)(void *info, const void *data, size_t size)`
type CGDataProviderReleaseDataCallback = GuestFunction;

pub(super) struct CGDataProviderHostObject {
    pub(super) data: ConstVoidPtr,
    pub(super) size: GuestUSize,
    /// User-provided pointer passed to release callback.
    info: MutVoidPtr,
    release_callback: CGDataProviderReleaseDataCallback,
}
impl HostObject for CGDataProviderHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// CGDataProvider is a CFType-based type, but in our implementation those
// are just Objective-C types, so we need a class for it, but its name is not
// visible anywhere.
@implementation _touchHLE_CGDataProvider: NSObject

- (())dealloc {
    let &CGDataProviderHostObject {
        info,
        data,
        size,
        release_callback,
    } = env.objc.borrow(this);

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
        Box::new(CGDataProviderHostObject {
            info,
            data,
            size,
            release_callback,
        }),
        &mut env.mem,
    )
}

fn CGDataProviderCopyData(env: &mut Environment, provider: CGDataProviderRef) -> CFDataRef {
    // Hack: CGDataProviderRef is assumed to actually be a CGImageRef.
    // See CGImageGetDataProvider() implementation.
    let bytes =
        crate::frameworks::core_graphics::cg_image::borrow_image(&env.objc, provider).pixels();

    let len: NSUInteger = bytes.len().try_into().unwrap();
    let alloc = env.mem.alloc(len);
    env.mem
        .bytes_at_mut(alloc.cast(), len)
        .copy_from_slice(bytes);

    // TODO: it would be cleaner to use CFDataCreateWithBytesNoCopy, but that's
    // a bit more tricky.
    let ns_data: id = msg_class![env; NSData alloc];
    let ns_data: id = msg![env; ns_data initWithBytesNoCopy:alloc length:len];
    ns_data
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CGDataProviderRetain(_)),
    export_c_func!(CGDataProviderRelease(_)),
    export_c_func!(CGDataProviderCreateWithData(_, _, _, _)),
    export_c_func!(CGDataProviderCopyData(_)),
];
