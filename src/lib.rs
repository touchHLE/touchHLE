/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(non_snake_case)]
#![allow(rustdoc::private_intra_doc_links)]

#[macro_use]
pub mod log; 

pub mod abi;
pub mod audio;
pub mod bundle;
pub mod cpu;
pub mod debug;
pub mod dyld;
pub mod environment;
pub mod font;
pub mod frameworks;
pub mod fs;
pub mod gdb;
pub mod gles;
pub mod image;

pub mod libc {
    pub mod clocale;
    pub mod ctype;
    pub mod dirent;   // Fixes mount.rs:9
    pub mod errno;
    pub mod keymgr;
    pub mod mach;
    pub mod mmap;
    pub mod netdb;    // Fixes socket.rs:34
    pub mod posix_io;
    pub mod pthread;
    pub mod semaphore;
    pub mod sqlite; 
    pub mod stdio;
    pub mod stdlib;
    pub mod string;
    pub mod sys;
    pub mod time;
    pub mod unistd;
    pub mod wchar;
    pub mod generic_char;
    
    // Fixes environment.rs:106 (cannot find type State)
    pub use self::posix_io::State;

    // Fixes dylib_list.rs:15 (cannot find value DYLIB)
    // In touchHLE, the type is usually 'Library', not 'Dylib'
    pub const DYLIB: crate::dyld::Library = crate::dyld::Library::Native("libc");
}

pub mod licenses;
pub mod mach_o;
pub mod matrix;
pub mod mem;
pub mod objc;
pub mod options;
pub mod paths;
pub mod stack;
pub mod window;

// Fixes mutex.rs:14 (unresolved import PTHREAD_MUTEX_DEFAULT)
pub const PTHREAD_MUTEX_DEFAULT: i32 = 0;

pub use environment::{Environment, MutexId, MutexType, ThreadId};
use std::path::PathBuf;

pub use touchHLE_version::*;

// ... [Keep your SDL_main and main functions here] ...
