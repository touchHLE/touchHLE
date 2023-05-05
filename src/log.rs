/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

/// Print a message (with implicit newline). This should be used for all
/// touchHLE output that isn't coming from the app itself.
///
/// Prefer use [log] or [log_dbg] for errors and warnings during emulation.
macro_rules! echo {
    ($($arg:tt)+) => {
        {
            #[cfg(target_os = "android")]
            sdl2::log::log(&format!($($arg)+));
            #[cfg(not(target_os = "android"))]
            eprintln!($($arg)+);
        }
    };
    () => {
        {
            #[cfg(target_os = "android")]
            sdl2::log::log("");
            #[cfg(not(target_os = "android"))]
            eprintln!("");
        }
    }
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

/// Put modules to enable [log_dbg] for here, e.g. "touchHLE::mem" to see when
/// memory is allocated and freed.
pub const ENABLED_MODULES: &[&str] = &[];
