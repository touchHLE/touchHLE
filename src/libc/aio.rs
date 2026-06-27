/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! POSIX asynchronous I/O (`aio.h`).
//!
//! Several games (e.g. Plants vs. Zombies HD) stream their resource files
//! through the POSIX `aio_read`/`aio_error`/`aio_return` family instead of
//! plain `read(2)`. Previously these symbols were missing, so dyld installed
//! return-0 stubs: `aio_read` reported "successfully enqueued", `aio_error`
//! reported "completed, no error" and `aio_return` reported "0 bytes
//! transferred". The guest therefore believed every load succeeded while the
//! destination buffers stayed empty, which downstream manifested as a black
//! screen followed by a crash (an empty buffer led to a bogus
//! `malloc((size_t)-1)`).
//!
//! HyperHLE runs the guest's main thread synchronously and there is no real
//! kernel AIO queue to talk to, so we implement these operations
//! synchronously: `aio_read`/`aio_write` perform the transfer immediately
//! (reusing the regular positional `pread`/`pwrite` paths) and stash the
//! outcome keyed by the guest `struct aiocb *`. `aio_error` then reports the
//! stored completion status and `aio_return` hands back the byte count. From
//! the app's point of view this is indistinguishable from an async request
//! that had already finished by the time it polled.

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::{get_errno, set_errno, EINVAL};
use crate::libc::posix_io::{off_t, pread, pwrite, FileDescriptor};
use crate::mem::{ConstPtr, ConstVoidPtr, GuestISize, GuestUSize, MutVoidPtr, SafeRead};
use crate::Environment;
use std::collections::HashMap;

/// Outcome of a completed (synchronous) aio operation.
#[derive(Copy, Clone)]
struct AioResult {
    /// Value reported by `aio_error`: 0 on success, otherwise an `errno`.
    error: i32,
    /// Value reported by `aio_return`: bytes transferred, or -1 on error.
    bytes: GuestISize,
}

#[derive(Default)]
pub struct State {
    /// Completed operations keyed by the guest address of their `struct aiocb`.
    results: HashMap<GuestUSize, AioResult>,
}

/// Prefix of Darwin's 32-bit `struct aiocb` covering the fields we need, up to
/// the `sigev_notify` member of the embedded `struct sigevent`. The real struct
/// continues past this, but reading only this prefix is fine because the guest
/// always allocates the full structure.
///
/// On the iOS armv7 ABI 64-bit scalars such as `off_t` are only 4-byte
/// aligned, so `aio_offset` sits immediately after the leading `int` with no
/// padding — hence `#[repr(C, packed)]`, matching how the rest of HyperHLE
/// models guest structs (`flock`, `iovec`, …). The resulting field offsets are:
/// `aio_fildes` 0, `aio_offset` 4, `aio_buf` 12, `aio_nbytes` 16,
/// `aio_reqprio` 20, `aio_sigevent.sigev_notify` 24.
#[repr(C, packed)]
struct aiocb {
    aio_fildes: FileDescriptor,
    aio_offset: off_t,
    aio_buf: MutVoidPtr,
    aio_nbytes: GuestUSize,
    _aio_reqprio: i32,
    sigev_notify: i32,
}
unsafe impl SafeRead for aiocb {}

/// `sigev_notify` value meaning "no completion notification" — the app will
/// poll with `aio_error`/`aio_return` instead. This is by far the common case.
const SIGEV_NONE: i32 = 0;

/// `LIO_NOWAIT`/`LIO_WAIT` opcode constants are not needed here, but the
/// `aio_cancel` return values are part of the documented ABI.
const AIO_CANCELED: i32 = 2;
const AIO_ALLDONE: i32 = 1;

fn do_transfer(env: &mut Environment, aiocbp: ConstPtr<aiocb>, is_write: bool) -> i32 {
    set_errno(env, 0);
    if aiocbp.is_null() {
        set_errno(env, EINVAL);
        return -1;
    }

    let cb = env.mem.read(aiocbp);
    let fildes = cb.aio_fildes;
    let offset = cb.aio_offset;
    let buf = cb.aio_buf;
    let nbytes = cb.aio_nbytes;

    // We complete the transfer synchronously, so SIGEV_NONE (the common case —
    // the app polls with aio_error/aio_return) is handled perfectly. We do not
    // yet deliver SIGEV_SIGNAL/SIGEV_THREAD completion notifications, though, so
    // surface that as a warning rather than silently leaving an app waiting on
    // a callback that never fires.
    let sigev_notify = cb.sigev_notify;
    if sigev_notify != SIGEV_NONE {
        log!(
            "Warning: aio request on fd {} asks for completion notification \
             (sigev_notify={}), which is not yet delivered; the app may block \
             if it relies on it instead of polling aio_error/aio_return.",
            fildes,
            sigev_notify
        );
    }

    let bytes = if is_write {
        pwrite(env, fildes, buf.cast_const(), nbytes, offset)
    } else {
        pread(env, fildes, buf, nbytes, offset)
    };

    let result = if bytes < 0 {
        AioResult {
            // pread/pwrite already set errno; mirror it into the stored
            // completion status so aio_error reports the same thing.
            error: get_errno(env),
            bytes: -1,
        }
    } else {
        AioResult { error: 0, bytes }
    };
    env.libc_state.aio.results.insert(aiocbp.to_bits(), result);

    log_dbg!(
        "aio_{}({:?}) fd={} offset={} nbytes={:#x} => {} bytes (synchronous)",
        if is_write { "write" } else { "read" },
        aiocbp,
        fildes,
        offset,
        nbytes,
        bytes
    );

    // The enqueue itself succeeded.
    0
}

fn aio_read(env: &mut Environment, aiocbp: ConstPtr<aiocb>) -> i32 {
    do_transfer(env, aiocbp, false)
}

fn aio_write(env: &mut Environment, aiocbp: ConstPtr<aiocb>) -> i32 {
    do_transfer(env, aiocbp, true)
}

fn aio_error(env: &mut Environment, aiocbp: ConstPtr<aiocb>) -> i32 {
    // Returns 0 if the operation completed successfully, the operation's
    // errno on failure, or EINPROGRESS if still running. Because we run
    // synchronously, an op we've seen is always complete; an unknown pointer
    // is treated as "nothing pending", i.e. success.
    env.libc_state
        .aio
        .results
        .get(&aiocbp.to_bits())
        .map_or(0, |r| r.error)
}

fn aio_return(env: &mut Environment, aiocbp: ConstPtr<aiocb>) -> GuestISize {
    // aio_return retrieves the result exactly once and frees the bookkeeping.
    env.libc_state
        .aio
        .results
        .remove(&aiocbp.to_bits())
        .map_or(0, |r| r.bytes)
}

fn aio_suspend(
    _env: &mut Environment,
    _aiocb_list: ConstPtr<ConstPtr<aiocb>>,
    _nent: i32,
    _timeout: ConstVoidPtr,
) -> i32 {
    // Every operation we accept has already completed by the time it is
    // polled, so there is never anything to wait for.
    0
}

fn aio_cancel(env: &mut Environment, _fildes: FileDescriptor, aiocbp: ConstPtr<aiocb>) -> i32 {
    // Synchronous ops are never outstanding, so nothing can be cancelled.
    if !aiocbp.is_null() && env.libc_state.aio.results.contains_key(&aiocbp.to_bits()) {
        AIO_ALLDONE
    } else {
        // No matching outstanding request; report all-done as well, which is
        // the safe answer for an emulator that completes everything inline.
        let _ = AIO_CANCELED;
        AIO_ALLDONE
    }
}

fn aio_fsync(_env: &mut Environment, _op: i32, _aiocbp: ConstPtr<aiocb>) -> i32 {
    // Data is written through to the host file synchronously already.
    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(aio_read(_)),
    export_c_func!(aio_write(_)),
    export_c_func!(aio_error(_)),
    export_c_func!(aio_return(_)),
    export_c_func!(aio_suspend(_, _, _)),
    export_c_func!(aio_cancel(_, _)),
    export_c_func!(aio_fsync(_, _)),
];
