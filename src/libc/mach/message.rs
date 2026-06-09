/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Mach IPC message system.
//!
//! Below messaging interface is the core of Mach's convoluted
//! messaging system for the interprocess communication.
//!
//! So how do we cope with that (considerable) complexity, which involves
//! different processes (or tasks), ports, port's rights, messages, messaging
//! queues, synchronization - all in and out seasoned with dull as hell Apple's
//! own documentation?
//!
//! Well... First, we only have one process (or task) and it will be like this
//! for the time being, so no "real" IPC here (thanks, for god's sake!).
//! Second, the only known use case so far is the Unity's one -
//! mono's mach exception thread which just catches thread
//! exceptions in the loop. (see [mini-darwin.c](https://github.com/mono/mono/blob/62121afbb28f0b62f100ec9a942d10c5e0f4814f/mono/mini/mini-darwin.c#L131))
//!
//! ~~Thus, by a divine benevolence, we stub those functions and
//! hope that no exception will ever happen! amen~~
//!
//! More seriously, as we would prefer to crash on exceptions anyway,
//! it should be fine to just have stubs.
//!
//! Useful resources:
//! - If you want to go deeper, check out "Chapter 4: Inter Process Communication" of [The GNU Mach Reference Manual](https://www.gnu.org/software/hurd/gnumach-doc/mach.pdf).

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::mach::core_types::{boolean_t, integer_t, natural_t};
use crate::libc::mach::thread_info::{kern_return_t, KERN_SUCCESS};
use crate::mem::MutVoidPtr;
use crate::Environment;

type mach_msg_return_t = kern_return_t;
type mach_port_name_t = natural_t;

type mach_msg_option_t = integer_t;
type mach_msg_size_t = natural_t;
type mach_msg_timeout_t = natural_t;

#[allow(clippy::too_many_arguments)]
fn mach_msg(
    _env: &mut Environment,
    msg: MutVoidPtr, // TODO: use MutPtr<mach_msg_header_t>,
    option: mach_msg_option_t,
    send_size: mach_msg_size_t,
    rcv_size: mach_msg_size_t,
    rcv_name: mach_port_name_t,
    timeout: mach_msg_timeout_t,
    notify: mach_port_name_t,
) -> mach_msg_return_t {
    log_once!("TODO: mach_msg send/rcv");
    // Performance note: Even with the stub, in the case of running in the
    // exception thread (Unity's case), this would become a busy loop.
    // Possibly, some performance optimization could be done here.
    // TODO: optimize perf if needed
    log_dbg!(
        "TODO: mach_msg({:?}, {}, {}, {}, {}, {}, {})",
        msg,
        option,
        send_size,
        rcv_size,
        rcv_name,
        timeout,
        notify
    );
    // Note: Because Unity _do_ check the return value of this function
    // with an assert, we must return a success here.
    // (See [mini-darwin.c](https://github.com/mono/mono/blob/62121afbb28f0b62f100ec9a942d10c5e0f4814f/mono/mini/mini-darwin.c#L139))
    KERN_SUCCESS
}

/// This function is to `Handle kernel-reported thread exception.`
/// See [exc_server](https://web.mit.edu/darwin/src/modules/xnu/osfmk/man/exc_server.html) for more details.
fn exc_server(
    _env: &mut Environment,
    request_msg: MutVoidPtr, // TODO: use MutPtr<mach_msg_header_t>,
    reply_msg: MutVoidPtr,   // TODO: use MutPtr<mach_msg_header_t>,
) -> boolean_t {
    log_dbg!("TODO: exc_server({:?}, {:?})", request_msg, reply_msg);
    // Note: Because Unity _doesn't_ check the return value of this function
    // with an assert, we can just return a false here.
    // (See [mini-darwin.c](https://github.com/mono/mono/blob/62121afbb28f0b62f100ec9a942d10c5e0f4814f/mono/mini/mini-darwin.c#L142))
    1 // FALSE
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(mach_msg(_, _, _, _, _, _, _)),
    export_c_func!(exc_server(_, _)),
];
