/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Paths for host files used by touchHLE: settings, fonts, etc.
//!
//! There are three categories of files:
//!
//! * Resources bundled with touchHLE that neither touchHLE nor the user should
//!   modify: [DYLIBS_DIR], [FONTS_DIR], [DEFAULT_OPTIONS_FILE].
//! * Files the user is expected to modify, but not touchHLE: [APPS_DIR],
//!   [USER_OPTIONS_FILE].
//! * Files that touchHLE will create and modify, and the user may modify if
//!   they want to: [SANDBOX_DIR].
//!
//! See also [crate::fs], which provides a virtual filesystem for the guest app
//! and defines path types.

use std::path::Path;

/// Name of the directory containing ARMv6 dynamic libraries bundled with
/// touchHLE.
pub const DYLIBS_DIR: &str = "touchHLE_dylibs";

/// Name of the directory containing fonts bundled with touchHLE.
pub const FONTS_DIR: &str = "touchHLE_fonts";

/// Name of the file containing touchHLE's default options for various apps.
pub const DEFAULT_OPTIONS_FILE: &str = "touchHLE_default_options.txt";

/// Name of the directory where the user can put apps if they want them to
/// appear in the app picker.
pub const APPS_DIR: &str = "touchHLE_apps";

/// Name of the file intended for the user's own options.
pub const USER_OPTIONS_FILE: &str = "touchHLE_options.txt";

/// Name of the directory where touchHLE will store sandboxed app data, e.g.
/// the `Documents` directory.
pub const SANDBOX_DIR: &str = "touchHLE_sandbox";

/// Get a platform-specific base path needed for accessing touchHLE's bundled
/// resources and other files. This is empty on platforms other than Android.
pub fn base_path() -> &'static Path {
    #[cfg(target_os = "android")]
    unsafe {
        // This is an exception to the rule that SDL2 should only be used
        // directly from src/window.rs. This is just too distant from windowing
        // to belong there.

        // Android storage has evolved in a quite messy fashion. Both "internal
        // storage" and "external storage" (aka the "SD card") are likely to be
        // internal on a modern device, as absurd as that might sound. SDL2 has
        // APIs to get paths for both. We use the "external storage" because
        // it's more likely to be user-accessible.
        extern "C" {
            fn SDL_AndroidGetExternalStoragePath() -> *const std::ffi::c_char;
        }
        let path = SDL_AndroidGetExternalStoragePath();
        if path.is_null() {
            log!("Couldn't get Android external storage path!");
            panic!();
        }
        Path::new(std::ffi::CStr::from_ptr(path).to_str().unwrap())
    }
    #[cfg(not(target_os = "android"))]
    Path::new("")
}
