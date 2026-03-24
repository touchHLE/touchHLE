/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![allow(non_snake_case)]
#![allow(rustdoc::private_intra_doc_links)]

#[macro_use]
pub mod log;

pub mod abi;
pub mod audio;
pub mod bundle;
pub mod cpu;
pub mod debug;
pub mod dyld;
pub mod environment;
pub mod font;
pub mod frameworks;
pub mod fs;
pub mod gdb;
pub mod gles;
pub mod image;

pub mod libc {
    pub mod clocale;
    pub mod ctype;
    pub mod dirent;
    pub mod errno;
    pub mod keymgr;
    pub mod mach;
    pub mod mmap;
    pub mod netdb;
    pub mod posix_io;
    pub mod pthread;
    pub mod semaphore;
    pub mod sqlite;
    pub mod stdio;
    pub mod stdlib;
    pub mod string;
    pub mod sys;
    pub mod time;
    pub mod unistd;
    pub mod wchar;
    pub mod generic_char;

    pub use self::posix_io::State;
    pub const DYLIB: crate::dyld::Library = crate::dyld::Library::Native("libc");
}

pub mod licenses;
pub mod mach_o;
pub mod matrix;
pub mod mem;
pub mod objc;
pub mod options;
pub mod paths;
pub mod stack;
pub mod window;

pub const PTHREAD_MUTEX_DEFAULT: i32 = 0;

pub use touchHLE_version::{branding, VERSION};
pub use environment::{Environment, MutexId, MutexType, ThreadId};

use std::path::PathBuf;

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
