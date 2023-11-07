/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Handling of `@synchronized` blocks (`objc_sync_enter/exit`).
//!
//! `@synchronized` blocks are sections of code that, for a given object, only
//! allow one thread inside any `@synchronized` block with that object.
//! These are internally implemented with the `objc_sync_enter` and
//! `objc_sync_exit` functions.
//!
//! Resources:
//! - [Section about `@synchronized` in *The Objective-C Programming Language*](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjectiveC/Chapters/ocThreading.html#//apple_ref/doc/uid/TP30001163-CH19-SW1)
//! - [Source code for `objc_sync_enter/exit`](https://opensource.apple.com/source/objc4/objc4-551.1/runtime/Accessors.subproj/objc-accessors.mm.auto.html), otherwise undocumented.
use crate::{Environment, MutexType};

use super::id;

/// Backing function of @synchronized block entry.
/// This function is entirely undocumented, with
/// [source code provided](https://opensource.apple.com/source/objc4/objc4-551.1/runtime/objc-sync.h.auto.html).
pub(super) fn objc_sync_enter(env: &mut Environment, obj: id) -> i32 {
    if let Some(mutex_id) = env.objc.sync_mutexes.get(&obj) {
        log_dbg!(
            "Reentry of {:#x} to objc_sync_enter, using mutex #{}",
            obj.to_bits(),
            mutex_id
        );
        env.lock_mutex(*mutex_id).unwrap();
    } else {
        let mutex_id = env
            .mutex_state
            .init_mutex(MutexType::PTHREAD_MUTEX_RECURSIVE);
        log_dbg!(
            "Entry of {:#x} to objc_sync_enter, using mutex #{}",
            obj.to_bits(),
            mutex_id
        );
        env.lock_mutex(mutex_id).unwrap();
        env.objc.sync_mutexes.insert(obj, mutex_id);
    }
    0 // OK
}

/// Backing function of @synchronized block exit.
/// This function is entirely undocumented, with
/// [source code provided](https://opensource.apple.com/source/objc4/objc4-551.1/runtime/objc-sync.h.auto.html).
pub(super) fn objc_sync_exit(env: &mut Environment, obj: id) -> i32 {
    match env.objc.sync_mutexes.get(&obj).cloned() {
        Some(mutex_id) => {
            match env.unlock_mutex(mutex_id) {
                Ok(lock_count) => {
                    if lock_count == 0 {
                        // Try to destroy mutex:
                        if env.mutex_state.destroy_mutex(mutex_id).is_ok() {
                            // If the mutex wasn't destroyed (Err), it means
                            // there's another mutex still locked, so we can't
                            // destroy the id->mutex mapping yet.
                            log_dbg!(
                                "Regular @synchronized block exit for {:#x} using mutex #{}, unlocked (destroying)",
                                obj.to_bits(),
                                mutex_id
                            );
                            log_dbg!("Destroyed mutex #{}", mutex_id);
                            env.objc.sync_mutexes.remove(&obj);
                        } else {
                            log_dbg!(
                                "Regular @synchronized block exit for {:#x} using mutex #{}, unlocked (not destroyed)",
                                obj.to_bits(),
                                mutex_id
                            );
                        }
                    } else {
                        log_dbg!(
                            "Regular @synchronized block exit for {:#x} using mutex #{}: {} locks remain",
                            obj.to_bits(),
                            mutex_id,
                            lock_count
                        );
                    }
                }
                Err(_) => {
                    panic!("Attempt to exit from @synchronized block for object {:#x} that was entered on a different thread!",
                           obj.to_bits()
                    );
                    // See below.
                }
            }
        }
        None => {
            panic!(
                "Attempt to exit from @synchronized block for object {:#x} that was not entered properly",
                obj.to_bits()
            );
            // Should technically return an error (non-zero), although I don't
            // think it's ever checked? Something probably went wrong to get
            // here.
        }
    }

    0 // OK
}
