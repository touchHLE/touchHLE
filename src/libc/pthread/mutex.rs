/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Guest mutex interface.
//!
//! See [crate::environment::mutex] for the internal implementation.

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, MutPtr, Ptr, SafeRead};
use crate::{Environment, MutexId, PTHREAD_MUTEX_DEFAULT};

/// Apple's implementation is a 4-byte magic number followed by an 8-byte opaque
/// region. We only have to match the size theirs has.
#[repr(C, packed)]
struct pthread_mutexattr_t {
    /// Magic number (must be [MAGIC_MUTEXATTR])
    magic: u32,
    type_: i32,
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
    mutex_id: MutexId,
}
unsafe impl SafeRead for pthread_mutex_t {}

/// Arbitrarily-chosen magic number for `pthread_mutexattr_t` (not Apple's).
const MAGIC_MUTEXATTR: u32 = u32::from_be_bytes(*b"MuAt");
/// Arbitrarily-chosen magic number for `pthread_mutex_t` (not Apple's).
const MAGIC_MUTEX: u32 = u32::from_be_bytes(*b"MUTX");
/// Magic number used by `PTHREAD_MUTEX_INITIALIZER`. This is part of the ABI!
const MAGIC_MUTEX_STATIC: u32 = 0x32AAABA7;

fn pthread_mutexattr_init(env: &mut Environment, attr: MutPtr<pthread_mutexattr_t>) -> i32 {
    env.mem.write(
        attr,
        pthread_mutexattr_t {
            magic: MAGIC_MUTEXATTR,
            type_: PTHREAD_MUTEX_DEFAULT as i32,
            _unused: 0,
        },
    );
    0 // success
}
fn pthread_mutexattr_settype(
    env: &mut Environment,
    attr: MutPtr<pthread_mutexattr_t>,
    type_: i32,
) -> i32 {
    check_magic!(env, attr, MAGIC_MUTEXATTR);
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
            type_: PTHREAD_MUTEX_DEFAULT as i32,
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
        type_.try_into().unwrap()
    } else {
        PTHREAD_MUTEX_DEFAULT
    };
    let mutex_id = env.mutex_state.init_mutex(type_);
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
    env.lock_mutex(mutex_data.mutex_id).err().unwrap_or(0)
}

fn pthread_mutex_unlock(env: &mut Environment, mutex: MutPtr<pthread_mutex_t>) -> i32 {
    let mutex_data = env.mem.read(mutex);
    check_or_register_mutex(env, mutex);
    env.unlock_mutex(mutex_data.mutex_id).err().unwrap_or(0)
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
    env.mutex_state.destroy_mutex(mutex_id).err().unwrap_or(0)
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
