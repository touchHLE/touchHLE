/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! touchHLE is a high-level emulator (HLE) for iPhone OS applications.

#![allow(non_snake_case)]
#![allow(rustdoc::private_intra_doc_links)]

#[macro_use]
mod log;
mod abi;
mod audio;
mod bundle;
mod cpu;
mod debug;
mod dyld;
mod environment;
mod font;
mod frameworks;
mod fs;
mod gdb;
mod gles;
mod image;

// Libc modules section. 
// Only include modules that have corresponding .rs files in src/libc/
pub mod libc {
    pub mod clocale;
    pub mod ctype;
    pub mod errno;
    pub mod posix_io;
    pub mod sqlite; // Our new stub module
    pub mod stdio;
    pub mod stdlib;
    pub mod string;
    pub mod wchar;
    
    // Core re-exports needed by the emulator
    pub use crate::dyld::DYLIB;
    pub use crate::environment::State;
}

mod licenses;
mod mach_o;
mod matrix;
mod mem;
mod objc;
mod options;
mod paths;
mod stack;
mod window;

use environment::{Environment, MutexId, MutexType, ThreadId, PTHREAD_MUTEX_DEFAULT};
use std::path::PathBuf;

pub use touchHLE_version::*;

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn SDL_main(
    _argc: std::ffi::c_int,
    _argv: *const *const std::ffi::c_char,
) -> std::ffi::c_int {
    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s
        } else {
            "(non-string payload)"
        };
        if let Some(location) = info.location() {
            echo!("Panic at {}: {}", location, payload);
        } else {
            echo!("Panic: {}", payload);
        }
    }));

    match main([String::new()].into_iter()) {
        Ok(_) => echo!("touchHLE finished"),
        Err(e) => echo!("touchHLE errored: {e:?}"),
    }
    0
}

pub fn main<T: Iterator<Item = String>>(mut args: T) -> Result<(), String> {
    echo!(
        "touchHLE {}{}{} — https://touchhle.org/",
        branding(),
        if branding().is_empty() { "" } else { " " },
        VERSION,
    );
    echo!();

    let _ = args.next().unwrap(); 

    let mut bundle_path: Option<PathBuf> = None;
    let mut options = options::Options::default();
    let mut app_args = None::<Vec<String>>;

    for arg in args {
        if let Some(ref mut app_args) = app_args {
            app_args.push(arg);
        } else if arg == "--args" {
            app_args = Some(Vec::new());
        } else if options.parse_argument(&arg)? {
            // Option handled
        } else if bundle_path.is_none() {
            bundle_path = Some(PathBuf::from(arg));
        }
    }

    let bundle_path = if let Some(bundle_path) = bundle_path {
        bundle_path
    } else {
        let (path, _) = environment::app_picker::app_picker(options.clone())?;
        path
    };

    let bundle_data = fs::BundleData::open_any(&bundle_path)
        .map_err(|e| format!("Could not open app bundle: {e}"))?;
    let (bundle, fs) = bundle::Bundle::new_bundle_and_fs_from_host_path(
        bundle_data,
        false,
    ).map_err(|e| e.to_string())?;

    let env = Environment::new(bundle, fs, options, app_args.unwrap_or_default())
        .map_err(|e| e.to_string())?;
    
    env.run();
    Ok(())
}
