/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `CFData.h`

use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::frameworks::core_foundation::{CFIndex, CFRange, CFTypeRef};
use crate::frameworks::core_graphics::cg_image::borrow_image;
use crate::mem::MutPtr;
use crate::objc::ObjC;
use crate::Environment;

pub type CFDataRef = CFTypeRef;

fn CFDataGetLength(env: &mut Environment, data: CFDataRef) -> CFIndex {
    // TODO: actually support general CFDataRef :p
    assert_cgimage(env, data);

    borrow_image(&env.objc, data)
        .pixels()
        .len()
        .try_into()
        .unwrap()
}

fn CFDataGetBytes(env: &mut Environment, data: CFDataRef, range: CFRange, buffer: MutPtr<u8>) {
    // TODO: actually support general CFDataRef :p
    assert_cgimage(env, data);

    let src_pixels = borrow_image(&env.objc, data).pixels();
    let len = src_pixels.len().try_into().unwrap();
    // TODO: respect range
    assert_eq!(len, range.length as u32);
    let _ = &env
        .mem
        .bytes_at_mut(buffer, len)
        .copy_from_slice(src_pixels);
}

fn assert_cgimage(env: &mut Environment, data: CFDataRef) {
    let data_class = ObjC::read_isa(data, &env.mem);
    let cgimage_class = env.objc.get_known_class("_touchHLE_CGImage", &mut env.mem);
    assert!(env.objc.class_is_subclass_of(data_class, cgimage_class));
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CFDataGetLength(_)),
    export_c_func!(CFDataGetBytes(_, _, _)),
];
