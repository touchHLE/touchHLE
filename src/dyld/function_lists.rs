/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Separate module just for the function lists, since this will probably be a
//! very long and frequently-updated list.

use crate::frameworks::{
    audio_toolbox, core_foundation, core_graphics, dnssd, foundation, openal, opengles, uikit,
};
use crate::libc;

/// All the lists of functions that the linker should search through.
pub const FUNCTION_LISTS: &[super::FunctionExports] = &[
    libc::clocale::FUNCTIONS,
    libc::ctype::FUNCTIONS,
    libc::cxxabi::FUNCTIONS,
    libc::dirent::FUNCTIONS,
    libc::dlfcn::FUNCTIONS,
    libc::errno::FUNCTIONS,
    libc::ifaddrs::FUNCTIONS,
    libc::keymgr::FUNCTIONS,
    libc::mach_thread_info::FUNCTIONS,
    libc::mach_time::FUNCTIONS,
    libc::math::FUNCTIONS,
    libc::mmap::FUNCTIONS,
    libc::net::if_::FUNCTIONS,
    libc::posix_io::FUNCTIONS,
    libc::posix_io::stat::FUNCTIONS,
    libc::pthread::cond::FUNCTIONS,
    libc::pthread::key::FUNCTIONS,
    libc::pthread::mutex::FUNCTIONS,
    libc::pthread::once::FUNCTIONS,
    libc::pthread::thread::FUNCTIONS,
    libc::semaphore::FUNCTIONS,
    libc::setjmp::FUNCTIONS,
    libc::signal::FUNCTIONS,
    libc::stdio::FUNCTIONS,
    libc::stdio::printf::FUNCTIONS,
    libc::stdlib::FUNCTIONS,
    libc::stdlib::qsort::FUNCTIONS,
    libc::string::FUNCTIONS,
    libc::sys::utsname::FUNCTIONS,
    libc::sysctl::FUNCTIONS,
    libc::time::FUNCTIONS,
    libc::unistd::FUNCTIONS,
    libc::wchar::FUNCTIONS,
    crate::objc::FUNCTIONS,
    audio_toolbox::audio_file::FUNCTIONS,
    audio_toolbox::audio_queue::FUNCTIONS,
    audio_toolbox::audio_services::FUNCTIONS,
    audio_toolbox::audio_session::FUNCTIONS,
    core_foundation::cf_array::FUNCTIONS,
    core_foundation::cf_bundle::FUNCTIONS,
    core_foundation::cf_data::FUNCTIONS,
    core_foundation::cf_run_loop::FUNCTIONS,
    core_foundation::cf_run_loop_timer::FUNCTIONS,
    core_foundation::cf_string::FUNCTIONS,
    core_foundation::cf_type::FUNCTIONS,
    core_foundation::cf_url::FUNCTIONS,
    core_foundation::time::FUNCTIONS,
    core_graphics::cg_affine_transform::FUNCTIONS,
    core_graphics::cg_bitmap_context::FUNCTIONS,
    core_graphics::cg_color_space::FUNCTIONS,
    core_graphics::cg_context::FUNCTIONS,
    core_graphics::cg_data_provider::FUNCTIONS,
    core_graphics::cg_geometry::FUNCTIONS,
    core_graphics::cg_image::FUNCTIONS,
    dnssd::FUNCTIONS,
    foundation::ns_file_manager::FUNCTIONS,
    foundation::ns_log::FUNCTIONS,
    foundation::ns_objc_runtime::FUNCTIONS,
    openal::FUNCTIONS,
    opengles::FUNCTIONS,
    uikit::ui_application::FUNCTIONS,
    uikit::ui_geometry::FUNCTIONS,
    uikit::ui_graphics::FUNCTIONS,
];
