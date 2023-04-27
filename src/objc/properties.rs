/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Handling of Objective-C properties.
//!
//! Note that these are not the same as instance variables (ivars), though
//! they're closely related, so maybe this file will end up being used for those
//! too.
//!
//! Resources:
//! - `objc_setProperty` and friends are not documented, so [reading the source code](https://opensource.apple.com/source/objc4/objc4-551.1/runtime/Accessors.subproj/objc-accessors.mm.auto.html) is useful.
//!
//! See also: [crate::frameworks::foundation::ns_object].

use std::num::NonZeroU32;

use super::{id, msg, nil, release, retain, SEL};
use crate::mem::{ConstVoidPtr, GuestISize, GuestUSize, MutPtr, MutVoidPtr, Ptr};
use crate::Environment;

/// Undocumented function (see link above) apparently used by auto-generated
/// methods for properties to set an ivar and handle reference counting, copying
/// and locking.
pub(super) fn objc_setProperty(
    env: &mut Environment,
    this: id,
    _cmd: SEL,
    offset: GuestISize,
    value: id,
    atomic: bool,
    should_copy: i8,
) {
    // We currently aren't touching the ivar layouts contained in the binary, so
    // we are assuming they are already correctly set by the compiler. Since we
    // aren't using ivars at all in our host classes, we shouldn't have any
    // issues with host classes' ivars clobbering guest classes' ivars, but
    // what if the compiler doesn't set the ivar layout at all? This is a simple
    // safeguard: any real ivar offset will be after the isa pointer.
    assert!(offset >= 4);

    assert!(!atomic); // what do we do with this?

    let ivar: MutPtr<id> = Ptr::from_bits(this.to_bits().checked_add_signed(offset).unwrap());
    let old = env.mem.read(ivar);

    let void_null: MutVoidPtr = Ptr::null();
    let value: id = if value != nil {
        match should_copy {
            0 => retain(env, value),
            1 => msg![env; value copyWithZone:void_null],
            2 => msg![env; value mutableCopyWithZone:void_null],
            // Apple's source code implies that any non-zero value that isn't 2
            // should mean "copy", but that seems weird, let's be conservative.
            _ => panic!("Unknown \"should copy\" value: {}", should_copy),
        }
    } else {
        nil
    };
    env.mem.write(ivar, value);

    if old != nil {
        release(env, old);
    }
}

// note: https://opensource.apple.com/source/objc4/objc4-723/runtime/objc-accessors.mm.auto.html
//       says that hasStrong is unused.
pub(super) fn objc_copyStruct(
    env: &mut Environment,
    dest: MutVoidPtr,
    src: ConstVoidPtr,
    size: GuestUSize,
    atomic: bool,
    _hasStrong: bool,
) {
    // TODO: implement atomic support
    assert!(!atomic);
    env.mem.memmove(dest, src, size);
}

/// Backing function of @synchronized block entry.
/// This function is entirely undocumented, with
/// [source code provided](https://opensource.apple.com/source/objc4/objc4-551.1/runtime/objc-sync.h.auto.html).
pub(super) fn objc_sync_enter(env: &mut Environment, obj: id) -> u32 {
    match env.objc.sync_state.get_mut(&obj) {
        Some(sync_data) if sync_data.0 == env.current_thread => {
            sync_data.1 = sync_data.1.checked_add(1).unwrap();
            log_dbg!("Re-entry of {:?} to synchronized", obj);
        }
        Some(_) => {
            // TODO: block thread here
            unimplemented!("Attempted cross-thread @synchronized of {:?}", obj);
        }
        None => {
            env.objc
                .sync_state
                .insert(obj, (env.current_thread, NonZeroU32::new(1).unwrap()));
            log_dbg!("Added {:?} to synchronized", obj);
        }
    }
    0u32 // OK
}

/// Backing function of @synchronized block exit.
/// This function is entirely undocumented, with
/// [source code provided](https://opensource.apple.com/source/objc4/objc4-551.1/runtime/objc-sync.h.auto.html).
pub(super) fn objc_sync_exit(env: &mut Environment, obj: id) -> u32 {
    match env.objc.sync_state.get_mut(&obj) {
        Some((tid, count)) if *tid == env.current_thread => {
            if count.get() == 1 {
                env.objc.sync_state.remove(&obj);
                log_dbg!("Regular @synchronized block exit for {:?}, unlocked", obj);
            } else {
                *count = NonZeroU32::new(count.get() - 1).unwrap();
                log_dbg!(
                    "Regular @synchronized block exit for {:?}: {} locks remain",
                    obj,
                    count.get()
                );
            }
        }
        Some(_) => {
            panic!(
                "Attempted exit of @synchronized block for {:?} not owned by current thread",
                obj
            );
            // See below.
        }
        None => {
            panic!(
                "Attempt to exit from @synchronized block for {:?} that was not entered properly",
                obj
            );
            // Should return an error (non-zero), although I don't think it's ever checked?
            // Something probably went wrong to get here.
        }
    }

    0u32 // OK
}
