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

/// Get a platform-specific path prefix needed for accessing touchHLE's bundled
/// resources and other files. This is empty on platforms other than Android.
pub fn files_prefix() -> &'static str {
    // FIXME: use SDL_AndroidGetInternalStoragePath
    if cfg!(target_os = "android") {
        "/data/data/org.touchhle.android/files/"
    } else {
        ""
    }
}
