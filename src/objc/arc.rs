/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! ARC (Automatic Reference Counting) runtime entry points.
//!
//! Every iOS 5+ binary compiled with ARC calls these libobjc helpers
//! directly instead of sending `retain`/`release` messages. They are thin
//! wrappers around the existing message-based retain/release machinery,
//! plus the autorelease-pool and return-value optimisation entry points.
//!
//! References:
//! - clang ARC specification, "Runtime support":
//!   <https://clang.llvm.org/docs/AutomaticReferenceCounting.html#runtime-support>
//! - Apple's open-source objc4 runtime, `NSObject.mm`:
//!   <https://opensource.apple.com/source/objc4/>

use crate::dyld::{export_c_func, export_c_func_aliased, FunctionExports};
use crate::mem::{ConstPtr, ConstVoidPtr, MutVoidPtr, Ptr};
use crate::objc::{id, msg, msg_class, nil, Class, SEL};
use crate::Environment;

// MARK: - Retain / release / autorelease

pub fn objc_retain(env: &mut Environment, obj: id) -> id {
    if !obj.is_null() {
        crate::objc::retain(env, obj);
    }
    obj
}

pub fn objc_release(env: &mut Environment, obj: id) -> id {
    if !obj.is_null() {
        crate::objc::release(env, obj);
    }
    obj
}

/// `id objc_autorelease(id obj)` — objc4 implements it as
/// `obj ? obj->autorelease() : nil`: the object is added to the current
/// autorelease pool and returned unchanged.
pub fn objc_autorelease(env: &mut Environment, obj: id) -> id {
    crate::objc::autorelease(env, obj)
}

pub fn objc_retainAutorelease(env: &mut Environment, obj: id) -> id {
    if !obj.is_null() {
        crate::objc::retain(env, obj);
        crate::objc::autorelease(env, obj);
    }
    obj
}

// MARK: - Return-value optimisation helpers
//
// We do not implement the caller/callee optimisation that elides the
// retain/autorelease pair; the ordinary fallback is used.

pub fn objc_retainAutoreleasedReturnValue(env: &mut Environment, obj: id) -> id {
    crate::objc::retain(env, obj)
}

pub fn objc_autoreleaseReturnValue(env: &mut Environment, obj: id) -> id {
    crate::objc::autorelease(env, obj)
}

pub fn objc_retainAutoreleaseReturnValue(env: &mut Environment, obj: id) -> id {
    objc_retainAutorelease(env, obj)
}

/// `id objc_unsafeClaimAutoreleasedReturnValue(id obj)` — accepts an
/// autoreleased return value and claims it without adding a retain.
/// There is no retain/autorelease elision to undo, so return the object
/// unchanged.
pub fn objc_unsafeClaimAutoreleasedReturnValue(_env: &mut Environment, obj: id) -> id {
    obj
}

// MARK: - Autorelease pools

/// The opaque token is a retained NSAutoreleasePool. Its implementation
/// already tracks nested pools per guest thread and drains inner pools first.
pub fn objc_autoreleasePoolPush(env: &mut Environment) -> MutVoidPtr {
    let pool: id = msg_class![env; NSAutoreleasePool new];
    pool.cast_void()
}

pub fn objc_autoreleasePoolPop(env: &mut Environment, context: MutVoidPtr) {
    if !context.is_null() {
        let pool: id = context.cast();
        let (): () = msg![env; pool drain];
    }
}

// MARK: - Strong stores and blocks

/// `void objc_storeStrong(id *location, id obj)` — ARC's strong-store
/// runtime helper. Canonical implementation per the clang ARC spec:
///
/// ```text
/// id prev = *location;
/// if (prev == obj) return;
/// objc_retain(obj);
/// *location = obj;
/// objc_release(prev);
/// ```
pub fn objc_storeStrong(env: &mut Environment, location: crate::mem::MutPtr<id>, obj: id) {
    if location.is_null() {
        return;
    }
    let prev = env.mem.read(location);
    if prev == obj {
        return;
    }
    if !obj.is_null() {
        crate::objc::retain(env, obj);
    }
    env.mem.write(location, obj);
    if !prev.is_null() {
        crate::objc::release(env, prev);
    }
}

/// `id objc_retainBlock(id block)` — ARC runtime helper for retaining a
/// block: `return (id)_Block_copy(x);`. ARC emits this whenever a block
/// is stored, so a stack block has to be promoted to the heap here like
/// in `_Block_copy` (see `src/libc/blocks.rs`).
pub fn objc_retainBlock(env: &mut Environment, block: id) -> id {
    let copy = crate::libc::blocks::_Block_copy(env, block.cast().cast_const());
    copy.cast_mut().cast()
}

// MARK: - Allocator fast paths

/// `id objc_alloc(Class cls)` — ARC/libobjc fast-path allocator,
/// equivalent to `[cls alloc]` (including honouring any class that
/// provides its own `+alloc`).
pub fn objc_alloc(env: &mut Environment, cls: Class) -> id {
    if cls.is_null() {
        return nil;
    }
    msg![env; cls alloc]
}

/// `id objc_allocWithZone(Class cls)` — same as [objc_alloc]; the zone
/// argument is a fossil and the guest passes it in `objc_msgSend` form
/// anyway.
pub fn objc_allocWithZone(env: &mut Environment, cls: Class) -> id {
    objc_alloc(env, cls)
}

// MARK: - Block trampolines

/// `IMP imp_implementationWithBlock(id block)` — returns an IMP that
/// invokes `block` when called. The block is copied by the runtime and
/// its signature is `^(id self, method_args...)` (no `_cmd` argument).
/// Like Apple's `objc-blocktramps-arm.s`, the returned IMP overwrites
/// `_cmd` with `self` and `self` with the block, then tail-calls the
/// block's invoke function:
///
/// ```text
/// mov r1, r0            ; _cmd = self
/// ldr r0, [pc, #0]      ; self = block literal
/// ldr pc, [r0, #12]     ; tail-call block->invoke (interworking)
/// <block literal>       ; literal pool
/// ```
pub fn imp_implementationWithBlock(env: &mut Environment, block: id) -> ConstVoidPtr {
    const BLOCK_USE_STRET: u32 = 1 << 29;

    if block.to_bits() == 0 {
        return ConstVoidPtr::from_bits(0);
    }
    let block = crate::libc::blocks::_Block_copy(env, block.cast().cast_const());
    let flags: u32 = env.mem.read(block.cast::<u32>() + 1);
    let tramp: crate::mem::MutPtr<u32> = env.mem.alloc(16).cast();
    if flags & BLOCK_USE_STRET != 0 {
        env.mem.write(tramp + 0, 0xe1a0_2001); // mov r2, r1
        env.mem.write(tramp + 1, 0xe59f_1000); // ldr r1, [pc, #0]
        env.mem.write(tramp + 2, 0xe591_f00c); // ldr pc, [r1, #12]
    } else {
        env.mem.write(tramp + 0, 0xe1a0_1000); // mov r1, r0
        env.mem.write(tramp + 1, 0xe59f_0000); // ldr r0, [pc, #0]
        env.mem.write(tramp + 2, 0xe590_f00c); // ldr pc, [r0, #12]
    }
    env.mem.write(tramp + 3, block.to_bits()); // literal: block literal
    tramp.cast().cast_const()
}

// MARK: - Message forwarding and exception personality

/// `_objc_msgForward` — the ObjC runtime's message forwarding trampoline.
/// We don't implement the forwarding chain; the stub returns nil, matching
/// sending a message to nil, and lets binaries with external relocations
/// to `__objc_msgForward` link without errors.
pub fn objc_msgForward(_env: &mut Environment, _receiver: id, _sel: SEL) -> id {
    log_dbg!(
        "_objc_msgForward called (receiver={:?}, sel={:?}) — returning nil",
        _receiver,
        _sel
    );
    nil
}

/// `_objc_msgForward_stret` — struct-return variant of `_objc_msgForward`.
pub fn objc_msgForward_stret(
    _env: &mut Environment,
    _stret: MutVoidPtr,
    _receiver: id,
    _sel: SEL,
) {
    log_dbg!(
        "_objc_msgForward_stret called (receiver={:?}, sel={:?}) — no-op",
        _receiver,
        _sel
    );
}

/// `void _objc_deallocOnMainThreadHelper(void *context)` — used by
/// libobjc to release an object back on the main thread when it is
/// released on a background thread. Execution is serialised on one host
/// thread, so "release on the main thread" reduces to "release now".
#[allow(non_snake_case)]
pub fn __objc_deallocOnMainThreadHelper(env: &mut Environment, context: MutVoidPtr) {
    let obj: id = Ptr::from_bits(context.to_bits());
    if !obj.is_null() {
        crate::objc::release(env, obj);
    }
}

/// `___objc_personality_v0` — the ObjC/C++ exception personality
/// routine invoked by `_Unwind_RaiseException` for every frame. Exception
/// handling is not fully implemented; returning `_URC_FATAL_PHASE1_ERROR`
/// keeps the unwinder from branching to NULL.
pub fn ___objc_personality_v0(
    _env: &mut Environment,
    version: i32,
    actions: i32,
    exception_class: u64,
    exception_object: MutVoidPtr,
    context: MutVoidPtr,
) -> i32 {
    log!(
        "___objc_personality_v0 called! Exception handling is not fully implemented.\n\
        version: {}, actions: {}, class: {:x}, object: {:?}, context: {:?}",
        version,
        actions,
        exception_class,
        exception_object,
        context
    );
    // _URC_FATAL_PHASE1_ERROR
    3
}

/// `void objc_enumerationMutation(id obj)` — called by the runtime when
/// a mutable collection is mutated during fast enumeration. We don't
/// track mutation state; just log it.
pub fn objc_enumerationMutation(_env: &mut Environment, obj: id) {
    log!(
        "Warning: objc_enumerationMutation({:?}) — collection mutated during fast enumeration",
        obj
    );
}

// MARK: - Class-name lookup helpers

/// `const char *class_getName(Class cls)` — returns the C-string class
/// name. The pointer is stable for the lifetime of the class on real
/// objc4; we cache the allocation in a side table.
pub fn class_getName(env: &mut Environment, cls: Class) -> ConstPtr<u8> {
    if cls.is_null() {
        return Ptr::null();
    }
    if let Some(&cached) = env.objc.class_name_cstrs.get(&cls) {
        return cached;
    }
    let name = env.objc.get_class_name(cls).to_owned();
    let bytes = name.as_bytes();
    let buf: crate::mem::MutPtr<u8> = env.mem.alloc(bytes.len() as u32 + 1).cast();
    for (i, &b) in bytes.iter().enumerate() {
        env.mem.write(buf + i as u32, b);
    }
    env.mem.write(buf + bytes.len() as u32, 0);
    let buf = buf.cast_const();
    env.objc.class_name_cstrs.insert(cls, buf);
    buf
}

/// `Class objc_lookUpClass(const char *name)` — like `objc_getClass`
/// but does not invoke the class handler if the class is missing
/// (returns nil instead). For our runtime the behaviour is identical.
pub fn objc_lookUpClass(env: &mut Environment, name: ConstPtr<u8>) -> Class {
    crate::objc::classes::objc_getClass(env, name).cast()
}

/// `Class objc_getRequiredClass(const char *name)` — same as
/// `objc_getClass` but documented to abort on miss. We degrade to the
/// lenient lookup; returning nil keeps the guest alive.
pub fn objc_getRequiredClass(env: &mut Environment, name: ConstPtr<u8>) -> Class {
    crate::objc::classes::objc_getClass(env, name).cast()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(objc_retain(_)),
    export_c_func!(objc_release(_)),
    export_c_func!(objc_autorelease(_)),
    export_c_func!(objc_retainAutorelease(_)),
    export_c_func!(objc_retainAutoreleasedReturnValue(_)),
    export_c_func!(objc_autoreleaseReturnValue(_)),
    export_c_func!(objc_retainAutoreleaseReturnValue(_)),
    export_c_func!(objc_unsafeClaimAutoreleasedReturnValue(_)),
    export_c_func!(objc_autoreleasePoolPush()),
    export_c_func!(objc_autoreleasePoolPop(_)),
    export_c_func!(objc_storeStrong(_, _)),
    export_c_func!(objc_retainBlock(_)),
    export_c_func!(objc_alloc(_)),
    export_c_func!(objc_allocWithZone(_)),
    export_c_func!(imp_implementationWithBlock(_)),
    export_c_func!(objc_msgForward(_, _)),
    export_c_func!(objc_msgForward_stret(_, _, _)),
    export_c_func!(__objc_deallocOnMainThreadHelper(_)),
    // The C symbol is `__objc_personality_v0`; the Mach-O name gets one more
    // leading underscore, so export explicitly to avoid the macro turning the
    // three-underscore Rust ident into a four-underscore symbol name.
    export_c_func_aliased!("__objc_personality_v0", ___objc_personality_v0(_, _, _, _, _)),
    export_c_func!(objc_enumerationMutation(_)),
    export_c_func!(class_getName(_)),
    export_c_func!(objc_lookUpClass(_)),
    export_c_func!(objc_getRequiredClass(_)),
];
