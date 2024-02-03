/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Our implementations of various things that Apple's libSystem would provide.
//! On other platforms these are part of the "libc", so let's call it that.
//!
//! Useful resources:
//! - Apple's [iOS Manual Pages](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/) (contains what would be `man` pages if iOS had a command line)

mod generic_char;

pub mod clocale;
pub mod crypto;
pub mod ctype;
pub mod cxxabi;
pub mod dirent;
pub mod dlfcn;
pub mod errno;
pub mod ifaddrs;
pub mod keymgr;
pub mod mach_host;
pub mod mach_init;
pub mod mach_semaphore;
pub mod mach_thread_info;
pub mod mach_time;
pub mod math;
pub mod mmap;
pub mod net;
pub mod netdb;
pub mod posix_io;
pub mod pthread;
pub mod sched;
pub mod semaphore;
pub mod setjmp;
pub mod signal;
pub mod stdio;
pub mod stdlib;
pub mod string;
pub mod sys;
pub mod sysctl;
pub mod time;
pub mod unistd;
pub mod wchar;

/// Container for state of various child modules
#[derive(Default)]
pub struct State {
    dirent: dirent::State,
    keymgr: keymgr::State,
    mach_semaphore: mach_semaphore::State,
    posix_io: posix_io::State,
    pub pthread: pthread::State,
    pub semaphore: semaphore::State,
    stdlib: stdlib::State,
    string: string::State,
    time: time::State,
    errno: errno::State,
    clocale: clocale::State,
}
