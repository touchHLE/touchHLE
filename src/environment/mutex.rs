/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Internal mutex interface.

use std::collections::HashMap;
use std::num::NonZeroU32;

use super::{Environment, ThreadId};
use crate::libc::errno::{EBUSY, EDEADLK, EPERM};

/// Stores and manages mutexes. Note that all the methods for locking and
/// unlocking mutexes are on [Environment] instead, because they interact with
/// threads.
#[derive(Default)]
pub struct MutexState {
    // TODO?: Maybe this should be a Vec instead? It would be bad if there were
    // many mutexes over the lifetime of an application, but it would perform
    // better. Maybe it could also be a fixed size allocator? (although that
    // seems a little overkill)
    mutexes: HashMap<MutexId, Mutex>,
    // Hopefully there will never be more than 2^64 mutexes in an application's
    // lifetime :P
    mutex_count: u64,
}

/// Unique identifier for mutexes, used for mutexes held by host objects and
/// guest pthread mutexes.
pub type MutexId = u64;

struct Mutex {
    type_: MutexType,
    waiting_count: u32,
    /// The `NonZeroU32` is the number of locks on this thread (if it's a
    /// recursive mutex).
    locked: Option<(ThreadId, NonZeroU32)>,
}

#[repr(i32)]
#[derive(Debug, PartialEq, Copy, Clone)]
#[allow(non_camel_case_types)]
pub enum MutexType {
    PTHREAD_MUTEX_NORMAL = 0,
    PTHREAD_MUTEX_ERRORCHECK = 1,
    PTHREAD_MUTEX_RECURSIVE = 2,
}

impl TryFrom<i32> for MutexType {
    type Error = &'static str;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MutexType::PTHREAD_MUTEX_NORMAL),
            1 => Ok(MutexType::PTHREAD_MUTEX_ERRORCHECK),
            2 => Ok(MutexType::PTHREAD_MUTEX_RECURSIVE),
            _ => Err("Value is not a valid mutex type!"),
        }
    }
}
pub const PTHREAD_MUTEX_DEFAULT: MutexType = MutexType::PTHREAD_MUTEX_NORMAL;

impl MutexState {
    /// Initializes a mutex and returns a handle to it. Similar to
    /// `pthread_mutex_init`, but for host code.
    pub fn init_mutex(&mut self, mutex_type: MutexType) -> MutexId {
        let mutex_id = self.mutex_count;
        self.mutex_count = self.mutex_count.checked_add(1).unwrap();
        self.mutexes.insert(
            mutex_id,
            Mutex {
                type_: mutex_type,
                waiting_count: 0,
                locked: None,
            },
        );
        log_dbg!("Created mutex #{}, type {:?}", mutex_id, mutex_type);
        mutex_id
    }

    /// Destroys a mutex and returns an error on failure (as errno). Similar to
    /// `pthread_mutex_destroy`, but for host code. Note that the mutex is not
    /// destroyed on an Err return.
    pub fn destroy_mutex(&mut self, mutex_id: MutexId) -> Result<(), i32> {
        let mutex = self.mutexes.get_mut(&mutex_id).unwrap();
        if mutex.locked.is_some() {
            log_dbg!("Attempted to destroy currently locked mutex, returning EBUSY!");
            return Err(EBUSY);
        } else if mutex.waiting_count != 0 {
            log_dbg!("Attempted to destroy mutex with waiting locks, returning EBUSY!");
            return Err(EBUSY);
        }
        // TODO?: If we switch to a vec-based system, we should reuse destroyed
        // ids if they are at the top of the stack.
        self.mutexes.remove(&mutex_id);
        Ok(())
    }

    pub fn mutex_is_locked(&self, mutex_id: MutexId) -> bool {
        self.mutexes
            .get(&mutex_id)
            .map_or(false, |mutex| mutex.locked.is_some())
    }
}

impl Environment {
    /// Relock mutex that was just unblocked. This should probably only be used
    /// by the thread scheduler.
    pub fn relock_unblocked_mutex(&mut self, mutex_id: MutexId) {
        log_dbg!(
            "Relocking unblocked mutex {}, waiting count {}",
            mutex_id,
            self.mutex_state
                .mutexes
                .get_mut(&mutex_id)
                .unwrap()
                .waiting_count
        );
        self.lock_mutex(mutex_id).unwrap();
        if self
            .mutex_state
            .mutexes
            .get_mut(&mutex_id)
            .unwrap()
            .waiting_count
            > 0
        {
            self.mutex_state
                .mutexes
                .get_mut(&mutex_id)
                .unwrap()
                .waiting_count -= 1;
        }
    }

    /// Locks a mutex and returns the lock count or an error (as errno). Similar
    /// to `pthread_mutex_lock`, but for host code.
    /// NOTE: This only takes effect _after_ the calling function returns to the
    /// host run loop ([crate::Environment::run]). As such, this should only be
    /// called right before a function returns (to the host run loop).
    pub fn lock_mutex(&mut self, mutex_id: MutexId) -> Result<u32, i32> {
        let current_thread = self.current_thread;
        let mutex: &mut _ = self.mutex_state.mutexes.get_mut(&mutex_id).unwrap();

        let Some((locking_thread, lock_count)) = mutex.locked else {
            log_dbg!("Locked mutex #{} for thread {}.", mutex_id, current_thread);
            mutex.locked = Some((current_thread, NonZeroU32::new(1).unwrap()));
            return Ok(1);
        };

        if locking_thread == current_thread {
            match mutex.type_ {
                MutexType::PTHREAD_MUTEX_NORMAL => {
                    // This case would be a deadlock, we may as well panic.
                    panic!(
                        "Attempted to lock non-error-checking mutex #{} for thread {}, already locked by same thread!",
                        mutex_id, current_thread,
                    );
                }
                MutexType::PTHREAD_MUTEX_ERRORCHECK => {
                    log_dbg!("Attempted to lock error-checking mutex #{} for thread {}, already locked by same thread! Returning EDEADLK.", mutex_id, current_thread);
                    return Err(EDEADLK);
                }
                MutexType::PTHREAD_MUTEX_RECURSIVE => {
                    log_dbg!(
                        "Increasing lock level on recursive mutex #{}, currently locked by thread {}.",
                        mutex_id,
                        locking_thread,
                    );
                    mutex.locked = Some((locking_thread, lock_count.checked_add(1).unwrap()));
                    return Ok(lock_count.get() + 1);
                }
            }
        }

        // Add to the waiting count, so that the mutex isn't destroyed. This is
        // subtracted in relock_unblocked_mutex.
        mutex.waiting_count += 1;

        // Mutex is already locked, block thread until it isn't.
        self.block_on_mutex(mutex_id);
        // Lock count is always 1 after a thread-blocking lock.
        Ok(1)
    }

    /// Unlocks a mutex and returns the lock count or an error (as errno).
    /// Similar to `pthread_mutex_unlock`, but for host code.
    pub fn unlock_mutex(&mut self, mutex_id: MutexId) -> Result<u32, i32> {
        let current_thread = self.current_thread;
        let mutex: &mut _ = self.mutex_state.mutexes.get_mut(&mutex_id).unwrap();

        let Some((locking_thread, lock_count)) = mutex.locked else {
            match mutex.type_ {
                MutexType::PTHREAD_MUTEX_NORMAL => {
                    // This case is undefined, we may as well panic.
                    panic!(
                        "Attempted to unlock non-error-checking mutex #{} for thread {}, already unlocked!",
                        mutex_id, current_thread,
                    );
                }
                MutexType::PTHREAD_MUTEX_ERRORCHECK | MutexType::PTHREAD_MUTEX_RECURSIVE => {
                    log_dbg!(
                        "Attempted to unlock error-checking or recursive mutex #{} for thread {}, already unlocked! Returning EPERM.",
                        mutex_id, current_thread,
                    );
                    return Err(EPERM);
                }
            }
        };

        if locking_thread != current_thread {
            match mutex.type_ {
                MutexType::PTHREAD_MUTEX_NORMAL => {
                    // This case is undefined, we may as well panic.
                    panic!(
                        "Attempted to unlock non-error-checking mutex #{} for thread {}, locked by different thread {}!",
                        mutex_id, current_thread, locking_thread,
                    );
                }
                MutexType::PTHREAD_MUTEX_ERRORCHECK | MutexType::PTHREAD_MUTEX_RECURSIVE => {
                    log_dbg!(
                        "Attempted to unlock error-checking or recursive mutex #{} for thread {}, locked by different thread {}! Returning EPERM.",
                        mutex_id, current_thread, locking_thread,
                    );
                    return Err(EPERM);
                }
            }
        }

        if lock_count.get() == 1 {
            log_dbg!(
                "Unlocked mutex #{} for thread {}.",
                mutex_id,
                current_thread
            );
            mutex.locked = None;
            Ok(0)
        } else {
            assert!(mutex.type_ == MutexType::PTHREAD_MUTEX_RECURSIVE);
            log_dbg!(
                "Decreasing lock level on recursive mutex #{}, currently locked by thread {}.",
                mutex_id,
                locking_thread
            );
            mutex.locked = Some((
                locking_thread,
                NonZeroU32::new(lock_count.get() - 1).unwrap(),
            ));
            Ok(lock_count.get() - 1)
        }
    }
}
