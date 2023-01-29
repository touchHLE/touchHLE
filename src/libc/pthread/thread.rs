//! Threads.

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, MutPtr, MutVoidPtr, SafeRead};
use crate::Environment;

/// Apple's implementation is a 4-byte magic number followed by an 36-byte
/// opaque region. We only have to match the size theirs has.
#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
struct pthread_attr_t {
    /// Magic number (must be [MAGIC_ATTR])
    magic: u32,
    detachstate: i32,
    _unused: [u32; 8],
}
unsafe impl SafeRead for pthread_attr_t {}

const DEFAULT_ATTR: pthread_attr_t = pthread_attr_t {
    magic: MAGIC_ATTR,
    detachstate: PTHREAD_CREATE_JOINABLE,
    _unused: [0; 8],
};

/// Apple's implementation is a 4-byte magic number followed by a massive
/// (>4KiB) opaque region. We will store the actual data on the host instead.
#[repr(C, packed)]
struct OpaqueThread {
    /// Magic number (must be [MAGIC_THREAD])
    magic: u32,
}
unsafe impl SafeRead for OpaqueThread {}

type pthread_t = MutPtr<OpaqueThread>;

/// Arbitrarily-chosen magic number for `pthread_attr_t` (not Apple's).
const MAGIC_ATTR: u32 = u32::from_be_bytes(*b"ThAt");
/// Arbitrarily-chosen magic number for `pthread_t` (not Apple's).
const _MAGIC_THREAD: u32 = u32::from_be_bytes(*b"THRD");

/// Custom typedef for readability (the C API just uses `int`)
type DetachState = i32;
const PTHREAD_CREATE_JOINABLE: DetachState = 1;
const PTHREAD_CREATE_DETACHED: DetachState = 2;

fn pthread_attr_init(env: &mut Environment, attr: MutPtr<pthread_attr_t>) -> i32 {
    env.mem.write(attr, DEFAULT_ATTR);
    0 // success
}
fn pthread_attr_setdetachstate(
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
fn pthread_attr_destroy(env: &mut Environment, attr: MutPtr<pthread_attr_t>) -> i32 {
    check_magic!(env, attr, MAGIC_ATTR);
    env.mem.write(
        attr,
        pthread_attr_t {
            magic: 0,
            detachstate: 0,
            _unused: Default::default(),
        },
    );
    0 // success
}

fn pthread_create(
    env: &mut Environment,
    _thread: MutPtr<pthread_t>,
    attr: ConstPtr<pthread_attr_t>,
    start_routine: GuestFunction, // (*void)(void *)
    arg: MutVoidPtr,
) -> i32 {
    let attr = if !attr.is_null() {
        check_magic!(env, attr, MAGIC_ATTR);
        env.mem.read(attr)
    } else {
        DEFAULT_ATTR
    };

    unimplemented!(
        "Create thread with {:?}, {:?}, {:?}",
        attr,
        start_routine,
        arg
    ); // TODO
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(pthread_attr_init(_)),
    export_c_func!(pthread_attr_setdetachstate(_, _)),
    export_c_func!(pthread_attr_destroy(_)),
    export_c_func!(pthread_create(_, _, _, _)),
];
