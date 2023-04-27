/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Handling of `@synchronized` blocks (`objc_sync_enter/exit`).
//!
//! `@synchronized` blocks are sections of code that, for a given object, only allow one thread inside any `@synchronized` block with that object. 
//! These are internally implemented with the `objc_sync_enter` and `objc_sync_exit` functions.
//!
//! Resources:
//! - [Section about `@synchronized` in *The Objective-C Programming
//! Language*](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjectiveC/Chapters/ocThreading.html#//apple_ref/doc/uid/TP30001163-CH19-SW1)
//! - [Source code for
//! `objc_sync_enter/exit`](https://opensource.apple.com/source/objc4/objc4-551.1/runtime/Accessors.subproj/objc-accessors.mm.auto.html),
//! otherwise undocumented.
use std::num::NonZeroU32;

use crate::environment::Environment;

use super::id;

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
