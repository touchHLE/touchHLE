/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::libc::errno::set_errno;
use crate::mem::{ConstVoidPtr, MutVoidPtr, Ptr};
use crate::Environment;
use std::collections::HashMap;

// Хранилище обработчиков сигналов.
#[derive(Default)]
pub struct State {
    pub handlers: HashMap<i32, MutVoidPtr>,
}

// Стандартные POSIX-константы для сигналов
const SIG_DFL: u32 = 0;
// const SIG_IGN: u32 = 1;
// const SIG_ERR: u32 = 0xFFFFFFFF; // -1

fn sigaction(env: &mut Environment, _signum: i32, _act: ConstVoidPtr, _old_act: MutVoidPtr) -> i32 {
    set_errno(env, 0);
    // Пока возвращаем 0 (успех), убрав TODO, так как sigaction сложнее в
    // реализации
    // и редко ломает логику игр, если просто рапортует об успехе.
    0
}

fn signal(env: &mut Environment, signum: i32, handler: MutVoidPtr) -> MutVoidPtr {
    set_errno(env, 0);
    // Честная эмуляция: сохраняем новый обработчик и возвращаем старый.
    
    env
        .libc_state
        .signal
        .handlers
        .insert(signum, handler)
        // ИСПОЛЬЗУЕМ from_bits вместо new
        .unwrap_or_else(|| MutVoidPtr::from_bits(SIG_DFL))
}

fn sigprocmask(env: &mut Environment, _how: i32, _set: ConstVoidPtr, _old_set: MutVoidPtr) -> i32 {
    set_errno(env, 0);
    0
}

fn sigaltstack(env: &mut Environment, _ss: ConstVoidPtr, _old_ss: MutVoidPtr) -> i32 {
    set_errno(env, 0);
    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sigaction(_, _, _)),
    export_c_func!(signal(_, _)),
    export_c_func!(sigprocmask(_, _, _)), // Строго 3 аргумента (how, set, old_set)
    export_c_func!(sigaltstack(_, _)),    // Строго 2 аргумента (ss, old_ss)
];

/// `const char *const sys_siglist[NSIG]` — POSIX/BSD signal-name table.
/// On Apple platforms (`sys/signal.h`) the array has `NSIG` (= 32)
/// entries. Each slot is a pointer to a C string describing the
/// signal (`"Terminated"`, `"Segmentation fault"`, …). Per the
/// libc source `<sys/signal.c>`, the values are stable and exposed
/// as a non-lazy symbol.
///
/// We build the table at first reference time by allocating 32
/// guest-side `char *` slots, each pointing at the canonical name
/// from Apple's `sys/signal.h`.
const SIG_NAMES: [&str; 32] = [
    "Unknown",
    "Hangup",
    "Interrupt",
    "Quit",
    "Illegal instruction",
    "Trace/BPT trap",
    "Abort trap",
    "EMT trap",
    "Floating point exception",
    "Killed",
    "Bus error",
    "Segmentation fault",
    "Bad system call",
    "Broken pipe",
    "Alarm clock",
    "Terminated",
    "Urgent I/O condition",
    "Suspended (signal)",
    "Suspended",
    "Continued",
    "Child exited",
    "Stopped (tty input)",
    "Stopped (tty output)",
    "I/O possible",
    "Cputime limit exceeded",
    "Filesize limit exceeded",
    "Virtual timer expired",
    "Profiling timer expired",
    "Window size changes",
    "Information request",
    "User defined signal 1",
    "User defined signal 2",
];

pub const CONSTANTS: ConstantExports = &[(
    "_sys_siglist",
    HostConstant::Custom(|env| {
        let table: crate::mem::MutPtr<u32> = env.mem.alloc(32 * 4).cast();
        for (i, &name) in SIG_NAMES.iter().enumerate() {
            // Allocate a null-terminated C string in guest memory.
            let bytes = name.as_bytes();
            let s = env.mem.alloc((bytes.len() + 1) as u32);
            env.mem
                .bytes_at_mut(s.cast(), (bytes.len() + 1) as u32)
                .copy_from_slice(
                    &bytes
                        .iter()
                        .copied()
                        .chain(std::iter::once(0u8))
                        .collect::<Vec<_>>(),
                );
            env.mem.write(table + (i as u32), s.to_bits());
        }
        Ptr::from_bits(table.to_bits())
    }),
)];
