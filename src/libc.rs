//! Our implementations of various things that Apple's libSystem would provide.
//! On other platforms these are part of the "libc", so let's call it that.
//!
//! Useful resources:
//! - Apple's [iOS Manual Pages](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/) (contains what would be `man` pages if iOS had a command line)

pub mod ctype;
pub mod cxxabi;
pub mod dlfcn;
pub mod errno;
pub mod keymgr;
pub mod mach_thread_info;
pub mod mach_time;
pub mod math;
pub mod pthread;
pub mod stdio;
pub mod stdlib;
pub mod string;
pub mod time;

/// Container for state of various child modules
#[derive(Default)]
pub struct State {
    keymgr: keymgr::State,
    pthread: pthread::State,
    stdio: stdio::State,
    stdlib: stdlib::State,
    string: string::State,
    time: time::State,
}
