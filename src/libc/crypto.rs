/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! CommonCrypto and friends

use crate::dyld::FunctionExports;
use crate::mem::{ConstVoidPtr, MutPtr};
use crate::{export_c_func, Environment};
use digest::Digest;
use md5::Md5;
use sha1::Sha1;
use std::collections::HashMap;

// TODO: struct definition
#[allow(non_camel_case_types)]
struct CC_MD5_CTX {}

#[derive(Default)]
pub struct State {
    md5_contexts: HashMap<MutPtr<CC_MD5_CTX>, Md5>,
}
impl State {
    fn get_mut(env: &mut Environment) -> &mut Self {
        &mut env.libc_state.crypto
    }
}

fn CC_MD5_Init(env: &mut Environment, ctx: MutPtr<CC_MD5_CTX>) -> i32 {
    log_once!("Warning: CC_MD5_Init doesn't update side effects! (internal changes to CC_MD5_CTX are not done)");
    assert!(!State::get_mut(env).md5_contexts.contains_key(&ctx));
    State::get_mut(env).md5_contexts.insert(ctx, Md5::new());
    1 // success
}

fn CC_MD5_Update(
    env: &mut Environment,
    ctx: MutPtr<CC_MD5_CTX>,
    data: ConstVoidPtr,
    len: u32,
) -> i32 {
    log_once!("Warning: CC_MD5_Update doesn't update side effects! (internal changes to CC_MD5_CTX are not done)");
    let hasher = env.libc_state.crypto.md5_contexts.get_mut(&ctx).unwrap();
    hasher.update(env.mem.bytes_at(data.cast(), len));
    1 // success
}

fn CC_MD5_Final(env: &mut Environment, md: MutPtr<u8>, ctx: MutPtr<CC_MD5_CTX>) -> i32 {
    log_once!("Warning: CC_MD5_Final doesn't update side effects! (internal changes to CC_MD5_CTX are not done)");
    let hasher = State::get_mut(env).md5_contexts.remove(&ctx).unwrap();
    let digest = hasher.finalize();
    env.mem.bytes_at_mut(md, 16).copy_from_slice(&digest[..]);
    1 // success
}

fn CC_MD5(env: &mut Environment, data: ConstVoidPtr, len: u32, md: MutPtr<u8>) -> MutPtr<u8> {
    let mut hasher = Md5::new();
    hasher.update(env.mem.bytes_at(data.cast(), len));
    let digest = hasher.finalize();
    env.mem.bytes_at_mut(md, 16).copy_from_slice(&digest[..]);
    md
}

fn CC_SHA1(env: &mut Environment, data: ConstVoidPtr, len: u32, md: MutPtr<u8>) -> MutPtr<u8> {
    let mut hasher = Sha1::new();
    hasher.update(env.mem.bytes_at(data.cast(), len));
    let digest = hasher.finalize();
    env.mem.bytes_at_mut(md, 20).copy_from_slice(&digest[..]);
    md
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(CC_MD5_Init(_)),
    export_c_func!(CC_MD5_Update(_, _, _)),
    export_c_func!(CC_MD5_Final(_, _)),
    export_c_func!(CC_MD5(_, _, _)),
    export_c_func!(CC_SHA1(_, _, _)),
];
