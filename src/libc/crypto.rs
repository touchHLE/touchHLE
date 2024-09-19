/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! CommonCrypto and friends

use crate::dyld::FunctionExports;
use crate::mem::{ConstVoidPtr, MutPtr};
use crate::{export_c_func, Environment};
use std::ops::Deref;

fn CC_MD5(env: &mut Environment, data: ConstVoidPtr, len: u32, md: MutPtr<u8>) -> MutPtr<u8> {
    let digest = md5::compute(env.mem.bytes_at(data.cast(), len));
    env.mem.bytes_at_mut(md, 16).copy_from_slice(digest.deref());
    md
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(CC_MD5(_, _, _))];
