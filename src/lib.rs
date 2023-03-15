/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! touchHLE is a high-level emulator (HLE) for iPhone OS applications.
//!
//! In various places, the terms "guest" and "host" are used to distinguish
//! between the emulated application (the "guest") and the emulator itself (the
//! "host"), and more generally, their different environments.
//! For example:
//! - The guest is a 32-bit application, so a "guest pointer" is 32 bits.
//! - The host is a 64-bit application, so a "host pointer" is 64 bits.
//! - The guest can only directly access "guest memory".
//! - The host can access both "guest memory" and "host memory".
//! - A "guest function" is emulated Arm code, usually from the app binary.
//! - A "host function" is a Rust function that is part of this emulator.

// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]

#[macro_use]
mod logg;
mod abi;
mod audio;
mod bundle;
mod cpu;
mod dyld;
mod environment;
mod font;
mod frameworks;
mod fs;
mod gdb;
mod image;
mod libc;
mod licenses;
mod mach_o;
mod mem;
mod objc;
mod options;
mod stack;
mod window;

// These are very frequently used and used to be in this module, so they are
// re-exported to avoid having to update lots of imports.
// Unlike its siblings, this module should be considered private and only used
// via re-exports.
use environment::{Environment, ThreadID};

use std::path::PathBuf;
use std::ffi::{c_int, c_char};

use log::LevelFilter;
use android_logger::{Config, LogId};

#[cfg(target_os = "android")]
#[no_mangle]
pub extern fn SDL_main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    sdl2::log::log("HERE");

    android_logger::init_once(Config::default()
        .with_max_level(LevelFilter::Trace)
        .with_log_buffer(LogId::System));

    match _main() {
        Ok(_) => sdl2::log::log("touchHLE finished"),
        Err(e) => sdl2::log::log(&format!("touchHLE errored: {e:?}").to_string()),
    }
    return 0;
}

/// Current version. See `build.rs` for how this is generated.
const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

const USAGE: &str = "\
Usage:
    touchHLE path/to/some.app

General options:
    --help
        Display this help text.

    --copyright
        Display copyright, authorship and license information.

    --info
        Print basic information about the app bundle without running the app.
";

fn _main() -> Result<(), String> {
    println!("touchHLE {} — https://touchhle.org/", VERSION);
    println!();

    let mut args = std::env::args();
    //let _ = args.next().unwrap(); // skip argv[0]

    let mut bundle_path: Option<PathBuf> = None;
    let mut just_info = false;
    let mut option_args = Vec::new();

    for arg in args {
        if arg == "--help" {
            println!("{}", USAGE);
            println!("{}", options::DOCUMENTATION);
            return Ok(());
        } else if arg == "--copyright" {
            licenses::print();
            return Ok(());
        } else if arg == "--info" {
            just_info = true;
        // Parse an option but discard the value, to test whether it's valid.
        // We don't want to apply it immediately, because then options loaded
        // from a file would take precedence over options from the command line.
        } else if options::Options::default().parse_argument(&arg)? {
            option_args.push(arg);
        } else if bundle_path.is_none() {
            bundle_path = Some(PathBuf::from(arg));
        } else {
            eprintln!("{}", USAGE);
            eprintln!("{}", options::DOCUMENTATION);
            return Err(format!("Unexpected argument: {:?}", arg));
        }
    }

    bundle_path = Some(PathBuf::from("/data/data/org.libsdl.app/files/Super Monkey Ball  v1.02 .ipa"));

    let Some(bundle_path) = bundle_path else {
        eprintln!("{}", USAGE);
        eprintln!("{}", options::DOCUMENTATION);
        return Err("Path to bundle must be specified".to_string());
    };

    // When PowerShell does tab-completion on a directory, for some reason it
    // expands it to `'..\My Bundle.app\'` and that trailing \ seems to
    // get interpreted as escaping a double quotation mark?
    #[cfg(windows)]
    if let Some(fixed) = bundle_path.to_str().and_then(|s| s.strip_suffix('"')) {
        logg!("Warning: The bundle path has a trailing quotation mark! This often happens accidentally on Windows when tab-completing, because '\\\"' gets interpreted by Rust in the wrong way. Did you meant to write {:?}?", fixed);
    }

    let bundle_data = fs::BundleData::open_any(&bundle_path)
        .map_err(|e| format!("Could not open app bundle: {e}"))?;
    let (bundle, fs) = match bundle::Bundle::new_bundle_and_fs_from_host_path(bundle_data) {
        Ok(bundle) => bundle,
        Err(err) => {
            return Err(format!("Application bundle error: {err}. Check that the path is to an .app directory or an .ipa file."));
        }
    };

    let app_id = bundle.bundle_identifier();
    let minimum_os_version = bundle.minimum_os_version();

    println!("App bundle info:");
    println!("- Display name: {}", bundle.display_name());
    println!("- Version: {}", bundle.bundle_version());
    println!("- Identifier: {}", app_id);
    if let Some(canonical_name) = bundle.canonical_bundle_name() {
        println!("- Internal name (canonical): {}.app", canonical_name);
    } else {
        println!("- Internal name (from FS): {}.app", bundle.bundle_name());
    }
    println!(
        "- Minimum OS version: {}",
        minimum_os_version.unwrap_or("(not specified)")
    );
    println!();

    if let Some(version) = minimum_os_version {
        let (major, _minor_etc) = version.split_once('.').unwrap();
        let major: u32 = major.parse().unwrap();
        if major > 2 {
            eprintln!("Warning: app requires OS version {}. Only iPhone OS 2 apps are currently supported.", version);
        }
    }

    if just_info {
        return Ok(());
    }

    let mut options = options::Options::default();

    // Apply options from files
    for filename in [options::DEFAULTS_FILENAME, options::USER_FILENAME] {
        match options::get_options_from_file(filename, app_id) {
            Ok(Some(options_string)) => {
                println!(
                    "Using options from {} for this app: {}",
                    filename, options_string
                );
                for option_arg in options_string.split_ascii_whitespace() {
                    match options.parse_argument(option_arg) {
                        Ok(true) => (),
                        Ok(false) => return Err(format!("Unknown option {:?}", option_arg)),
                        Err(err) => {
                            return Err(format!("Invalid option {:?}: {}", option_arg, err))
                        }
                    }
                }
            }
            Ok(None) => {
                println!("No options found for this app in {}", filename);
            }
            Err(e) => {
                eprintln!("Warning: {}", e);
            }
        }
    }
    println!();

    // Apply command-line options
    for option_arg in option_args {
        let parse_result = options.parse_argument(&option_arg);
        assert!(parse_result == Ok(true));
    }

    let mut env = Environment::new(bundle, fs, options)?;
    env.run();
    Ok(())
}
