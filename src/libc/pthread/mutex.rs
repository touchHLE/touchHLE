/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Mutexes.

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::{EBUSY, EDEADLK, EPERM};
use crate::mem::{ConstPtr, MutPtr, Ptr, SafeRead};
use crate::{Environment, ThreadID};
use std::collections::HashMap;
use std::num::NonZeroU32;

#[derive(Default)]
pub struct State {
    // TODO?: Maybe this should be a Vec instead? It would be bad if there were many mutexes over
    // the lifetime of an application, but it would perform better.
    // Maybe it could also be a fixed size allocator? (although that seems a little overkill)
    mutexes: HashMap<HostMutexId, MutexHostObject>,
    // Hopefully there will never be more than 2^64 mutexes in an applications lifetime :P
    mutex_count: u64,
}
impl State {
    fn get_mut(env: &mut Environment) -> &mut Self {
        &mut env.libc_state.pthread.mutex
    }
    fn get(env: &Environment) -> &Self {
        &env.libc_state.pthread.mutex
    }
}

/// Apple's implementation is a 4-byte magic number followed by an 8-byte opaque
/// region. We only have to match the size theirs has.
#[repr(C, packed)]
struct pthread_mutexattr_t {
    /// Magic number (must be [MAGIC_MUTEXATTR])
    magic: u32,
    type_: MutexType,
    /// This should eventually be a bitfield with the other attributes.
    _unused: u32,
}
unsafe impl SafeRead for pthread_mutexattr_t {}

/// Apple's implementation is a 4-byte magic number followed by a 56-byte opaque
/// region. We will store the actual data on the host, determined by a mutex identifier.
#[repr(C, packed)]
struct pthread_mutex_t {
    /// Magic number (must be [MAGIC_MUTEX])
    magic: u32,
    /// Unique mutex identifier, used in matching the mutex to it's host object.
    mutex_id: HostMutexId,
}
unsafe impl SafeRead for pthread_mutex_t {}

/// Unique identifier for mutexes, used for mutexes held by host objects and guest pthread mutexes.
/// Used in host_mutex_* functions, which are mostly similar to pthread iterfaces.
pub type HostMutexId = u64;

struct MutexHostObject {
    type_: MutexType,
    /// The `NonZeroU32` is the number of locks on this thread (if it's a
    /// recursive mutex).
    locked: Option<(ThreadID, NonZeroU32)>,
}

/// Arbitrarily-chosen magic number for `pthread_mutexattr_t` (not Apple's).
const MAGIC_MUTEXATTR: u32 = u32::from_be_bytes(*b"MuAt");
/// Arbitrarily-chosen magic number for `pthread_mutex_t` (not Apple's).
const MAGIC_MUTEX: u32 = u32::from_be_bytes(*b"MUTX");
/// Magic number used by `PTHREAD_MUTEX_INITIALIZER`. This is part of the ABI!
const MAGIC_MUTEX_STATIC: u32 = 0x32AAABA7;

/// Custom typedef for readability (the C API just uses `int`)
type MutexType = i32;
pub const PTHREAD_MUTEX_NORMAL: MutexType = 0;
pub const PTHREAD_MUTEX_ERRORCHECK: MutexType = 1;
pub const PTHREAD_MUTEX_RECURSIVE: MutexType = 2;
const PTHREAD_MUTEX_DEFAULT: MutexType = PTHREAD_MUTEX_NORMAL;

fn pthread_mutexattr_init(env: &mut Environment, attr: MutPtr<pthread_mutexattr_t>) -> i32 {
    env.mem.write(
        attr,
        pthread_mutexattr_t {
            magic: MAGIC_MUTEXATTR,
            type_: PTHREAD_MUTEX_DEFAULT,
            _unused: 0,
        },
    );
    0 // success
}
fn pthread_mutexattr_settype(
    env: &mut Environment,
    attr: MutPtr<pthread_mutexattr_t>,
    type_: MutexType,
) -> i32 {
    check_magic!(env, attr, MAGIC_MUTEXATTR);
    assert!(
        type_ == PTHREAD_MUTEX_NORMAL
            || type_ == PTHREAD_MUTEX_ERRORCHECK
            || type_ == PTHREAD_MUTEX_RECURSIVE
    ); // should be EINVAL
    let mut attr_copy = env.mem.read(attr);
    attr_copy.type_ = type_;
    env.mem.write(attr, attr_copy);
    0 // success
}
fn pthread_mutexattr_destroy(env: &mut Environment, attr: MutPtr<pthread_mutexattr_t>) -> i32 {
    check_magic!(env, attr, MAGIC_MUTEXATTR);
    env.mem.write(
        attr,
        pthread_mutexattr_t {
            magic: 0,
            type_: 0,
            _unused: 0,
        },
    );
    0 // success
}

fn pthread_mutex_init(
    env: &mut Environment,
    mutex: MutPtr<pthread_mutex_t>,
    attr: ConstPtr<pthread_mutexattr_t>,
) -> i32 {
    let type_ = if !attr.is_null() {
        check_magic!(env, attr, MAGIC_MUTEXATTR);
        let pthread_mutexattr_t { type_, .. } = env.mem.read(attr);
        assert!(
            type_ == PTHREAD_MUTEX_NORMAL
                || type_ == PTHREAD_MUTEX_ERRORCHECK
                || type_ == PTHREAD_MUTEX_RECURSIVE
        );
        type_
    } else {
        PTHREAD_MUTEX_DEFAULT
    };
    let mutex_id = host_mutex_init(env, type_);
    log_dbg!(
        "Mutex #{} created from pthread_mutex_init ({:#x})",
        mutex_id,
        mutex.to_bits()
    );
    env.mem.write(
        mutex,
        pthread_mutex_t {
            magic: MAGIC_MUTEX,
            mutex_id,
        },
    );

    0 // success
}

fn check_or_register_mutex(env: &mut Environment, mutex: MutPtr<pthread_mutex_t>) {
    let magic: u32 = env.mem.read(mutex.cast());
    // This is a statically-initialized mutex, we need to register it, and
    // change the magic number in the process.
    if magic == MAGIC_MUTEX_STATIC {
        log_dbg!(
            "Detected statically-initialized mutex at {:?}, registering.",
            mutex
        );
        pthread_mutex_init(env, mutex, Ptr::null());
    } else {
        // We should actually return an error if the magic number doesn't match,
        // but this almost certainly indicates a memory corruption, so panicking
        // is more useful.
        assert_eq!(magic, MAGIC_MUTEX);
    }
}

fn pthread_mutex_lock(env: &mut Environment, mutex: MutPtr<pthread_mutex_t>) -> i32 {
    let mutex_data = env.mem.read(mutex);
    check_or_register_mutex(env, mutex);
    host_mutex_lock(env, mutex_data.mutex_id).err().unwrap_or(0)
}

fn pthread_mutex_unlock(env: &mut Environment, mutex: MutPtr<pthread_mutex_t>) -> i32 {
    let mutex_data = env.mem.read(mutex);
    check_or_register_mutex(env, mutex);
    host_mutex_unlock(env, mutex_data.mutex_id)
        .err()
        .unwrap_or(0)
}

fn pthread_mutex_destroy(env: &mut Environment, mutex: MutPtr<pthread_mutex_t>) -> i32 {
    check_or_register_mutex(env, mutex);
    let mutex_id = env.mem.read(mutex).mutex_id;
    env.mem.write(
        mutex,
        pthread_mutex_t {
            magic: 0,
            mutex_id: 0xFFFFFFFFFFFFFFFF,
        },
    );
    host_mutex_destroy(env, mutex_id).err().unwrap_or(0)
}

pub fn host_mutex_init(env: &mut Environment, mutex_type: MutexType) -> HostMutexId {
    let state = State::get_mut(env);
    let mutex_id = state.mutex_count;
    state.mutex_count = state.mutex_count.checked_add(1).unwrap();
    state.mutexes.insert(
        mutex_id,
        MutexHostObject {
            type_: mutex_type,
            locked: None,
        },
    );
    log_dbg!(
        "Created mutex #{}, type {:?}",
        state.mutex_count,
        mutex_type
    );
    mutex_id
}

pub fn host_mutex_lock(env: &mut Environment, mutex_id: HostMutexId) -> Result<u32, i32> {
    let current_thread = env.current_thread;
    let host_object: &mut _ = State::get_mut(env).mutexes.get_mut(&mutex_id).unwrap();

    let Some((locking_thread, lock_count)) = host_object.locked else {
        log_dbg!("Locked mutex #{} for thread {}.", mutex_id, current_thread);
        host_object.locked = Some((current_thread, NonZeroU32::new(1).unwrap()));
        return Ok(1);
    };

    if locking_thread == current_thread {
        match host_object.type_ {
            PTHREAD_MUTEX_NORMAL => {
                // This case would be a deadlock, we may as well panic.
                panic!(
                    "Attempted to lock non-error-checking mutex #{} for thread {}, already locked by same thread!",
                    mutex_id, current_thread,
                );
            }
            PTHREAD_MUTEX_ERRORCHECK => {
                log_dbg!("Attempted to lock error-checking mutex #{} for thread {}, already locked by same thread! Returning EDEADLK.", mutex_id, current_thread);
                return Err(EDEADLK);
            }
            PTHREAD_MUTEX_RECURSIVE => {
                log_dbg!(
                    "Increasing lock level on recursive mutex #{}, currently locked by thread {}.",
                    mutex_id,
                    locking_thread,
                );
                host_object.locked = Some((locking_thread, lock_count.checked_add(1).unwrap()));
                return Ok(lock_count.get() + 1);
            }
            _ => unreachable!(),
        }
    }

    // Mutex is already locked, block thread until it isn't.
    env.block_on_mutex(mutex_id);
    // Lock count is always 1 after a thread-blocking lock
    Ok(1)
}

pub fn host_mutex_unlock(env: &mut Environment, mutex_id: HostMutexId) -> Result<u32, i32> {
    let current_thread = env.current_thread;
    let host_object: &mut _ = State::get_mut(env).mutexes.get_mut(&mutex_id).unwrap();

    let Some((locking_thread, lock_count)) = host_object.locked else {
        match host_object.type_ {
            PTHREAD_MUTEX_NORMAL => {
                // This case is undefined, we may as well panic.
                panic!(
                    "Attempted to unlock non-error-checking mutex #{} for thread {}, already unlocked!",
                    mutex_id, current_thread,
                );
            },
            PTHREAD_MUTEX_ERRORCHECK | PTHREAD_MUTEX_RECURSIVE => {
                log_dbg!(
                    "Attempted to unlock error-checking or recursive mutex #{} for thread {}, already unlocked! Returning EPERM.",
                    mutex_id, current_thread,
                );
                return Err(EPERM);
            },
            _ => unreachable!(),
        }
    };

    if locking_thread != current_thread {
        match host_object.type_ {
            PTHREAD_MUTEX_NORMAL => {
                // This case is undefined, we may as well panic.
                panic!(
                    "Attempted to unlock non-error-checking mutex #{} for thread {}, locked by different thread {}!",
                    mutex_id, current_thread, locking_thread,
                );
            }
            PTHREAD_MUTEX_ERRORCHECK | PTHREAD_MUTEX_RECURSIVE => {
                log_dbg!(
                    "Attempted to unlock error-checking or recursive mutex #{} for thread {}, locked by different thread {}! Returning EPERM.",
                    mutex_id, current_thread, locking_thread,
                );
                return Err(EPERM);
            }
            _ => unreachable!(),
        }
    }

    if lock_count.get() == 1 {
        log_dbg!(
            "Unlocked mutex #{} for thread {}.",
            mutex_id,
            current_thread
        );
        host_object.locked = None;
        Ok(0)
    } else {
        assert!(host_object.type_ == PTHREAD_MUTEX_RECURSIVE);
        log_dbg!(
            "Decreasing lock level on recursive mutex #{}, currently locked by thread {}.",
            mutex_id,
            locking_thread
        );
        host_object.locked = Some((
            locking_thread,
            NonZeroU32::new(lock_count.get() - 1).unwrap(),
        ));
        Ok(lock_count.get() - 1)
    }
}

pub fn host_mutex_destroy(env: &mut Environment, mutex_id: HostMutexId) -> Result<(), i32> {
    let state = State::get_mut(env);
    let host_object = state.mutexes.get_mut(&mutex_id).unwrap();
    if host_object.locked.is_some() {
        log_dbg!("Attempted to destroy locked mutex, returning EBUSY!");
        return Err(EBUSY);
    }
    state.mutexes.remove(&mutex_id);
    // If the mutex used the current highest id, it can be reclaimed.
    if mutex_id + 1 == state.mutex_count {
        state.mutex_count = state.mutex_count.checked_sub(1).unwrap();
    }
    Ok(())
}

pub fn mutex_is_locked(env: &Environment, mutex_id: HostMutexId) -> bool {
    let state = State::get(env);
    state
        .mutexes
        .get(&mutex_id)
        .map_or(false, |host_obj| host_obj.locked.is_some())
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(pthread_mutexattr_init(_)),
    export_c_func!(pthread_mutexattr_settype(_, _)),
    export_c_func!(pthread_mutexattr_destroy(_)),
    export_c_func!(pthread_mutex_init(_, _)),
    export_c_func!(pthread_mutex_lock(_)),
    export_c_func!(pthread_mutex_unlock(_)),
    export_c_func!(pthread_mutex_destroy(_)),
];
