/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `libBlocksRuntime` — Apple Blocks ABI helpers.
//!
//! These functions are called by the compiler-generated copy/dispose
//! helpers when an Objective-C block captures a `__strong` ObjC object,
//! a `__weak` reference, another block, or a `__block` storage variable.
//! They are documented in the
//! [Blocks ABI](https://clang.llvm.org/docs/Block-ABI-Apple.html#imported-variables-1).
//!
//! Stack blocks and byref captures are promoted to guest heap storage before
//! asynchronous use. Compiler-generated copy/dispose helpers own captures.

use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstVoidPtr, MutVoidPtr, Ptr};
use crate::objc::{release, retain};
use crate::Environment;

/// Bit-flag values passed to `_Block_object_assign` / `_Block_object_dispose`.
/// See the Blocks ABI document referenced above.
const BLOCK_FIELD_IS_OBJECT: i32 = 3;
const BLOCK_FIELD_IS_BLOCK: i32 = 7;
const BLOCK_FIELD_IS_BYREF: i32 = 8;
const BLOCK_FIELD_IS_WEAK: i32 = 16;
const BLOCK_BYREF_CALLER: i32 = 128;

const BLOCK_NEEDS_FREE: u32 = 1 << 24;
const BLOCK_HAS_COPY_DISPOSE: u32 = 1 << 25;
const BLOCK_IS_GLOBAL: u32 = 1 << 28;
const REFCOUNT_MASK: u32 = 0xffff;

fn add_reference(env: &mut Environment, flags_ptr: crate::mem::MutPtr<u32>) {
    let flags: u32 = env.mem.read(flags_ptr);
    // A saturated reference count is immortal, rather than wrapping to zero.
    if flags & REFCOUNT_MASK != REFCOUNT_MASK {
        env.mem.write(flags_ptr, flags + 1);
    }
}

fn remove_reference(env: &mut Environment, flags_ptr: crate::mem::MutPtr<u32>) -> bool {
    let flags: u32 = env.mem.read(flags_ptr);
    let count = flags & REFCOUNT_MASK;
    if count == REFCOUNT_MASK {
        return false;
    }
    assert!(count != 0, "Releasing a block with zero references");
    env.mem.write(flags_ptr, flags - 1);
    count == 1
}

/// Promote a stack block, or retain an existing heap block. Global blocks
/// are immortal. All offsets below are words in the 32-bit Apple Blocks ABI.
pub fn _Block_copy(env: &mut Environment, block: ConstVoidPtr) -> ConstVoidPtr {
    if block.is_null() {
        return block;
    }
    let words = block.cast::<u32>();
    let flags: u32 = env.mem.read(words + 1);
    if flags & BLOCK_IS_GLOBAL != 0 {
        return block;
    }
    if flags & BLOCK_NEEDS_FREE != 0 {
        add_reference(env, (words + 1).cast_mut());
        return block;
    }
    let descriptor: crate::mem::ConstPtr<u32> = env.mem.read((words + 4).cast());
    let size: u32 = env.mem.read(descriptor + 1);
    assert!(size >= 20, "Invalid block descriptor size");
    let bytes = env.mem.bytes_at(block.cast(), size).to_vec();
    let copy = env.mem.alloc(size);
    env.mem
        .bytes_at_mut(copy.cast(), size)
        .copy_from_slice(&bytes);
    env.mem.write(
        copy.cast::<u32>() + 1,
        (flags & !REFCOUNT_MASK) | BLOCK_NEEDS_FREE | 1,
    );
    if flags & BLOCK_HAS_COPY_DISPOSE != 0 {
        let helper: u32 = env.mem.read(descriptor + 2);
        let helper = GuestFunction::from_addr_with_thumb_bit(helper);
        let (): () = helper.call_from_host(env, (copy, block));
    }
    copy.cast_const()
}

pub fn _Block_release(env: &mut Environment, block: ConstVoidPtr) {
    if block.is_null() {
        return;
    }
    let words = block.cast::<u32>();
    let flags: u32 = env.mem.read(words + 1);
    if flags & BLOCK_IS_GLOBAL != 0 || flags & BLOCK_NEEDS_FREE == 0 {
        return;
    }
    if !remove_reference(env, (words + 1).cast_mut()) {
        return;
    }
    if flags & BLOCK_HAS_COPY_DISPOSE != 0 {
        let descriptor: crate::mem::ConstPtr<u32> = env.mem.read((words + 4).cast());
        let helper: u32 = env.mem.read(descriptor + 3);
        let helper = GuestFunction::from_addr_with_thumb_bit(helper);
        let (): () = helper.call_from_host(env, (block,));
    }
    env.mem.free(block.cast_mut());
}

fn copy_byref(env: &mut Environment, object: ConstVoidPtr) -> ConstVoidPtr {
    let original = object.cast::<u32>();
    let forwarded: crate::mem::MutPtr<u32> = env.mem.read((original + 1).cast());
    let flags: u32 = env.mem.read(forwarded + 2);
    if flags & BLOCK_NEEDS_FREE != 0 {
        add_reference(env, forwarded + 2);
        return forwarded.cast().cast_const();
    }
    let size: u32 = env.mem.read(forwarded + 3);
    assert!(size >= 16, "Invalid byref size");
    let bytes = env
        .mem
        .bytes_at(forwarded.cast().cast_const(), size)
        .to_vec();
    let copy = env.mem.alloc(size).cast::<u32>();
    env.mem
        .bytes_at_mut(copy.cast(), size)
        .copy_from_slice(&bytes);
    // One reference belongs to the stack scope, the other to the copied block.
    env.mem
        .write(copy + 2, (flags & !REFCOUNT_MASK) | BLOCK_NEEDS_FREE | 2);
    env.mem.write((copy + 1).cast(), copy);
    env.mem.write((forwarded + 1).cast(), copy);
    if flags & BLOCK_HAS_COPY_DISPOSE != 0 {
        let helper: u32 = env.mem.read(forwarded + 4);
        let helper = GuestFunction::from_addr_with_thumb_bit(helper);
        let (): () = helper.call_from_host(env, (copy, forwarded));
    }
    copy.cast().cast_const()
}

fn release_byref(env: &mut Environment, object: ConstVoidPtr) {
    let forwarded: crate::mem::MutPtr<u32> = env.mem.read((object.cast::<u32>() + 1).cast());
    let flags: u32 = env.mem.read(forwarded + 2);
    if flags & BLOCK_NEEDS_FREE == 0 || !remove_reference(env, forwarded + 2) {
        return;
    }
    if flags & BLOCK_HAS_COPY_DISPOSE != 0 {
        let helper: u32 = env.mem.read(forwarded + 5);
        let helper = GuestFunction::from_addr_with_thumb_bit(helper);
        let (): () = helper.call_from_host(env, (forwarded,));
    }
    env.mem.free(forwarded.cast());
}

/// `_Block_object_assign(destAddr, object, flags)`. Called by the
/// compiler-generated copy helper to retain `object` and store it at
/// `destAddr`. We perform the retain side-effect via `objc::retain`.
///
/// `flags` is a bitwise OR of `BLOCK_FIELD_IS_*` constants telling us what
/// the captured value is — an ObjC object, another block, or a `__block`
/// storage location. For ObjC objects and blocks we retain; for `__weak`
/// captures we do nothing (per the Blocks ABI).
fn _Block_object_assign(
    env: &mut Environment,
    dest_addr: MutVoidPtr,
    object: ConstVoidPtr,
    flags: i32,
) {
    // Byref copy helpers use BYREF_CALLER for their payload. They must not
    // recursively retain/copy that payload (nor retain weak captures).
    let value = if flags & (BLOCK_FIELD_IS_WEAK | BLOCK_BYREF_CALLER) != 0 {
        if flags & BLOCK_FIELD_IS_BYREF != 0 && !object.is_null() {
            copy_byref(env, object)
        } else {
            object
        }
    } else {
        match flags & 0xf {
            BLOCK_FIELD_IS_OBJECT => {
                retain(env, Ptr::from_bits(object.to_bits()));
                object
            }
            BLOCK_FIELD_IS_BLOCK => _Block_copy(env, object),
            BLOCK_FIELD_IS_BYREF if !object.is_null() => copy_byref(env, object),
            _ => object,
        }
    };
    env.mem.write(dest_addr.cast(), value);
}

fn _Block_object_dispose(env: &mut Environment, object: ConstVoidPtr, flags: i32) {
    if flags & BLOCK_BYREF_CALLER != 0 {
        return;
    }
    if flags & BLOCK_FIELD_IS_BYREF != 0 && !object.is_null() {
        release_byref(env, object);
    } else if flags & BLOCK_FIELD_IS_WEAK == 0 {
        match flags & 0xf {
            BLOCK_FIELD_IS_OBJECT => release(env, Ptr::from_bits(object.to_bits())),
            BLOCK_FIELD_IS_BLOCK => _Block_release(env, object),
            _ => (),
        }
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(_Block_copy(_)),
    export_c_func!(_Block_release(_)),
    export_c_func!(_Block_object_assign(_, _, _)),
    export_c_func!(_Block_object_dispose(_, _)),
];
