/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Logging and terminal output macros.

/// Accessing log output on Android is more difficult than on other platforms;
/// logcat requires a separate device. As an alternative, let's write to a file
/// too.
#[cfg(target_os = "android")]
pub static mut LOG_FILE: Option<std::fs::File> = None;

/// Set up log file. Only call this once, right at the start of the program!
#[cfg(target_os = "android")]
pub unsafe fn setup_log_file() {
    {
        LOG_FILE = Some(
            std::fs::File::create(crate::paths::user_data_base_path().join("log.txt")).unwrap(),
        );
    }
}

/// Only for internal use by the logging macros.
#[cfg(target_os = "android")]
pub fn get_log_file() -> &'static std::fs::File {
    unsafe { LOG_FILE.as_ref().unwrap() }
}

/// Prints a log message unconditionally. Use this for errors or warnings.
///
/// The message is prefixed with the module path, so it is clear where it comes
/// from.
macro_rules! log {
    ($($arg:tt)+) => {
        echo!("{}: {}", module_path!(), format_args!($($arg)+));
    }
}

/// Like [log], but prints the message only if debugging is enabled for the
/// module where it is used. This can be used for verbose things only needed
/// when debugging.
macro_rules! log_dbg {
    ($($arg:tt)+) => {
        if $crate::log::ENABLED_MODULES.contains(&module_path!()) {
            log!($($arg)*);
        }
    }
}

/// Print a message (with implicit newline). This should be used for all
/// touchHLE output that isn't coming from the app itself.
///
/// Prefer use [log] or [log_dbg] for errors and warnings during emulation.
macro_rules! echo {
    ($($arg:tt)+) => {
        {
            #[cfg(target_os = "android")]
            {
                let formatted_str = format!($($arg)+);
                sdl2::log::log(&formatted_str);
                use std::io::Write;
                let mut log_file = $crate::log::get_log_file();
                let _ = log_file.write_all(formatted_str.as_bytes());
                let _ = log_file.write_all(b"\n");
            }
            #[cfg(not(target_os = "android"))]
            eprintln!($($arg)+);
        }
    };
    () => {
        {
            #[cfg(target_os = "android")]
            {
                sdl2::log::log("");
                use std::io::Write;
                let _ = $crate::log::get_log_file().write_all(b"\n");
            }
            #[cfg(not(target_os = "android"))]
            eprintln!("");
        }
    }
}

/// Put modules to enable [log_dbg] for here, e.g. "touchHLE::mem" to see when
/// memory is allocated and freed.
pub const ENABLED_MODULES: &[&str] = &[];
