/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Threads.

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::{EDEADLK, EINVAL};
use crate::libc::mach_host::PAGE_SIZE;
use crate::mem::{self, ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, SafeRead};
use crate::{Environment, ThreadId};
use std::collections::HashMap;

#[derive(Default)]
pub struct State {
    threads: HashMap<pthread_t, ThreadHostObject>,
    main_thread_object_created: bool,
}
impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.libc_state.pthread.thread
    }
}

/// Apple's implementation is a 4-byte magic number followed by an 36-byte
/// opaque region. We only have to match the size theirs has.
#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct pthread_attr_t {
    /// Magic number (must be [MAGIC_ATTR])
    magic: u32,
    detachstate: i32,
    stacksize: GuestUSize,
    _unused: [u32; 7],
}
unsafe impl SafeRead for pthread_attr_t {}

const DEFAULT_ATTR: pthread_attr_t = pthread_attr_t {
    magic: MAGIC_ATTR,
    detachstate: PTHREAD_CREATE_JOINABLE,
    stacksize: mem::Mem::SECONDARY_THREAD_DEFAULT_STACK_SIZE,
    _unused: [0; 7],
};

/// Apple's implementation is a 4-byte magic number followed by a massive
/// (>4KiB) opaque region. We will store the actual data on the host instead.
#[repr(C, packed)]
pub struct OpaqueThread {
    /// Magic number (must be [MAGIC_THREAD])
    magic: u32,
}
unsafe impl SafeRead for OpaqueThread {}

#[allow(non_camel_case_types)]
pub type pthread_t = MutPtr<OpaqueThread>;

struct ThreadHostObject {
    thread_id: ThreadId,
    joined_by: Option<ThreadId>,
    _attr: pthread_attr_t,
}

/// Arbitrarily-chosen magic number for `pthread_attr_t` (not Apple's).
const MAGIC_ATTR: u32 = u32::from_be_bytes(*b"ThAt");
/// Arbitrarily-chosen magic number for `pthread_t` (not Apple's).
const MAGIC_THREAD: u32 = u32::from_be_bytes(*b"THRD");

/// Custom typedef for readability (the C API just uses `int`)
type DetachState = i32;
const PTHREAD_CREATE_JOINABLE: DetachState = 1;
pub const PTHREAD_CREATE_DETACHED: DetachState = 2;

/// Value taken from an iOS 2.0 simulator
const PTHREAD_STACK_MIN: GuestUSize = 2 * PAGE_SIZE;

pub fn pthread_attr_init(env: &mut Environment, attr: MutPtr<pthread_attr_t>) -> i32 {
    env.mem.write(attr, DEFAULT_ATTR);
    0 // success
}
pub fn pthread_attr_setdetachstate(
    env: &mut Environment,
    attr: MutPtr<pthread_attr_t>,
    detachstate: DetachState,
) -> i32 {
    check_magic!(env, attr, MAGIC_ATTR);
    assert!(detachstate == PTHREAD_CREATE_JOINABLE || detachstate == PTHREAD_CREATE_DETACHED); // should be EINVAL
    let mut attr_copy = env.mem.read(attr);
    attr_copy.detachstate = detachstate;
    env.mem.write(attr, attr_copy);
    0 // success
}
pub fn pthread_attr_setstacksize(
    env: &mut Environment,
    attr: MutPtr<pthread_attr_t>,
    stacksize: GuestUSize,
) -> i32 {
    if attr.is_null() || stacksize < PTHREAD_STACK_MIN || stacksize % PAGE_SIZE != 0 {
        return EINVAL;
    }
    check_magic!(env, attr, MAGIC_ATTR);
    let mut attr_copy = env.mem.read(attr);
    attr_copy.stacksize = stacksize;
    env.mem.write(attr, attr_copy);
    0 // success
}
fn pthread_attr_destroy(env: &mut Environment, attr: MutPtr<pthread_attr_t>) -> i32 {
    check_magic!(env, attr, MAGIC_ATTR);
    env.mem.write(
        attr,
        pthread_attr_t {
            magic: 0,
            detachstate: 0,
            stacksize: 0,
            _unused: Default::default(),
        },
    );
    0 // success
}

pub fn pthread_create(
    env: &mut Environment,
    thread: MutPtr<pthread_t>,
    attr: ConstPtr<pthread_attr_t>,
    start_routine: GuestFunction, // (*void)(void *)
    user_data: MutVoidPtr,
) -> i32 {
    let attr = if !attr.is_null() {
        check_magic!(env, attr, MAGIC_ATTR);
        env.mem.read(attr)
    } else {
        DEFAULT_ATTR
    };

    let thread_id = env.new_thread(start_routine, user_data, attr.stacksize);

    let opaque = env.mem.alloc_and_write(OpaqueThread {
        magic: MAGIC_THREAD,
    });
    env.mem.write(thread, opaque);

    assert!(!State::get(env).threads.contains_key(&opaque));
    State::get(env).threads.insert(
        opaque,
        ThreadHostObject {
            thread_id,
            joined_by: None,
            _attr: attr,
        },
    );

    log_dbg!("pthread_create({:?}, {:?}, {:?}, {:?}) => 0 (success), created new pthread_t {:?} (thread ID: {})", thread, attr, start_routine, user_data, opaque, thread_id);

    0 // success
}

fn pthread_self(env: &mut Environment) -> pthread_t {
    let current_thread = env.current_thread;

    // The main thread is a special case since it's not created via pthreads,
    // so we need to create its object on-demand.
    if current_thread == 0 && !State::get(env).main_thread_object_created {
        State::get(env).main_thread_object_created = true;

        let opaque = env.mem.alloc_and_write(OpaqueThread {
            magic: MAGIC_THREAD,
        });

        assert!(!State::get(env).threads.contains_key(&opaque));
        State::get(env).threads.insert(
            opaque,
            ThreadHostObject {
                thread_id: 0,
                joined_by: None,
                _attr: DEFAULT_ATTR,
            },
        );
        log_dbg!(
            "pthread_self: created pthread object {:?} for main thread",
            opaque
        );
    }

    let (&ptr, _) = State::get(env)
        .threads
        .iter()
        .find(|&(_ptr, host_obj)| host_obj.thread_id == current_thread)
        .unwrap();
    ptr
}

fn pthread_join(env: &mut Environment, thread: pthread_t, retval: MutPtr<MutVoidPtr>) -> i32 {
    let current_thread = env.current_thread;
    let curr_pthread_t = pthread_self(env);
    // The joinee is the thread that is being waited on.
    let joinee_thread = State::get(env).threads.get_mut(&thread).unwrap().thread_id;

    // FIXME?: Blocking on the main thread is technically allowed, but
    // effectively useless (as the main thread exiting means the whole
    // application exits). It complicates some handling and is probably safe to
    // ignore here.
    assert!(joinee_thread != 0);

    // Can't join thread with itself!
    if joinee_thread == current_thread {
        log_dbg!("Thread attempted join with self, returning EDEADLK!");
        return EDEADLK;
    }

    // Check that the current thread is not being waited on by the joinee, to
    // prevent deadlocks.
    // This only prevents 2-long cycles (matching aspen simulator), which is
    // to say:
    //       joining                joining        joining
    // [T1] --------> [T2]    [T1] --------> [T2] --------> [T3]
    //   ^              |       ^                             |
    //   |    joining   |       |           joining           |
    //   '--------------'       '-----------------------------'
    //  This is prevented,               but this is not.
    let host_obj_curr = State::get(env).threads.get(&curr_pthread_t).unwrap();
    if let Some(thread) = host_obj_curr.joined_by {
        if thread == joinee_thread {
            log_dbg!("Thread attempted deadlocking join, returning EDEADLK!");
            return EDEADLK;
        }
    }

    // Deattached threads cannot be joined with.
    let host_obj_joinee = State::get(env).threads.get_mut(&thread).unwrap();
    if host_obj_joinee._attr.detachstate == PTHREAD_CREATE_DETACHED {
        log_dbg!("Thread attempted join with deattached thread, returning EINVAL!");
        return EINVAL;
    }

    host_obj_joinee.joined_by = Some(current_thread);
    // The executor will write the return value (void*) to *retval after the
    // join occurs.
    env.join_with_thread(joinee_thread, retval);
    0
}

fn pthread_setcanceltype(_env: &mut Environment, type_: i32, oldtype: MutPtr<i32>) -> i32 {
    log!("TODO: pthread_setcanceltype({}, {:?})", type_, oldtype);
    0
}
fn pthread_testcancel(_env: &mut Environment) {
    log!("TODO: pthread_testcancel()");
}

#[allow(non_camel_case_types)]
type mach_port_t = u32;

/// Undocumented Darwin function that returns a `mach_port_t`, which in practice
/// is used by apps as a unique thread ID.
fn pthread_mach_thread_np(env: &mut Environment, thread: pthread_t) -> mach_port_t {
    let host_object = State::get(env).threads.get(&thread).unwrap();
    host_object.thread_id.try_into().unwrap()
}

fn pthread_getschedparam(
    _env: &mut Environment,
    thread: pthread_t,
    policy: i32,
    param: MutVoidPtr,
) -> i32 {
    log_dbg!(
        "TODO: pthread_getschedparam({:?}, {}, {:?})",
        thread,
        policy,
        param
    );
    0
}

fn pthread_setschedparam(
    _env: &mut Environment,
    thread: pthread_t,
    policy: i32,
    param: ConstVoidPtr,
) -> i32 {
    log_dbg!(
        "TODO: pthread_setschedparam({:?}, {}, {:?})",
        thread,
        policy,
        param
    );
    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(pthread_attr_init(_)),
    export_c_func!(pthread_attr_setdetachstate(_, _)),
    export_c_func!(pthread_attr_setstacksize(_, _)),
    export_c_func!(pthread_attr_destroy(_)),
    export_c_func!(pthread_create(_, _, _, _)),
    export_c_func!(pthread_self()),
    export_c_func!(pthread_join(_, _)),
    export_c_func!(pthread_setcanceltype(_, _)),
    export_c_func!(pthread_testcancel()),
    export_c_func!(pthread_mach_thread_np(_)),
    export_c_func!(pthread_getschedparam(_, _, _)),
    export_c_func!(pthread_setschedparam(_, _, _)),
];
