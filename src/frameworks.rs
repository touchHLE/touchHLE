//! Our implementations of various frameworks.
//!
//! Each child module should be named after the framework it implements.
//! It can potentially have multiple child modules itself if it's a particularly
//! complex framework.
//!
//! See also `dyld/function_lists.rs` and `objc/classes/class_lists.rs`.
//!
//! Most modules in here are not going to link to documentation that should be
//! trivial to find by searching for the class or function name. For example,
//! the documentation of `NSArray` won't link to the main developer.apple.com
//! page documenting that class, but if there's something interesting in the
//! Documentation Archive relating to arrays, that might be linked.

pub mod core_animation;
pub mod foundation;
pub mod uikit;

/// Container for state of various child modules
#[derive(Default)]
pub struct State {
    foundation: foundation::State,
}
