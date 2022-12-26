//! POSIX Threads implementation.

#![allow(non_camel_case_types)]

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{MutPtr, MutVoidPtr, Ptr, SafeRead};
use crate::Environment;

#[derive(Default)]
pub struct State {
    /// The `pthread_key_t` value is the index into this vector. The tuple
    /// contains the pointer to the thread-specific data for the main thread
    /// (currently no other threads exist) and the destructor pointer.
    keys: Vec<(MutVoidPtr, GuestFunction)>,
}

fn get_state(env: &mut Environment) -> &mut State {
    &mut env.libc_state.pthread
}

/// Magic number used in `PTHREAD_ONCE_INIT`. This is part of the ABI!
const MAGIC_ONCE: u32 = 0x30B1BCBA;

#[repr(C, packed)]
struct pthread_once_t {
    /// Magic number (must be [MAGIC_ONCE])
    magic: u32,
    /// Boolean marking whether this has been initialised yet. This seems to be
    /// initialized to zero.
    init: u32,
}
impl SafeRead for pthread_once_t {}

type pthread_key_t = u32;

fn pthread_once(
    env: &mut Environment,
    once_control: MutPtr<pthread_once_t>,
    init_routine: GuestFunction, // void (*init_routine)(void)
) -> i32 {
    let pthread_once_t { magic, init } = env.mem.read(once_control);
    assert!(magic == MAGIC_ONCE);
    match init {
        0 => {
            let new_once = pthread_once_t {
                magic,
                init: 0xFFFFFFFF,
            };
            env.mem.write(once_control, new_once);
            init_routine.call(env);
        }
        0xFFFFFFFF => (), // already initialized, do nothing
        _ => panic!(),
    };
    0 // success. TODO: return an error on failure?
}

fn pthread_key_create(
    env: &mut Environment,
    key_ptr: MutPtr<pthread_key_t>,
    destructor: GuestFunction, // void (*destructor)(void *), may be NULL
) -> i32 {
    let key: pthread_key_t = get_state(env).keys.len().try_into().unwrap();
    get_state(env).keys.push((Ptr::null(), destructor));
    env.mem.write(key_ptr, key);
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(pthread_once(_, _)),
    export_c_func!(pthread_key_create(_, _)),
];
