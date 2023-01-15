//! Our implementations of various things that Apple's libSystem would provide.
//! On other platforms these are part of the "libc", so let's call it that.
//!
//! Useful resources:
//! - Apple's [iOS Manual Pages](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/) (contains what would be `man` pages if iOS had a command line)

pub mod cxxabi;
pub mod errno;
pub mod mach_time;
pub mod math;
pub mod pthread;
pub mod stdio;
pub mod stdlib;
pub mod string;

/// Container for state of various child modules
#[derive(Default)]
pub struct State {
    pthread: pthread::State,
    stdio: stdio::State,
}
