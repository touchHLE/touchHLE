//! Separate module just for the function lists, since this will probably be a
//! very long and frequently-updated list.

use crate::frameworks::{
    audio_toolbox, core_foundation, core_graphics, foundation, openal, opengles, uikit,
};
use crate::libc;

/// All the lists of functions that the linker should search through.
pub const FUNCTION_LISTS: &[super::FunctionExports] = &[
    libc::ctype::FUNCTIONS,
    libc::cxxabi::FUNCTIONS,
    libc::mach_time::FUNCTIONS,
    libc::math::FUNCTIONS,
    libc::pthread::key::FUNCTIONS,
    libc::pthread::mutex::FUNCTIONS,
    libc::pthread::once::FUNCTIONS,
    libc::stdio::FUNCTIONS,
    libc::stdio::printf::FUNCTIONS,
    libc::stdlib::FUNCTIONS,
    libc::string::FUNCTIONS,
    libc::time::FUNCTIONS,
    crate::objc::FUNCTIONS,
    audio_toolbox::audio_file::FUNCTIONS,
    audio_toolbox::audio_queue::FUNCTIONS,
    core_foundation::cf_bundle::FUNCTIONS,
    core_foundation::cf_run_loop::FUNCTIONS,
    core_foundation::cf_type::FUNCTIONS,
    core_foundation::cf_url::FUNCTIONS,
    core_graphics::cg_color_space::FUNCTIONS,
    foundation::ns_file_manager::FUNCTIONS,
    openal::FUNCTIONS,
    opengles::FUNCTIONS,
    uikit::FUNCTIONS,
];
