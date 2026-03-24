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
    pub mod dirent;
    pub mod errno;
    pub mod keymgr;
    pub mod mach;
    pub mod mmap;
    pub mod netdb;
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

    pub use self::posix_io::State;
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

pub const PTHREAD_MUTEX_DEFAULT: i32 = 0;

pub use environment::{Environment, MutexId, MutexType, ThreadId};
