/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CGDataProvider.h`

use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::frameworks::core_foundation::cf_data::CFDataRef;
use crate::frameworks::foundation::NSUInteger;
use crate::objc::{id, msg, msg_class};
use crate::Environment;

pub type CGDataProviderRef = crate::frameworks::core_foundation::CFTypeRef;

fn CGDataProviderCopyData(env: &mut Environment, provider: CGDataProviderRef) -> CFDataRef {
    // Hack: CGDataProviderRef is assumed to actually be a CGImageRef.
    // See CGImageGetDataProvider() implementation.
    let bytes =
        crate::frameworks::core_graphics::cg_image::borrow_image(&env.objc, provider).pixels();

    let len: NSUInteger = bytes.len().try_into().unwrap();
    let alloc = env.mem.alloc(len);
    env.mem.bytes_at_mut(alloc.cast(), len).copy_from_slice(bytes);

    // TODO: it would be cleaner to use CFDataCreateWithBytesNoCopy, but that's
    // a bit more tricky.
    let ns_data: id = msg_class![env; NSData alloc];
    let ns_data: id = msg![env; ns_data initWithBytesNoCopy:alloc length:len];
    ns_data
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(CGDataProviderCopyData(_))];
