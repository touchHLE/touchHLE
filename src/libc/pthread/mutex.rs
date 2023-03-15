/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Mutexes.

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::{EDEADLK, EPERM};
use crate::mem::{ConstPtr, MutPtr, Ptr, SafeRead};
use crate::{Environment, ThreadID};
use std::collections::HashMap;
use std::num::NonZeroU32;

#[derive(Default)]
pub struct State {
    mutexes: HashMap<MutPtr<pthread_mutex_t>, MutexHostObject>,
}
impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.libc_state.pthread.mutex
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
/// region. We will store the actual data on the host instead.
#[repr(C, packed)]
struct pthread_mutex_t {
    /// Magic number (must be [MAGIC_MUTEX])
    magic: u32,
}
unsafe impl SafeRead for pthread_mutex_t {}

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
const PTHREAD_MUTEX_NORMAL: MutexType = 0;
const PTHREAD_MUTEX_ERRORCHECK: MutexType = 1;
const PTHREAD_MUTEX_RECURSIVE: MutexType = 2;
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
    env.mem.write(mutex, pthread_mutex_t { magic: MAGIC_MUTEX });

    assert!(!State::get(env).mutexes.contains_key(&mutex));
    State::get(env).mutexes.insert(
        mutex,
        MutexHostObject {
            type_,
            locked: None,
        },
    );

    0 // success
}

fn check_or_register_mutex(env: &mut Environment, mutex: MutPtr<pthread_mutex_t>) {
    let magic: u32 = env.mem.read(mutex.cast());
    // This is a statically-initialized mutex, we need to register it, and
    // change the magic number in the process.
    if magic == MAGIC_MUTEX_STATIC {
        logg_dbg!(
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
    check_or_register_mutex(env, mutex);

    let current_thread = env.current_thread;
    let host_object: &mut _ = State::get(env).mutexes.get_mut(&mutex).unwrap();

    let Some((locking_thread, lock_count)) = host_object.locked else {
        logg_dbg!("Locked mutex {:?} for thread {}.", mutex, current_thread);
        host_object.locked = Some((current_thread, NonZeroU32::new(1).unwrap()));
        return 0; // success
    };

    if locking_thread == current_thread {
        match host_object.type_ {
            PTHREAD_MUTEX_NORMAL => {
                // This case would be a deadlock, we may as well panic.
                panic!(
                    "Attempted to lock non-error-checking mutex {:?} for thread {}, already locked by same thread!",
                    mutex, current_thread,
                );
            }
            PTHREAD_MUTEX_ERRORCHECK => {
                logg_dbg!("Attempted to lock error-checking mutex {:?} for thread {}, already locked by same thread! Returning EDEADLK.", mutex, current_thread);
                return EDEADLK;
            }
            PTHREAD_MUTEX_RECURSIVE => {
                logg_dbg!(
                    "Increasing lock level on recursive mutex {:?}, currently locked by thread {}.",
                    mutex,
                    locking_thread,
                );
                host_object.locked = Some((locking_thread, lock_count.checked_add(1).unwrap()));
                return 0; // success
            }
            _ => unreachable!(),
        }
    }

    // TODO: wait for unlock on other thread
    unimplemented!(
        "Attempted to lock mutex {:?} for thread {}, already locked by thread {}",
        mutex,
        current_thread,
        locking_thread,
    )
}

fn pthread_mutex_unlock(env: &mut Environment, mutex: MutPtr<pthread_mutex_t>) -> i32 {
    check_or_register_mutex(env, mutex);

    let current_thread = env.current_thread;
    let host_object: &mut _ = State::get(env).mutexes.get_mut(&mutex).unwrap();

    let Some((locking_thread, lock_count)) = host_object.locked else {
        match host_object.type_ {
            PTHREAD_MUTEX_NORMAL => {
                // This case is undefined, we may as well panic.
                panic!(
                    "Attempted to unlock non-error-checking mutex {:?} for thread {}, already unlocked!",
                    mutex, current_thread,
                );
            },
            PTHREAD_MUTEX_ERRORCHECK | PTHREAD_MUTEX_RECURSIVE => {
                logg_dbg!(
                    "Attempted to unlock error-checking or recursive mutex {:?} for thread {}, already unlocked! Returning EPERM.",
                    mutex, current_thread,
                );
                return EPERM;
            },
            _ => unreachable!(),
        }
    };

    if locking_thread != current_thread {
        match host_object.type_ {
            PTHREAD_MUTEX_NORMAL => {
                // This case is undefined, we may as well panic.
                panic!(
                    "Attempted to unlock non-error-checking matrix {:?} for thread {}, locked by different thread {}!",
                    mutex, current_thread, locking_thread,
                );
            }
            PTHREAD_MUTEX_ERRORCHECK | PTHREAD_MUTEX_RECURSIVE => {
                logg_dbg!(
                    "Attempted to unlock error-checking or recursive mutex {:?} for thread {}, lobkced by different thread {}! Returning EPERM.",
                    mutex, current_thread, locking_thread,
                );
                return EPERM;
            }
            _ => unreachable!(),
        }
    }

    if lock_count.get() == 1 {
        logg_dbg!("Unlocked mutex {:?} for thread {}.", mutex, current_thread);
        host_object.locked = None;
    } else {
        assert!(host_object.type_ == PTHREAD_MUTEX_RECURSIVE);
        logg_dbg!(
            "Decreasing lock level on recursive mutex {:?}, currently locked by thread {}.",
            mutex,
            locking_thread
        );
        host_object.locked = Some((
            locking_thread,
            NonZeroU32::new(lock_count.get() - 1).unwrap(),
        ));
    }
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(pthread_mutexattr_init(_)),
    export_c_func!(pthread_mutexattr_settype(_, _)),
    export_c_func!(pthread_mutexattr_destroy(_)),
    export_c_func!(pthread_mutex_init(_, _)),
    export_c_func!(pthread_mutex_lock(_)),
    export_c_func!(pthread_mutex_unlock(_)),
];
