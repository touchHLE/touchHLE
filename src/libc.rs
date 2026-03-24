/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Implementations of various things that Apple's libSystem would provide.

mod generic_char;

pub mod arpa;
pub mod clocale;
pub mod crypto;
pub mod ctype;
pub mod cxxabi;
pub mod dirent;
pub mod dlfcn;
pub mod dns_sd;
pub mod errno;
pub mod ifaddrs;
pub mod keymgr;
pub mod libkern;
pub mod mach;
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

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/usr/lib/libSystem.B.dylib",
    aliases: &["/usr/lib/libSystem.dylib"],
    class_exports: &[],
    constant_exports: &[ctype::CONSTANTS, stdio::CONSTANTS, mach::init::CONSTANTS],
    function_exports: &[
        arpa::inet::FUNCTIONS,
        clocale::FUNCTIONS,
        ctype::FUNCTIONS,
        cxxabi::FUNCTIONS,
        crypto::FUNCTIONS,
        dirent::FUNCTIONS,
        dlfcn::FUNCTIONS,
        dns_sd::FUNCTIONS,
        errno::FUNCTIONS,
        ifaddrs::FUNCTIONS,
        keymgr::FUNCTIONS,
        libkern::os_atomic::FUNCTIONS,
        mach::host::FUNCTIONS,
        mach::init::FUNCTIONS,
        mach::semaphore::FUNCTIONS,
        mach::thread_info::FUNCTIONS,
        mach::time::FUNCTIONS,
        math::FUNCTIONS,
        mmap::FUNCTIONS,
        net::if_::FUNCTIONS,
        netdb::FUNCTIONS,
        posix_io::FUNCTIONS,
        posix_io::stat::FUNCTIONS,
        posix_io::statvfs::FUNCTIONS,
        pthread::cond::FUNCTIONS,
        pthread::key::FUNCTIONS,
        pthread::mutex::FUNCTIONS,
        pthread::once::FUNCTIONS,
        pthread::thread::FUNCTIONS,
        sched::FUNCTIONS,
        semaphore::FUNCTIONS,
        setjmp::FUNCTIONS,
        signal::FUNCTIONS,
        stdio::FUNCTIONS,
        stdio::printf::FUNCTIONS,
        stdlib::FUNCTIONS,
        stdlib::qsort::FUNCTIONS,
        string::FUNCTIONS,
        sys::mount::FUNCTIONS,
        sys::ptrace::FUNCTIONS,
        sys::timeb::FUNCTIONS,
        sys::socket::FUNCTIONS,
        sys::utsname::FUNCTIONS,
        sys::wait::FUNCTIONS,
        sysctl::FUNCTIONS,
        time::FUNCTIONS,
        unistd::FUNCTIONS,
        wchar::FUNCTIONS,
    ],
};

/// Container for state of various child modules
#[derive(Default)]
pub struct State {
    pub dirent: dirent::State,
    pub keymgr: keymgr::State,
    pub math: math::State,
    pub posix_io: posix_io::State,
    pub pthread: pthread::State,
    pub semaphore: semaphore::State,
    pub socket: sys::socket::State,
    pub stdlib: stdlib::State,
    pub string: string::State,
    pub stdio: stdio::State,
    pub time: time::State,
    pub errno: errno::State,
    pub clocale: clocale::State,
    pub mmap: mmap::State,
}
