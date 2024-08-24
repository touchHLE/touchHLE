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
// The documentation for this crate is intended to include private items.
// rustdoc complains about some public macros that link to private items, but
// we're forced to make those macros public by the weird macro scoping rules,
// so this warning is unhelpful.
#![allow(rustdoc::private_intra_doc_links)]

#[macro_use]
mod log;
mod abi;
mod app_picker;
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
mod libc;
mod licenses;
mod mach_o;
mod matrix;
mod mem;
mod objc;
mod options;
mod paths;
mod stack;
mod window;

// Environment is used very frequently used and used to be in this module, so
// it is re-exported to avoid having to update lots of imports. The other things
// probably shouldn't be, but they need a new home (TODO).
// Unlike its siblings, this module should be considered private and only used
// via re-exports.
use environment::{Environment, MutexId, MutexType, ThreadId, PTHREAD_MUTEX_DEFAULT};

use std::path::PathBuf;

/// Current version. See `build.rs` for how this is generated.
const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));
// Environment variables set by GitHub Actions
const GITHUB_REPOSITORY: Option<&str> = option_env!("GITHUB_REPOSITORY");
const GITHUB_SERVER_URL: Option<&str> = option_env!("GITHUB_SERVER_URL");
const GITHUB_RUN_ID: Option<&str> = option_env!("GITHUB_RUN_ID");
const GITHUB_REF_NAME: Option<&str> = option_env!("GITHUB_REF_NAME");

fn branding() -> &'static str {
    if GITHUB_RUN_ID.is_none() {
        return "";
    }
    if (GITHUB_REPOSITORY, GITHUB_REF_NAME) == (Some("touchHLE/touchHLE"), Some("trunk")) {
        "PREVIEW"
    } else {
        "UNOFFICIAL"
    }
}

/// This is the true entry point on Android (SDLActivity calls it after
/// initialization). On other platforms the true entry point is in src/bin.rs.
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn SDL_main(
    _argc: std::ffi::c_int,
    _argv: *const *const std::ffi::c_char,
) -> std::ffi::c_int {
    unsafe { log::setup_log_file() };

    // Rust's default panic handler prints to stderr, but on Android that just
    // gets discarded, so we set a custom hook to make debugging easier.
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

    // Empty args: brings up app picker.
    match main([String::new()].into_iter()) {
        Ok(_) => echo!("touchHLE finished"),
        Err(e) => echo!("touchHLE errored: {e:?}"),
    }
    0
}

const USAGE: &str = "\
Usage:
    touchHLE path/to/some.app

If no app path or special option is specified, a GUI app picker is displayed.

Special options:
    --help
        Display this help text.

    --copyright
        Display copyright, authorship and license information.

    --info
        Print basic information about the app bundle without running the app.
";

pub fn main<T: Iterator<Item = String>>(mut args: T) -> Result<(), String> {
    echo!(
        "touchHLE {}{}{} — https://touchhle.org/",
        branding(),
        if branding().is_empty() { "" } else { " " },
        VERSION,
    );
    if GITHUB_RUN_ID.is_some() {
        echo!(
            "Built from branch {:?} of {:?} by GitHub Actions workflow run {}/{}/actions/runs/{}.",
            GITHUB_REF_NAME.unwrap(),
            GITHUB_REPOSITORY.unwrap(),
            GITHUB_SERVER_URL.unwrap(),
            GITHUB_REPOSITORY.unwrap(),
            GITHUB_RUN_ID.unwrap()
        );
    }
    echo!();

    {
        let base_path = paths::user_data_base_path().to_str().unwrap();
        if !base_path.is_empty() {
            log!("Base path for touchHLE files: {}", base_path);
        }
        paths::prepopulate_user_data_dir();
    }

    let _ = args.next().unwrap(); // skip argv[0]

    let mut bundle_path: Option<PathBuf> = None;
    let mut just_info = false;
    let mut option_args = Vec::new();

    for arg in args {
        if arg == "--help" {
            echo!("{}", USAGE);
            echo!("{}", options::OPTIONS_HELP);
            return Ok(());
        } else if arg == "--copyright" {
            echo!("{}", licenses::get_text());
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
            echo!("{}", USAGE);
            echo!("{}", options::OPTIONS_HELP);
            return Err(format!("Unexpected argument: {:?}", arg));
        }
    }

    let (bundle_path, env_for_salvage) = if let Some(bundle_path) = bundle_path {
        (bundle_path, None)
    } else {
        let mut options = options::Options::default();
        // Apply command-line options only (no app-specific options apply)
        for option_arg in &option_args {
            let parse_result = options.parse_argument(option_arg);
            assert!(parse_result == Ok(true));
        }
        if options.headless {
            return Err(
                "No app specified. Use the --help flag to see command-line usage.".to_string(),
            );
        }
        echo!(
            "No app specified, opening app picker. Use the --help flag to see command-line usage."
        );
        let (bundle_path, env_for_salvage) = app_picker::app_picker(options, &mut option_args)?;
        (bundle_path, Some(env_for_salvage))
    };

    // When PowerShell does tab-completion on a directory, for some reason it
    // expands it to `'..\My Bundle.app\'` and that trailing \ seems to
    // get interpreted as escaping a double quotation mark?
    #[cfg(windows)]
    if let Some(fixed) = bundle_path.to_str().and_then(|s| s.strip_suffix('"')) {
        log!("Warning: The bundle path has a trailing quotation mark! This often happens accidentally on Windows when tab-completing, because '\\\"' gets interpreted by Rust in the wrong way. Did you meant to write {:?}?", fixed);
    }

    let bundle_data = fs::BundleData::open_any(&bundle_path)
        .map_err(|e| format!("Could not open app bundle: {e}"))?;
    let (bundle, fs) = match bundle::Bundle::new_bundle_and_fs_from_host_path(
        bundle_data,
        /* read_only_mode: */ false,
    ) {
        Ok(bundle) => bundle,
        Err(err) => {
            return Err(format!("Application bundle error: {err}. Check that the path is to an .app directory or an .ipa file."));
        }
    };

    let app_id = bundle.bundle_identifier();
    let minimum_os_version = bundle.minimum_os_version();

    echo!("App bundle info:");
    echo!("- Display name: {}", bundle.display_name());
    echo!("- Version: {}", bundle.bundle_version());
    echo!("- Identifier: {}", app_id);
    if let Some(canonical_name) = bundle.canonical_bundle_name() {
        echo!("- Internal name (canonical): {}.app", canonical_name);
    } else {
        echo!("- Internal name (from FS): {}.app", bundle.bundle_name());
    }
    echo!(
        "- Minimum OS version: {}",
        minimum_os_version.unwrap_or("(not specified)")
    );
    echo!();

    if let Some(version) = minimum_os_version {
        let (major, minor_etc) = version.split_once('.').unwrap();
        let minor = minor_etc
            .split_once('.')
            .map_or(minor_etc, |(minor, _etc)| minor);
        let major: u32 = major.parse().unwrap();
        let minor: u32 = minor.parse().unwrap();
        if major > 3 || (major == 3 && minor > 0) {
            echo!("Warning: app requires OS version {}. Only iPhone OS 2.x and iPhone OS 3.0 apps are currently supported.", version);
        }
    }

    if just_info {
        return Ok(());
    }

    let mut options = options::Options::default();

    // Apply options from files
    fn apply_options<F: std::io::Read, P: std::fmt::Display>(
        file: F,
        path: P,
        options: &mut options::Options,
        app_id: &str,
    ) -> Result<(), String> {
        match options::get_options_from_file(file, app_id) {
            Ok(Some(options_string)) => {
                echo!(
                    "Using options from {} for this app: {}",
                    path,
                    options_string
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
                echo!("No options found for this app in {}", path);
            }
            Err(e) => {
                echo!("Warning: {}", e);
            }
        }
        Ok(())
    }
    let default_options_path = paths::DEFAULT_OPTIONS_FILE;
    match paths::ResourceFile::open(default_options_path) {
        Ok(mut file) => apply_options(file.get(), default_options_path, &mut options, app_id)?,
        Err(err) => echo!("Warning: Could not open {}: {}", default_options_path, err),
    }
    let user_options_path = paths::user_data_base_path().join(paths::USER_OPTIONS_FILE);
    match std::fs::File::open(&user_options_path) {
        Ok(file) => apply_options(file, user_options_path.display(), &mut options, app_id)?,
        Err(err) => echo!(
            "Warning: Could not open {}: {}",
            user_options_path.display(),
            err
        ),
    }
    echo!();

    // Apply command-line options
    for option_arg in option_args {
        let parse_result = options.parse_argument(&option_arg);
        assert!(parse_result == Ok(true));
    }

    let mut env = Environment::new(bundle, fs, options, env_for_salvage)?;
    env.run();
    Ok(())
}
