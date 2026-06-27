/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Read–write locks (`pthread_rwlock_t`).
//!
//! Apple references:
//! - <https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/pthread_rwlock_init.3.html>
//! - <https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/pthread_rwlock_rdlock.3.html>
//! - <https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/pthread_rwlock_wrlock.3.html>
//! - <https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/pthread_rwlock_unlock.3.html>
//! - <https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/pthread_rwlock_destroy.3.html>
//!
//! POSIX permits a conforming implementation that does not allow multiple
//! concurrent readers, provided every reader/writer is serialised through a
//! single critical section (POSIX.1‑2008 §4.12). touchHLE uses that
//! conservative model: each rwlock is backed by a normal mutex from the
//! shared [`crate::environment::mutex::MutexState`], so every
//! `pthread_rwlock_rdlock` / `pthread_rwlock_wrlock` acquires the mutex and
//! `pthread_rwlock_unlock` releases it. This loses reader-parallelism but
//! preserves correctness — and it integrates with the existing thread
//! scheduler so blocked callers actually yield.

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::{EBUSY, EINVAL};
use crate::mem::{ConstPtr, MutPtr, SafeRead};
use crate::{Environment, MutexId, PTHREAD_MUTEX_DEFAULT};

/// `pthread_rwlockattr_t` — Apple's real implementation is a 4-byte magic
/// plus a 16-byte opaque region. We only need to model the size and to
/// recognise our own magic number.
#[repr(C, packed)]
pub struct pthread_rwlockattr_t {
    magic: u32,
    _opaque: [u32; 4],
}
unsafe impl SafeRead for pthread_rwlockattr_t {}

/// `pthread_rwlock_t` — Apple's real implementation is a 4-byte magic plus
/// a 192-byte opaque region. We store the real data on the host, keyed by
/// the mutex id stashed in the guest-visible header.
#[repr(C, packed)]
pub struct pthread_rwlock_t {
    magic: u32,
    rwlock_id: MutexId,
}
unsafe impl SafeRead for pthread_rwlock_t {}

const MAGIC_RWLOCKATTR: u32 = u32::from_be_bytes(*b"RwAt");
const MAGIC_RWLOCK: u32 = u32::from_be_bytes(*b"RWLK");
/// Magic word used by Apple's `PTHREAD_RWLOCK_INITIALIZER`. This is the
/// observed bit-pattern from `pthread/_pthread_types.h` (`_PTHREAD_RWLOCK_SIG_init`).
const MAGIC_RWLOCK_STATIC: u32 = 0x2DA8B3B4;

fn pthread_rwlockattr_init(env: &mut Environment, attr: MutPtr<pthread_rwlockattr_t>) -> i32 {
    env.mem.write(
        attr,
        pthread_rwlockattr_t {
            magic: MAGIC_RWLOCKATTR,
            _opaque: [0; 4],
        },
    );
    0
}

fn pthread_rwlockattr_destroy(env: &mut Environment, attr: MutPtr<pthread_rwlockattr_t>) -> i32 {
    check_magic!(env, attr, MAGIC_RWLOCKATTR);
    env.mem.write(
        attr,
        pthread_rwlockattr_t {
            magic: 0,
            _opaque: [0; 4],
        },
    );
    0
}

fn pthread_rwlockattr_setpshared(
    env: &mut Environment,
    attr: MutPtr<pthread_rwlockattr_t>,
    _pshared: i32,
) -> i32 {
    check_magic!(env, attr, MAGIC_RWLOCKATTR);
    0
}

fn pthread_rwlockattr_getpshared(
    env: &mut Environment,
    attr: ConstPtr<pthread_rwlockattr_t>,
    pshared: MutPtr<i32>,
) -> i32 {
    check_magic!(env, attr, MAGIC_RWLOCKATTR);
    if pshared.is_null() {
        return EINVAL;
    }
    // PTHREAD_PROCESS_PRIVATE is the only mode we support.
    env.mem.write(pshared, 2);
    0
}

pub fn pthread_rwlock_init(
    env: &mut Environment,
    rwlock: MutPtr<pthread_rwlock_t>,
    attr: ConstPtr<pthread_rwlockattr_t>,
) -> i32 {
    if !attr.is_null() {
        check_magic!(env, attr, MAGIC_RWLOCKATTR);
    }
    let rwlock_id = env.mutex_state.init_mutex(PTHREAD_MUTEX_DEFAULT);
    env.mem.write(
        rwlock,
        pthread_rwlock_t {
            magic: MAGIC_RWLOCK,
            rwlock_id,
        },
    );
    log_dbg!(
        "pthread_rwlock_init: rwlock at {:?} mapped to mutex #{}",
        rwlock,
        rwlock_id
    );
    0
}

fn check_or_register_rwlock(
    env: &mut Environment,
    rwlock: MutPtr<pthread_rwlock_t>,
) -> Result<(), i32> {
    let magic: u32 = env.mem.read(rwlock.cast());
    if magic == MAGIC_RWLOCK_STATIC {
        log_dbg!(
            "Detected statically-initialised rwlock at {:?}, registering.",
            rwlock
        );
        pthread_rwlock_init(env, rwlock, ConstPtr::null());
        Ok(())
    } else if magic == MAGIC_RWLOCK {
        Ok(())
    } else {
        Err(EINVAL)
    }
}

fn lock_inner(env: &mut Environment, rwlock: MutPtr<pthread_rwlock_t>) -> i32 {
    if let Err(e) = check_or_register_rwlock(env, rwlock) {
        return e;
    }
    let id = env.mem.read(rwlock).rwlock_id;
    env.lock_mutex(id).err().unwrap_or(0)
}

fn trylock_inner(env: &mut Environment, rwlock: MutPtr<pthread_rwlock_t>) -> i32 {
    if let Err(e) = check_or_register_rwlock(env, rwlock) {
        return e;
    }
    let id = env.mem.read(rwlock).rwlock_id;
    if env.mutex_state.mutex_is_locked(id) {
        EBUSY
    } else {
        env.lock_mutex(id).err().unwrap_or(0)
    }
}

pub fn pthread_rwlock_rdlock(env: &mut Environment, rwlock: MutPtr<pthread_rwlock_t>) -> i32 {
    lock_inner(env, rwlock)
}

pub fn pthread_rwlock_tryrdlock(env: &mut Environment, rwlock: MutPtr<pthread_rwlock_t>) -> i32 {
    trylock_inner(env, rwlock)
}

pub fn pthread_rwlock_wrlock(env: &mut Environment, rwlock: MutPtr<pthread_rwlock_t>) -> i32 {
    lock_inner(env, rwlock)
}

pub fn pthread_rwlock_trywrlock(env: &mut Environment, rwlock: MutPtr<pthread_rwlock_t>) -> i32 {
    trylock_inner(env, rwlock)
}

pub fn pthread_rwlock_unlock(env: &mut Environment, rwlock: MutPtr<pthread_rwlock_t>) -> i32 {
    if let Err(e) = check_or_register_rwlock(env, rwlock) {
        return e;
    }
    let id = env.mem.read(rwlock).rwlock_id;
    env.unlock_mutex(id).err().unwrap_or(0)
}

pub fn pthread_rwlock_destroy(env: &mut Environment, rwlock: MutPtr<pthread_rwlock_t>) -> i32 {
    if let Err(e) = check_or_register_rwlock(env, rwlock) {
        return e;
    }
    let id = env.mem.read(rwlock).rwlock_id;
    env.mem.write(
        rwlock,
        pthread_rwlock_t {
            magic: 0,
            rwlock_id: 0xFFFFFFFFFFFFFFFF,
        },
    );
    env.mutex_state.destroy_mutex(id).err().unwrap_or(0)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(pthread_rwlockattr_init(_)),
    export_c_func!(pthread_rwlockattr_destroy(_)),
    export_c_func!(pthread_rwlockattr_setpshared(_, _)),
    export_c_func!(pthread_rwlockattr_getpshared(_, _)),
    export_c_func!(pthread_rwlock_init(_, _)),
    export_c_func!(pthread_rwlock_rdlock(_)),
    export_c_func!(pthread_rwlock_tryrdlock(_)),
    export_c_func!(pthread_rwlock_wrlock(_)),
    export_c_func!(pthread_rwlock_trywrlock(_)),
    export_c_func!(pthread_rwlock_unlock(_)),
    export_c_func!(pthread_rwlock_destroy(_)),
];
