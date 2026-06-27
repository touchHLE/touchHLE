/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! ARC zeroing-weak-reference runtime helpers.
//!
//! Implements `objc_storeWeak`, `objc_loadWeak`, `objc_loadWeakRetained`,
//! `objc_destroyWeak`, `objc_initWeak`, `objc_storeWeakOrNil`,
//! `objc_copyWeak`, `objc_moveWeak`. These functions are called by ARC-
//! compiled code (every iOS 5+ binary that uses `__weak` references).
//!
//! References:
//! - The clang Automatic Reference Counting specification, section
//!   "Runtime support", subsection "Weak references":
//!   <https://clang.llvm.org/docs/AutomaticReferenceCounting.html#runtime-support>
//! - Apple's open-source objc4 runtime, `objc-weak.{h,mm}` on
//!   <https://opensource.apple.com/source/objc4/>
//!
//! Behaviour summary (mirrors clang ARC §7.1.4 and Apple's `objc-weak.mm`):
//! * `objc_storeWeak(location, newObj)` updates the weak slot at
//!   `*location` to refer to `newObj`. On deallocation of `newObj`
//!   every registered `location` is zeroed (`*location = nil`).
//! * `objc_loadWeakRetained(location)` returns `*location` retained
//!   (or `nil` if the referent has already been deallocated).
//! * `objc_destroyWeak(location)` unregisters `*location` and clears
//!   it. After this point the slot bytes may be reused by the caller
//!   for non-weak storage.
//!
//! `nil` is treated as "no registration" everywhere — this matches
//! clang's spec which says any of these helpers may be called with a
//! null `newObj` to clear the slot, and that `objc_destroyWeak` on a
//! never-stored slot is a no-op.

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::MutPtr;
use crate::objc::{id, nil, ObjC};
use crate::Environment;

impl ObjC {
    /// Register `location` as a weak slot that points at `target`.
    /// The previous registration of `location` (if any) is removed
    /// first so the side table never holds duplicate entries for one
    /// slot.
    fn register_weak(&mut self, target: id, location: MutPtr<id>) {
        if target == nil {
            return;
        }
        self.weak_refs
            .entry(target)
            .or_default()
            .push(location);
    }

    /// Remove `location` from the weak entry of whatever object it
    /// currently points to.
    ///
    /// We don't track the reverse mapping (slot → object) because the
    /// weak side table is rarely large enough for it to matter and
    /// the slot value tells us which list to walk. We require the
    /// caller to pass the previous referent.
    fn unregister_weak(&mut self, target: id, location: MutPtr<id>) {
        if target == nil {
            return;
        }
        if let Some(list) = self.weak_refs.get_mut(&target) {
            list.retain(|&l| l != location);
            if list.is_empty() {
                self.weak_refs.remove(&target);
            }
        }
    }

    /// Called by `dealloc_object` for every object that is being torn
    /// down. Walks the entry for the object and writes `nil` into
    /// every registered weak slot. After this method returns no slot
    /// references the object any more, so any subsequent
    /// `objc_loadWeak` for those slots correctly returns `nil`.
    pub(crate) fn zero_weak_references_for(
        &mut self,
        object: id,
        mem: &mut crate::mem::Mem,
    ) {
        if let Some(list) = self.weak_refs.remove(&object) {
            for slot in list {
                // It's possible the slot has already been overwritten
                // by guest code (e.g. through a non-ARC store) — only
                // zero it if it still points at us.
                if mem.read(slot) == object {
                    mem.write(slot, nil);
                }
            }
        }
    }
}

/// `id objc_storeWeak(id *location, id newObj)`
///
/// Per the ARC runtime contract: register `location` as a weak slot
/// referring to `newObj`, after first unregistering it from any
/// previous referent. Returns `newObj`.
pub fn objc_storeWeak(
    env: &mut Environment,
    location: MutPtr<id>,
    new_obj: id,
) -> id {
    if location.is_null() {
        return new_obj;
    }
    let old = env.mem.read(location);
    if old != nil {
        env.objc.unregister_weak(old, location);
    }
    env.mem.write(location, new_obj);
    if new_obj != nil {
        env.objc.register_weak(new_obj, location);
    }
    new_obj
}

/// `id objc_storeWeakOrNil(id *location, id newObj)` — same as above
/// but documented to tolerate a deallocating object (returns nil
/// instead of crashing). touchHLE never enters the "deallocating but
/// not yet deallocated" intermediate state, so we forward.
pub fn objc_storeWeakOrNil(
    env: &mut Environment,
    location: MutPtr<id>,
    new_obj: id,
) -> id {
    objc_storeWeak(env, location, new_obj)
}

/// `id objc_initWeak(id *location, id newObj)` — same as
/// `objc_storeWeak` except `*location` is assumed to be uninitialised
/// (so we don't try to unregister a previous value).
pub fn objc_initWeak(
    env: &mut Environment,
    location: MutPtr<id>,
    new_obj: id,
) -> id {
    if location.is_null() {
        return new_obj;
    }
    env.mem.write(location, new_obj);
    if new_obj != nil {
        env.objc.register_weak(new_obj, location);
    }
    new_obj
}

/// `id objc_loadWeakRetained(id *location)` — read the weak slot,
/// retain the result, return it (or `nil` if the referent has been
/// deallocated). On real ARC this is the only operation that the
/// compiler emits for reading a `__weak` variable; `objc_loadWeak`
/// just calls this and autoreleases.
pub fn objc_loadWeakRetained(env: &mut Environment, location: MutPtr<id>) -> id {
    if location.is_null() {
        return nil;
    }
    let val = env.mem.read(location);
    if val == nil {
        return nil;
    }
    crate::objc::retain(env, val);
    val
}

/// `id objc_loadWeak(id *location)` — autoreleased version of
/// `objc_loadWeakRetained`.
pub fn objc_loadWeak(env: &mut Environment, location: MutPtr<id>) -> id {
    let val = objc_loadWeakRetained(env, location);
    if val == nil {
        return nil;
    }
    crate::objc::autorelease(env, val)
}

/// `void objc_destroyWeak(id *location)` — unregister and clear the
/// slot. Called from the ivar destructor of any class that has a
/// `__weak` instance variable, and at the end of any function with a
/// `__weak` local.
pub fn objc_destroyWeak(env: &mut Environment, location: MutPtr<id>) {
    if location.is_null() {
        return;
    }
    let old = env.mem.read(location);
    if old != nil {
        env.objc.unregister_weak(old, location);
        env.mem.write(location, nil);
    }
}

/// `void objc_copyWeak(id *to, id *from)` — initialises `*to` as a
/// weak slot pointing at the same object that `*from` does, without
/// disturbing `*from`. Used for `__weak` ivars during object copy.
pub fn objc_copyWeak(
    env: &mut Environment,
    to: MutPtr<id>,
    from: MutPtr<id>,
) {
    if to.is_null() || from.is_null() {
        return;
    }
    let val = env.mem.read(from);
    objc_initWeak(env, to, val);
}

/// `void objc_moveWeak(id *to, id *from)` — moves the weak
/// registration: `*to` becomes the live slot and `*from` is left
/// uninitialised. Per the ARC spec we must update the side table so
/// the deallocation walker zeroes `*to`, not `*from`.
pub fn objc_moveWeak(
    env: &mut Environment,
    to: MutPtr<id>,
    from: MutPtr<id>,
) {
    if to.is_null() || from.is_null() {
        return;
    }
    let val = env.mem.read(from);
    if val != nil {
        // Atomically: unregister `from`, register `to`.
        env.objc.unregister_weak(val, from);
        env.objc.register_weak(val, to);
    }
    env.mem.write(to, val);
    env.mem.write(from, nil);
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(objc_storeWeak(_, _)),
    export_c_func!(objc_storeWeakOrNil(_, _)),
    export_c_func!(objc_initWeak(_, _)),
    export_c_func!(objc_loadWeak(_)),
    export_c_func!(objc_loadWeakRetained(_)),
    export_c_func!(objc_destroyWeak(_)),
    export_c_func!(objc_copyWeak(_, _)),
    export_c_func!(objc_moveWeak(_, _)),
];
