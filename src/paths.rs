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
//!   modify: [DYLIBS_DIR], [FONTS_DIR], [DEFAULT_OPTIONS_FILE]. Depending on
//!   the platform these may or may not be ordinary files, and must be accessed
//!   through [ResourceFile].
//! * Files the user is expected to modify, but not touchHLE: [APPS_DIR],
//!   [USER_OPTIONS_FILE], [WALLPAPER_FILES]. These are ordinary files and are
//!   found in [user_data_base_path].
//! * Files that touchHLE will create and modify, and the user may modify if
//!   they want to: [SANDBOX_DIR]. These are ordinary files and are found in
//!   [user_data_base_path].
//!
//! See also [crate::fs], which provides a virtual filesystem for the guest app
//! and defines path types.

use std::io::{Read, Seek};
use std::path::Path;

/// Name of the directory containing ARMv6 dynamic libraries bundled with
/// touchHLE.
pub const DYLIBS_DIR: &str = "touchHLE_dylibs";

/// Name of the directory containing fonts bundled with touchHLE.
pub const FONTS_DIR: &str = "touchHLE_fonts";

/// Name of the file containing touchHLE's default options for various apps.
pub const DEFAULT_OPTIONS_FILE: &str = "touchHLE_default_options.txt";

/// Abstraction over a platform-specific type for accessing a resource bundled
/// with touchHLE.
pub struct ResourceFile {
    #[cfg(target_os = "android")]
    file: sdl2::rwops::RWops<'static>,
    #[cfg(not(target_os = "android"))]
    file: std::fs::File,
}
impl ResourceFile {
    pub fn open(path: &str) -> Result<Self, String> {
        Ok(Self {
            // On Android, these resources are included as "assets" within the
            // APK. We access them via SDL2's wrapper of Android's assets API.
            #[cfg(target_os = "android")]
            file: sdl2::rwops::RWops::from_file(path, "r")?,
            // On other OSes, resources are ordinary files in the current
            // directory.
            #[cfg(not(target_os = "android"))]
            file: std::fs::File::open(path).map_err(|e| e.to_string())?,
        })
    }
    pub fn get(&mut self) -> &mut (impl Read + Seek) {
        &mut self.file
    }
}
impl std::fmt::Debug for ResourceFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "ResourceFile")
    }
}

/// Whether various resources are in user-accessible files. If they aren't,
/// touchHLE has to be able to display their license terms.
pub const RESOURCES_ARE_EXTERNAL_FILES: bool = cfg!(not(target_os = "android"));

/// Name of the directory where the user can put apps if they want them to
/// appear in the app picker.
pub const APPS_DIR: &str = "touchHLE_apps";

/// Name of the file intended for the user's own options.
pub const USER_OPTIONS_FILE: &str = "touchHLE_options.txt";

/// Names of files the user can put a wallpaper image (for the app picker) in.
#[allow(unused)]
pub const WALLPAPER_FILES: &[&str] = &[
    "touchHLE_wallpaper.png",
    "touchHLE_wallpaper.jpg",
    "touchHLE_wallpaper.jpeg",
];

/// Name of the directory where touchHLE will store sandboxed app data, e.g.
/// the `Documents` directory.
pub const SANDBOX_DIR: &str = "touchHLE_sandbox";

/// Get a platform-specific base path needed for accessing touchHLE's
/// user-modifiable files. This is empty on platforms other than Android.
pub fn user_data_base_path() -> &'static Path {
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

/// Get a URI that can be used to open a file manager or similar for the path
/// that [user_data_base_path] represents.
pub fn url_for_opening_user_data_dir() -> Result<String, String> {
    if std::env::consts::OS == "android" {
        // See DocumentsProvider.kt and AndroidManifest.xml
        Ok("content://org.touchhle.android.provider/root/root".to_string())
    } else {
        let path = user_data_base_path()
            .join(".")
            .canonicalize()
            .map_err(|e| format!("Can't canonicalize path to user data directory: {}", e))?;
        let path = path
            .to_str()
            .ok_or_else(|| "User data directory path is not UTF-8".to_string())?;
        // std::fs::canonicalize() on Windows uses the extended-length path
        // syntax, but Windows Explorer doesn't understand it.
        let path = if std::env::consts::OS == "windows" {
            path.strip_prefix("\\\\?\\").unwrap_or(path)
        } else {
            path
        };
        Ok(format!("file://{}", path))
    }
}

/// Only meaningful on Android: create the user data directory if it doesn't
/// exist, and populate it with templates or README files. (On other platforms
/// these are simply bundled with touchHLE in a ZIP file.)
pub fn prepopulate_user_data_dir() {
    if std::env::consts::OS != "android" {
        return;
    }

    let apps_dir = user_data_base_path().join(APPS_DIR);
    if !apps_dir.is_dir() {
        match std::fs::create_dir(&apps_dir) {
            Ok(()) => {
                log!("Created: {}", apps_dir.display());
            }
            Err(e) => {
                log!("Warning: Couldn't create {}: {}", apps_dir.display(), e);
            }
        }
    }

    fn create_file(path: &Path, content: &str) {
        match std::fs::write(path, content) {
            Ok(()) => {
                log!("Created: {}", path.display());
            }
            Err(e) => {
                log!("Warning: Couldn't create {}: {}", path.display(), e);
            }
        }
    }

    let apps_dir_readme = apps_dir.join("README.txt");
    if !apps_dir_readme.is_file() {
        let content = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/touchHLE_apps/README.txt"
        ));
        create_file(&apps_dir_readme, content);
    }

    let user_options = user_data_base_path().join(USER_OPTIONS_FILE);
    if !user_options.is_file() {
        let content = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/touchHLE_options.txt"));
        create_file(&user_options, content);
    }

    let options_help = user_data_base_path().join("OPTIONS_HELP.txt");
    if !options_help.is_file() {
        create_file(&options_help, crate::options::OPTIONS_HELP);
    }
}
