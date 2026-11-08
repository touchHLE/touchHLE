/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Logging and terminal output macros.

use std::fs::File;
use std::sync::LazyLock;

/// Get a handle to the log file. This is only for use by logging macros!
///
/// In headless/CLI mode on macOS, logs are only written to a file if
/// `PEAR_LEGACY_LOG_FILE` is explicitly set, avoiding unnecessary disk writes
/// in the working directory when PearSilicon already captures stdout/stderr.
pub fn get_log_file() -> Option<&'static File> {
    static LOG_FILE: LazyLock<Option<File>> = LazyLock::new(|| {
        if let Ok(path) = std::env::var("PEAR_LEGACY_LOG_FILE") {
            if !path.is_empty() {
                return File::create(path).ok();
            }
        }
        None
    });

    LOG_FILE.as_ref()
}

/// Prints a log message unconditionally. Use this for errors or warnings.
///
/// The message is prefixed with the module path, so it is clear where it comes
/// from.
macro_rules! log {
    ($($arg:tt)+) => {
        echo!("{}: {}", module_path!().replacen("touchHLE", "PearLegacyEngine", 1), format_args!($($arg)+));
    }
}

/// Same as [log], but silently fails on panic instead of
/// panicking.
macro_rules! log_no_panic {
    ($($arg:tt)+) => {
        echo_no_panic!("{}: {}", module_path!().replacen("touchHLE", "PearLegacyEngine", 1), format_args!($($arg)+));
    }
}

/// Like [log], but prints the message only if debugging is enabled for the
/// module where it is used. This can be used for verbose things only needed
/// when debugging.
macro_rules! log_dbg {
    ($($arg:tt)+) => {
        if $crate::log::debug_module_enabled(module_path!()) {
            log!($($arg)*);
        }
    }
}

/// Like [log], but messages only log once and cannot have formatting.
/// To be used for log messages that are known to spam the log file (like those
/// logged every frame).
macro_rules! log_once {
    ($msg:literal) => {{
        static LOG_ONCE: std::sync::Once = std::sync::Once::new();
        LOG_ONCE.call_once(|| {
            log!("{} [this log will only be shown once]", $msg);
        });
    }};
}

/// Print a message (with implicit newline). This should be used for all
/// touchHLE output that isn't coming from the app itself.
///
/// Prefer use [log] or [log_dbg] for errors and warnings during emulation.
macro_rules! echo {
    ($($arg:tt)+) => {
        {
            let formatted_str = format!($($arg)+);

            #[cfg(target_os = "android")]
            {
                sdl2::log::log(&formatted_str);
            }
            #[cfg(not(target_os = "android"))]
            eprintln!("{}", formatted_str);

            use std::io::Write;
            if let Some(mut log_file) = $crate::log::get_log_file() {
                let _ = log_file.write_all(formatted_str.as_bytes());
                let _ = log_file.write_all(b"\n");
            }
        }
    };
    () => {
        {
            #[cfg(target_os = "android")]
            {
                sdl2::log::log("");
            }
            #[cfg(not(target_os = "android"))]
            eprintln!("");

            use std::io::Write;
            if let Some(mut log_file) = $crate::log::get_log_file() {
                let _ = log_file.write_all(b"\n");
            }
        }
    }
}

/// Same as [echo], but silently fails on panic instead of
/// panicking.
macro_rules! echo_no_panic {
    ($($arg:tt)*) => {
        {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                echo!($($arg)*);
            }));
        }
    }
}

/// Extra modules to enable [log_dbg] for, e.g.
/// `PEAR_DEBUG_MODULES=PearLegacyEngine::mem,PearLegacyEngine::frameworks::uikit`.
///
/// Entries are matched against `module_path!()`, which still uses the crate's
/// historical name internally, so both spellings are accepted.
static ENABLED_MODULES: std::sync::LazyLock<Vec<String>> = std::sync::LazyLock::new(|| {
    std::env::var("PEAR_DEBUG_MODULES")
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(|module| module.trim().replacen("PearLegacyEngine", "touchHLE", 1))
                .filter(|module| !module.is_empty())
                .collect()
        })
        .unwrap_or_default()
});

pub fn debug_module_enabled(module: &str) -> bool {
    ENABLED_MODULES.iter().any(|enabled| enabled == module)
}
