/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#[macro_use]
pub mod log;

// --- Essential Modules ---
pub mod abi;
pub mod cpu;
pub mod dyld;
pub mod environment;
pub mod frameworks;
pub mod fs;
pub mod libc;
pub mod mach_o;
pub mod mem;
pub mod objc;
pub mod paths;

// --- Re-exports and Type Aliases ---
pub use crate::environment::Environment;

// Resolves E0432: unresolved import `crate::ThreadId`
pub type ThreadId = std::thread::ThreadId;
pub type MutexId = u32; 
pub const PTHREAD_MUTEX_DEFAULT: i32 = 0;

// --- State Definitions ---

#[derive(Default)]
pub struct LibcState {
    pub posix_io: libc::posix_io::State,
    pub errno: i32,
}

// --- Macros (Resolves unused macro warnings) ---

#[macro_export]
macro_rules! log_no_panic {
    ($($arg:tt)*) => {
        // Implementation for logging that won't panic
        println!("[LOG] {}", format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! echo_no_panic {
    ($($arg:tt)*) => {
        println!("[ECHO] {}", format_args!($($arg)*));
    };
}

pub fn step(env: &mut Environment) {
    env.cpu.step(&mut env.mem);
}
