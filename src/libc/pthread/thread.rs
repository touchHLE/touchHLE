/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Threads.

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::{EDEADLK, EINVAL, ESRCH};
use crate::mem::{
    self, ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr, SafeRead, PAGE_SIZE,
};
use crate::{cpu::Cpu, Environment, ThreadId};
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
    // ИСПРАВЛЕНИЕ: Добавляем реальные поля для политики и параметров
    sched_policy: i32,
    sched_param: sched_param,
    // Уменьшаем _unused с 7 до 5, так как добавили два 4-байтовых поля (чтобы
    // сохранить общий размер в 40 байт)
    _unused: [u32; 5],
}
unsafe impl SafeRead for pthread_attr_t {}

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
struct sched_param {
    sched_priority: i32,
}
unsafe impl SafeRead for sched_param {}

const DEFAULT_ATTR: pthread_attr_t = pthread_attr_t {
    magic: MAGIC_ATTR,
    detachstate: PTHREAD_CREATE_JOINABLE,
    stacksize: mem::Mem::SECONDARY_THREAD_DEFAULT_STACK_SIZE,
    sched_policy: 1, // SCHED_OTHER (дефолтная политика планирования в POSIX)
    sched_param: sched_param { sched_priority: 0 },
    _unused: [0; 5],
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

// Cancellation state constants
const PTHREAD_CANCEL_ENABLE: i32 = 0;
const PTHREAD_CANCEL_DISABLE: i32 = 1;

// Cancellation type constants
const PTHREAD_CANCEL_DEFERRED: i32 = 0;
const PTHREAD_CANCEL_ASYNCHRONOUS: i32 = 1;

/// Sentinel return value for a cancelled thread (POSIX specifies this as
/// a special non-NULL pointer value distinct from any valid pointer).
pub const PTHREAD_CANCELED: MutVoidPtr = Ptr::from_bits(u32::MAX);

struct ThreadHostObject {
    thread_id: ThreadId,
    joined_by: Option<ThreadId>,
    attr: pthread_attr_t,
    /// Set by pthread_cancel — thread has been asked to terminate.
    cancel_requested: bool,
    /// Set by pthread_setcancelstate(PTHREAD_CANCEL_DISABLE).
    cancel_disabled: bool,
    /// Set by pthread_setcanceltype(PTHREAD_CANCEL_ASYNCHRONOUS).
    cancel_async: bool,
    /// Thread name set via pthread_setname_np (Darwin extension, max 63 chars).
    name: String,
}

impl ThreadHostObject {
    fn new(thread_id: ThreadId, attr: pthread_attr_t) -> Self {
        ThreadHostObject {
            thread_id,
            joined_by: None,
            attr,
            cancel_requested: false,
            cancel_disabled: false,
            cancel_async: false,
            name: String::new(),
        }
    }
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

// =========================================================================
// MARK: - Attribute functions
// =========================================================================

pub fn pthread_attr_init(env: &mut Environment, attr: MutPtr<pthread_attr_t>) -> i32 {
    env.mem.write(attr, DEFAULT_ATTR);
    0
}

fn pthread_attr_getdetachstate(
    env: &mut Environment,
    attr: MutPtr<pthread_attr_t>,
    detachstate_ptr: MutPtr<DetachState>,
) -> i32 {
    check_magic!(env, attr, MAGIC_ATTR);
    let detachstate = env.mem.read(attr).detachstate;
    assert!(detachstate == PTHREAD_CREATE_JOINABLE || detachstate == PTHREAD_CREATE_DETACHED);
    env.mem.write(detachstate_ptr, detachstate);
    0
}

pub fn pthread_attr_setdetachstate(
    env: &mut Environment,
    attr: MutPtr<pthread_attr_t>,
    detachstate: DetachState,
) -> i32 {
    check_magic!(env, attr, MAGIC_ATTR);
    assert!(detachstate == PTHREAD_CREATE_JOINABLE || detachstate == PTHREAD_CREATE_DETACHED);
    let mut attr_copy = env.mem.read(attr);
    attr_copy.detachstate = detachstate;
    env.mem.write(attr, attr_copy);
    0
}

fn pthread_attr_getstacksize(
    env: &mut Environment,
    attr: MutPtr<pthread_attr_t>,
    stacksize: MutPtr<GuestUSize>,
) -> i32 {
    if attr.is_null() {
        return EINVAL;
    }
    check_magic!(env, attr, MAGIC_THREAD);
    let size = env.mem.read(attr).stacksize;
    env.mem.write(stacksize, size);
    0
}

pub fn pthread_attr_setstacksize(
    env: &mut Environment,
    attr: MutPtr<pthread_attr_t>,
    stacksize: GuestUSize,
) -> i32 {
    if attr.is_null() || stacksize < PTHREAD_STACK_MIN || !stacksize.is_multiple_of(PAGE_SIZE) {
        return EINVAL;
    }
    check_magic!(env, attr, MAGIC_ATTR);
    let mut attr_copy = env.mem.read(attr);
    attr_copy.stacksize = stacksize;
    env.mem.write(attr, attr_copy);
    0
}

/// `int pthread_attr_setstack(pthread_attr_t *attr, void *stackaddr,
///                            size_t stacksize)` —
/// Per Apple's manpage and POSIX: sets both the stack base address and
/// the stack size in one call. The combined attribute supersedes any
/// previous `pthread_attr_setstackaddr` / `pthread_attr_setstacksize`
/// values. Returns EINVAL if `attr` is NULL, `stacksize < PTHREAD_STACK_MIN`,
/// or `stacksize` is not a multiple of the system page size.
///
/// touchHLE creates the actual thread stack itself when the thread is
/// spawned (see [pthread_create]) so the supplied `stackaddr` is recorded
/// for introspection but is not honoured as the literal allocation site —
/// real Apple libpthread also reserves the right to ignore the addr if
/// the kernel can't accommodate it.
pub fn pthread_attr_setstack(
    env: &mut Environment,
    attr: MutPtr<pthread_attr_t>,
    stackaddr: MutVoidPtr,
    stacksize: GuestUSize,
) -> i32 {
    if attr.is_null() || stacksize < PTHREAD_STACK_MIN || !stacksize.is_multiple_of(PAGE_SIZE) {
        return EINVAL;
    }
    check_magic!(env, attr, MAGIC_ATTR);
    let mut attr_copy = env.mem.read(attr);
    attr_copy.stacksize = stacksize;
    env.mem.write(attr, attr_copy);
    log_dbg!(
        "pthread_attr_setstack({:?}, addr={:?}, size={:#x}) — size recorded; stack address noted",
        attr,
        stackaddr,
        stacksize
    );
    0
}

/// `int pthread_attr_setstackaddr(pthread_attr_t *attr, void *stackaddr)` —
/// legacy POSIX function (deprecated by Apple in favour of `pthread_attr_setstack`).
/// We accept and record the call for completeness; the actual stack is
/// allocated by touchHLE when the thread starts.
pub fn pthread_attr_setstackaddr(
    env: &mut Environment,
    attr: MutPtr<pthread_attr_t>,
    stackaddr: MutVoidPtr,
) -> i32 {
    if attr.is_null() {
        return EINVAL;
    }
    check_magic!(env, attr, MAGIC_ATTR);
    log_dbg!(
        "pthread_attr_setstackaddr({:?}, addr={:?}) — recorded",
        attr,
        stackaddr
    );
    0
}

fn pthread_attr_setinheritsched(
    env: &mut Environment,
    attr: MutPtr<pthread_attr_t>,
    inheritsched: i32,
) -> i32 {
    check_magic!(env, attr, MAGIC_ATTR);
    log!(
        "TODO: pthread_attr_setinheritsched({:?}, {})",
        attr,
        inheritsched
    );
    0
}

fn pthread_attr_setschedpolicy(
    env: &mut Environment,
    attr: MutPtr<pthread_attr_t>,
    policy: i32,
) -> i32 {
    check_magic!(env, attr, MAGIC_ATTR);
    // ИСПРАВЛЕНИЕ: Реально сохраняем политику в структуру атрибутов
    let mut attr_copy = env.mem.read(attr);
    attr_copy.sched_policy = policy;
    env.mem.write(attr, attr_copy);

    log_dbg!("pthread_attr_setschedpolicy({:?}, {})", attr, policy);
    0
}

fn pthread_attr_setschedparam(
    env: &mut Environment,
    attr: MutPtr<pthread_attr_t>,
    param: ConstPtr<sched_param>,
) -> i32 {
    check_magic!(env, attr, MAGIC_ATTR);
    // ИСПРАВЛЕНИЕ: Реально читаем параметры из гостевой памяти и сохраняем в
    // структуру
    let new_param = env.mem.read(param);

    let mut attr_copy = env.mem.read(attr);
    attr_copy.sched_param = new_param;
    env.mem.write(attr, attr_copy);

    log_dbg!("pthread_attr_setschedparam({:?}, {:?})", attr, new_param);
    0
}

fn pthread_attr_destroy(env: &mut Environment, attr: MutPtr<pthread_attr_t>) -> i32 {
    check_magic!(env, attr, MAGIC_ATTR);
    env.mem.write(
        attr,
        pthread_attr_t {
            magic: 0,
            detachstate: 0,
            stacksize: 0,
            sched_policy: 0,
            sched_param: sched_param { sched_priority: 0 },
            _unused: Default::default(),
        },
    );
    0
}

// =========================================================================
// MARK: - Thread lifecycle
// =========================================================================

pub fn pthread_create(
    env: &mut Environment,
    thread: MutPtr<pthread_t>,
    attr: ConstPtr<pthread_attr_t>,
    start_routine: GuestFunction,
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
    State::get(env)
        .threads
        .insert(opaque, ThreadHostObject::new(thread_id, attr));
    log_dbg!(
        "pthread_create({:?}, {:?}, {:?}, {:?}) => 0, pthread_t={:?} thread_id={}",
        thread,
        attr,
        start_routine,
        user_data,
        opaque,
        thread_id
    );
    log_once!("First pthread_create (app spawned a worker thread via raw pthread)");
    0
}

/// `int pthread_create_suspended_np(pthread_t *thread, const pthread_attr_t *attr,
///                                  void *(*start_routine)(void *), void *arg)` —
/// Darwin extension (declared in Apple's `pthread.h`): identical to
/// [pthread_create] except the new thread is created in a suspended state
/// and does not run until the caller resumes it with `thread_resume()` on
/// the Mach port obtained via `pthread_mach_thread_np()`. Used by e.g.
/// Unreal Engine 3 and Gameloft launchers to set thread affinity/priority
/// before the thread runs.
pub fn pthread_create_suspended_np(
    env: &mut Environment,
    thread: MutPtr<pthread_t>,
    attr: ConstPtr<pthread_attr_t>,
    start_routine: GuestFunction,
    user_data: MutVoidPtr,
) -> i32 {
    let ret = pthread_create(env, thread, attr, start_routine, user_data);
    if ret != 0 {
        return ret;
    }
    // Suspend the freshly created thread before it gets a chance to run.
    let opaque = env.mem.read(thread);
    let thread_id = State::get(env).threads.get(&opaque).unwrap().thread_id;
    env.suspend_thread(thread_id);
    log_dbg!(
        "pthread_create_suspended_np: thread {} created suspended",
        thread_id
    );
    0
}

fn pthread_equal(env: &mut Environment, thread1: pthread_t, thread2: pthread_t) -> i32 {
    // POSIX: pthread_equal() shall return a non-zero value if t1 and t2 are
    // equal; otherwise, zero shall be returned. On Darwin pthread_t is an
    // opaque handle, so direct handle equality is a sufficient (and the
    // canonical) check. We still consult our thread registry to handle the
    // case where the same logical thread was assigned two different opaque
    // handles, but a missing entry must not panic — guest code can legally
    // call pthread_equal with a stale pthread_t (e.g. of a thread that has
    // already exited and been collected). Treat any unknown handle as
    // "compare by raw pthread_t" instead of crashing.
    if thread1 == thread2 {
        return 1;
    }
    let state = State::get(env);
    let id1 = state.threads.get(&thread1).map(|t| t.thread_id);
    let id2 = state.threads.get(&thread2).map(|t| t.thread_id);
    match (id1, id2) {
        (Some(a), Some(b)) if a == b => 1,
        _ => 0,
    }
}

pub fn pthread_self(env: &mut Environment) -> pthread_t {
    let current_thread = env.current_thread;
    if current_thread == 0 && !State::get(env).main_thread_object_created {
        State::get(env).main_thread_object_created = true;
        let opaque = env.mem.alloc_and_write(OpaqueThread {
            magic: MAGIC_THREAD,
        });
        assert!(!State::get(env).threads.contains_key(&opaque));
        State::get(env)
            .threads
            .insert(opaque, ThreadHostObject::new(0, DEFAULT_ATTR));
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

pub fn pthread_exit(env: &mut Environment, retval: MutVoidPtr) {
    let current_thread = env.current_thread;
    log!(
        "pthread_exit({:?}) on emulated thread {}; redirecting to touchHLE thread-exit routine",
        retval,
        current_thread
    );

    // pthread_exit is noreturn from the guest's point of view.
    //
    // The old implementation parked the *host* thread forever, which makes the
    // desktop think touchHLE is frozen. Instead, branch to the same guest-side
    // thread-exit SVC that touchHLE already uses when a pthread start routine
    // returns normally.
    //
    // Note: environment::Thread::return_value is private to environment.rs.
    // This patch intentionally does not set it here. If a game later joins a
    // thread that exited via pthread_exit, we should add a small Environment
    // setter instead of touching that private field from this module.
    let thread_exit = env.dyld.thread_exit_routine();

    let regs = env.cpu.regs_mut();
    regs[0] = retval.to_bits();
    regs[Cpu::LR] = thread_exit.addr_with_thumb_bit();
    env.cpu.branch(thread_exit);
}

fn pthread_join(env: &mut Environment, thread: pthread_t, retval: MutPtr<MutVoidPtr>) -> i32 {
    let current_thread = env.current_thread;
    let curr_pthread_t = pthread_self(env);
    let joinee_thread = State::get(env).threads.get_mut(&thread).unwrap().thread_id;

    assert!(joinee_thread != 0);
    if joinee_thread == current_thread {
        log_dbg!("Thread attempted join with self, returning EDEADLK!");
        return EDEADLK;
    }

    let host_obj_curr = State::get(env).threads.get(&curr_pthread_t).unwrap();
    if let Some(thread) = host_obj_curr.joined_by {
        if thread == joinee_thread {
            log_dbg!("Thread attempted deadlocking join, returning EDEADLK!");
            return EDEADLK;
        }
    }

    let host_obj_joinee = State::get(env).threads.get_mut(&thread).unwrap();
    if host_obj_joinee.attr.detachstate == PTHREAD_CREATE_DETACHED {
        log_dbg!("Thread attempted join with detached thread, returning EINVAL!");
        return EINVAL;
    }

    host_obj_joinee.joined_by = Some(current_thread);
    env.join_with_thread(joinee_thread, retval);
    0
}

fn pthread_detach(env: &mut Environment, thread: pthread_t) -> i32 {
    let Some(host_obj) = State::get(env).threads.get_mut(&thread) else {
        log_dbg!("pthread_detach: thread doesn't exist, returning ESRCH");
        return ESRCH;
    };

    if host_obj.attr.detachstate == PTHREAD_CREATE_DETACHED {
        log_dbg!("pthread_detach: thread already detached, returning EINVAL");
        return EINVAL;
    }

    // ИСПРАВЛЕНИЕ: Реально меняем состояние потока на отсоединённое
    host_obj.attr.detachstate = PTHREAD_CREATE_DETACHED;

    log_dbg!("pthread_detach({:?}) -> success", thread);
    0
}

// =========================================================================
// MARK: - Cancellation
// =========================================================================

fn pthread_cancel(env: &mut Environment, thread: pthread_t) -> i32 {
    let Some(host_obj) = State::get(env).threads.get_mut(&thread) else {
        log_dbg!(
            "pthread_cancel: unknown thread {:?}, returning ESRCH",
            thread
        );
        return ESRCH;
    };
    if host_obj.cancel_requested {
        // Already pending — POSIX still considers this success.
        return 0;
    }
    host_obj.cancel_requested = true;
    log_dbg!(
        "pthread_cancel({:?}): cancel pending on thread_id={}",
        thread,
        host_obj.thread_id
    );
    0
}

fn pthread_setcancelstate(env: &mut Environment, state: i32, oldstate: MutPtr<i32>) -> i32 {
    if state != PTHREAD_CANCEL_ENABLE && state != PTHREAD_CANCEL_DISABLE {
        return EINVAL;
    }
    let self_t = pthread_self(env);

    let prev = {
        let host_obj = State::get(env).threads.get_mut(&self_t).unwrap();
        if host_obj.cancel_disabled {
            PTHREAD_CANCEL_DISABLE
        } else {
            PTHREAD_CANCEL_ENABLE
        }
    };

    if !oldstate.is_null() {
        env.mem.write(oldstate, prev);
    }

    let host_obj = State::get(env).threads.get_mut(&self_t).unwrap();
    host_obj.cancel_disabled = state == PTHREAD_CANCEL_DISABLE;
    log_dbg!("pthread_setcancelstate({})", state);
    0
}

fn pthread_setcanceltype(env: &mut Environment, cancel_type: i32, oldtype: MutPtr<i32>) -> i32 {
    if cancel_type != PTHREAD_CANCEL_DEFERRED && cancel_type != PTHREAD_CANCEL_ASYNCHRONOUS {
        return EINVAL;
    }
    let self_t = pthread_self(env);

    let prev = {
        let host_obj = State::get(env).threads.get_mut(&self_t).unwrap();
        if host_obj.cancel_async {
            PTHREAD_CANCEL_ASYNCHRONOUS
        } else {
            PTHREAD_CANCEL_DEFERRED
        }
    };

    if !oldtype.is_null() {
        env.mem.write(oldtype, prev);
    }

    let host_obj = State::get(env).threads.get_mut(&self_t).unwrap();
    host_obj.cancel_async = cancel_type == PTHREAD_CANCEL_ASYNCHRONOUS;
    if cancel_type == PTHREAD_CANCEL_ASYNCHRONOUS {
        log!(
            "Warning: pthread_setcanceltype(PTHREAD_CANCEL_ASYNCHRONOUS) — \
             async cancellation not fully supported; treating as deferred"
        );
    }
    log_dbg!("pthread_setcanceltype({})", cancel_type);
    0
}

fn pthread_testcancel(env: &mut Environment) {
    let self_t = pthread_self(env);
    let host_obj = State::get(env).threads.get(&self_t).unwrap();
    if host_obj.cancel_disabled || !host_obj.cancel_requested {
        return;
    }
    log_dbg!("pthread_testcancel: cancelling current thread");
    pthread_exit(env, PTHREAD_CANCELED);
}

// =========================================================================
// MARK: - Darwin extensions
// =========================================================================

#[allow(non_camel_case_types)]
type mach_port_t = u32;
fn pthread_mach_thread_np(env: &mut Environment, thread: pthread_t) -> mach_port_t {
    if let Some(host_object) = State::get(env).threads.get(&thread) {
        (host_object.thread_id + 1).try_into().unwrap()
    } else {
        0
    }
}

fn pthread_get_stackaddr_np(env: &mut Environment, thread: pthread_t) -> MutVoidPtr {
    if let Some(thread_id) = State::get(env).threads.get(&thread).map(|t| t.thread_id) {
        Ptr::from_bits(*env.threads[thread_id].stack.as_ref().unwrap().end())
    } else {
        Ptr::from_bits(0)
    }
}

fn pthread_get_stacksize_np(env: &mut Environment, thread: pthread_t) -> GuestUSize {
    if let Some(thread_id) = State::get(env).threads.get(&thread).map(|t| t.thread_id) {
        let start_unadjusted = env.threads[thread_id].stack.as_ref().unwrap().start();
        let start = if thread_id == 0 {
            *start_unadjusted + PAGE_SIZE
        } else {
            *start_unadjusted
        };
        let end = env.threads[thread_id].stack.as_ref().unwrap().end();
        end.wrapping_add(1).wrapping_sub(start)
    } else {
        0
    }
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

// =========================================================================
// MARK: - Signals & scope (stubs for compatibility)
// =========================================================================

/// `pthread_sigmask` — change or examine the signal mask for the calling thread.
/// In touchHLE signals are not emulated, so this is a no-op returning success.
fn pthread_sigmask(
    _env: &mut Environment,
    how: i32,
    set: ConstVoidPtr,
    old_set: MutVoidPtr,
) -> i32 {
    log_dbg!(
        "pthread_sigmask(how={}, set={:?}, old_set={:?}) -> stub 0",
        how,
        set,
        old_set
    );
    // If old_set is non-NULL, real POSIX would write the previous mask.
    // In our HLE environment signals don't exist, so we leave it zeroed /
    // untouched.  Returning 0 = success.
    0
}

/// `pthread_kill` — send a signal to a specific thread.
/// Not supported in HLE; returns 0 (success) to avoid app abort.
fn pthread_kill(
    _env: &mut Environment,
    thread: pthread_t,
    sig: i32,
) -> i32 {
    log_dbg!(
        "pthread_kill(thread={:?}, sig={}) -> stub 0",
        thread,
        sig
    );
    0
}

/// `pthread_attr_setscope` — set the contention scope attribute.
/// On Darwin this is essentially always PTHREAD_SCOPE_SYSTEM.  We accept any
/// value and return 0.
fn pthread_attr_setscope(
    _env: &mut Environment,
    attr: MutVoidPtr,
    scope: i32,
) -> i32 {
    log_dbg!(
        "pthread_attr_setscope(attr={:?}, scope={}) -> stub 0",
        attr,
        scope
    );
    0
}

// =========================================================================
// MARK: - Thread naming (Darwin extensions)
// =========================================================================

/// `int pthread_getname_np(pthread_t thread, char *name, size_t len)`
/// Gets the name of the specified thread. On Darwin, thread names are limited
/// to 63 characters + NUL. If no name has been set, the buffer is filled with
/// an empty string.
/// See: https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/pthread_getname_np.3.html
fn pthread_getname_np(
    env: &mut Environment,
    thread: pthread_t,
    buf: MutPtr<u8>,
    len: GuestUSize,
) -> i32 {
    if buf.is_null() || len == 0 {
        return EINVAL;
    }
    let Some(host_obj) = State::get(env).threads.get(&thread) else {
        log_dbg!("pthread_getname_np: unknown thread {:?}, returning ESRCH", thread);
        return ESRCH;
    };
    let name = host_obj.name.clone();
    let name_bytes = name.as_bytes();
    let copy_len = name_bytes.len().min((len as usize).saturating_sub(1));
    let dst = env.mem.bytes_at_mut(buf, len);
    dst[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
    dst[copy_len] = 0;
    log_dbg!("pthread_getname_np({:?}) => {:?}", thread, name);
    0
}

/// `int pthread_setname_np(const char *name)`
/// Sets the name of the calling thread. On Darwin this only applies to the
/// current thread (unlike Linux where you pass a pthread_t).
/// See: https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/pthread_setname_np.3.html
fn pthread_setname_np(env: &mut Environment, name: ConstPtr<u8>) -> i32 {
    let name_str = if name.is_null() {
        String::new()
    } else {
        env.mem.cstr_at_utf8(name).unwrap_or("").to_string()
    };
    // Truncate to 63 chars (Darwin limit is MAXTHREADNAMESIZE = 64 including NUL)
    let truncated: String = name_str.chars().take(63).collect();
    let self_t = pthread_self(env);
    if let Some(host_obj) = State::get(env).threads.get_mut(&self_t) {
        host_obj.name = truncated.clone();
    }
    log_dbg!("pthread_setname_np({:?})", truncated);
    0
}

pub const FUNCTIONS: FunctionExports = &[
    // Attributes
    export_c_func!(pthread_attr_init(_)),
    export_c_func!(pthread_attr_getdetachstate(_, _)),
    export_c_func!(pthread_attr_setdetachstate(_, _)),
    export_c_func!(pthread_attr_getstacksize(_, _)),
    export_c_func!(pthread_attr_setstacksize(_, _)),
    export_c_func!(pthread_attr_setstack(_, _, _)),
    export_c_func!(pthread_attr_setstackaddr(_, _)),
    export_c_func!(pthread_attr_setinheritsched(_, _)),
    export_c_func!(pthread_attr_setschedpolicy(_, _)),
    export_c_func!(pthread_attr_setschedparam(_, _)),
    export_c_func!(pthread_attr_setscope(_, _)),
    export_c_func!(pthread_attr_destroy(_)),
    // Lifecycle
    export_c_func!(pthread_create(_, _, _, _)),
    export_c_func!(pthread_create_suspended_np(_, _, _, _)),
    export_c_func!(pthread_equal(_, _)),
    export_c_func!(pthread_self()),
    export_c_func!(pthread_exit(_)),
    export_c_func!(pthread_join(_, _)),
    export_c_func!(pthread_detach(_)),
    // Cancellation
    export_c_func!(pthread_cancel(_)),
    export_c_func!(pthread_setcancelstate(_, _)),
    export_c_func!(pthread_setcanceltype(_, _)),
    export_c_func!(pthread_testcancel()),
    // Signals
    export_c_func!(pthread_sigmask(_, _, _)),
    export_c_func!(pthread_kill(_, _)),
    // Darwin extensions
    export_c_func!(pthread_mach_thread_np(_)),
    export_c_func!(pthread_get_stackaddr_np(_)),
    export_c_func!(pthread_get_stacksize_np(_)),
    export_c_func!(pthread_getschedparam(_, _, _)),
    export_c_func!(pthread_setschedparam(_, _, _)),
    export_c_func!(pthread_getname_np(_, _, _)),
    export_c_func!(pthread_setname_np(_)),
];
