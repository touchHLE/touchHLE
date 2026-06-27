/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `pthread_once`.

use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{MutPtr, SafeRead};
use crate::Environment;

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
unsafe impl SafeRead for pthread_once_t {}

fn pthread_once(
    env: &mut Environment,
    once_control: MutPtr<pthread_once_t>,
    init_routine: GuestFunction, // void (*init_routine)(void)
) -> i32 {
    let pthread_once_t { magic, init } = env.mem.read(once_control);
    if magic != MAGIC_ONCE {
        // The caller passed something that isn't a pthread_once_t (e.g. it
        // forgot to initialise the storage with PTHREAD_ONCE_INIT). Real
        // Apple libc would dereference whatever garbage is there and likely
        // crash; we have no good way to know what the caller intended.
        // Treat the slot as fresh (initialise it to MAGIC_ONCE + init=0) and
        // run the init routine once, so the guest at least sees a valid
        // pthread_once_t after the call.
        log!(
            "Warning: pthread_once(): once_control at {:?} has bad magic {:#x}; re-initialising and running init routine once.",
            once_control,
            magic
        );
        let new_once = pthread_once_t {
            magic: MAGIC_ONCE,
            init: 0xFFFFFFFF,
        };
        env.mem.write(once_control, new_once);
        () = init_routine.call_from_host(env, ());
        return 0;
    }
    match init {
        0 => {
            log_dbg!(
                "pthread_once_t at {:?} hasn't been run yet, running init routine {:?}",
                once_control,
                init_routine
            );
            let new_once = pthread_once_t {
                magic,
                init: 0xFFFFFFFF,
            };
            env.mem.write(once_control, new_once);
            () = init_routine.call_from_host(env, ());
            log_dbg!("Init routine {:?} done", init_routine);
        }
        0xFFFFFFFF => {
            log_dbg!(
                "pthread_once_t at {:?} has already been run, doing nothing",
                once_control
            );
        }
        other => {
            // Some other intermediate state we don't model (e.g. another
            // thread is currently running the init routine on real Apple
            // libc). The least surprising behaviour is to treat it as
            // already-initialised so we don't re-enter the init routine.
            log!(
                "Warning: pthread_once(): once_control at {:?} has unexpected init state {:#x}; treating as already initialised.",
                once_control,
                other
            );
        }
    };
    0 // success. TODO: return an error on failure?
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(pthread_once(_, _))];
