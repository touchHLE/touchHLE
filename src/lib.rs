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
mod audio;
mod bundle;
mod cpu;
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

use std::ffi::OsStr;
use std::path::PathBuf;

/// Current version. See `build.rs` for how this is generated.
const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

/// This is the true entry point on Android (SDLActivity calls it after
/// initialization). On other platforms the true entry point is in src/bin.rs.
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn SDL_main(
    _argc: std::ffi::c_int,
    _argv: *const *const std::ffi::c_char,
) -> std::ffi::c_int {
    // Rust's default panic handler prints to stderr, but on Android that just
    // gets discarded, so we set a custom hook to make debugging easier.
    std::panic::set_hook(Box::new(|info| {
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            &s
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
    return 0;
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

fn app_picker(title: &str) -> Result<PathBuf, String> {
    let apps_dir = paths::user_data_base_path().join(paths::APPS_DIR);

    fn enumerate_apps(apps_dir: &std::path::Path) -> Result<Vec<PathBuf>, std::io::Error> {
        let mut app_paths = Vec::new();
        for app in std::fs::read_dir(apps_dir)? {
            let app_path = app?.path();
            if app_path.extension() != Some(OsStr::new("app"))
                && app_path.extension() != Some(OsStr::new("ipa"))
            {
                continue;
            }
            if app_path.to_str().is_none() {
                continue;
            }
            app_paths.push(app_path);
        }
        Ok(app_paths)
    }

    let app_paths: Result<Vec<PathBuf>, String> = if !apps_dir.is_dir() {
        Err(format!("The {} directory couldn't be found. Check you're running touchHLE from the right directory.", apps_dir.display()))
    } else {
        enumerate_apps(&apps_dir).map_err(|err| {
            format!(
                "Couldn't get list of apps in the {} directory: {}.",
                apps_dir.display(),
                err
            )
        })
    };

    let is_error = app_paths.is_err();
    let (app_paths, mut message): (&[PathBuf], String) = match app_paths {
        Ok(ref paths) => (
            paths,
            if !paths.is_empty() {
                "Select an app:".to_string()
            } else {
                format!(
                    "No apps were found in the {} directory.",
                    apps_dir.display()
                )
            },
        ),
        Err(err) => (&[], err),
    };

    let mut app_buttons: Vec<(i32, &str)> = app_paths
        .iter()
        .enumerate()
        .map(|(idx, path)| {
            let name = path.file_name().unwrap().to_str().unwrap();
            (
                idx.try_into().unwrap(),
                // On Windows, the buttons are too small to display a full app
                // name, so it's more practical to use a short symbol and put
                // the full name in the message.
                // As for Android, there's not enough horizontal space for
                // multiple buttons if we don't do this.
                if cfg!(target_os = "windows") || cfg!(target_os = "android") {
                    // TODO: hopefully we'll have a better app picker before we
                    // have more than twenty supported apps? ^^;
                    let symbols = [
                        "(1)", "(2)", "(3)", "(4)", "(5)", "(6)", "(7)", "(8)", "(9)", "(10)",
                        "(11)", "(12)", "(13)", "(14)", "(15)", "(16)", "(17)", "(18)", "(19)",
                        "(20)",
                    ];
                    let symbol = symbols[idx % symbols.len()];
                    use std::fmt::Write;
                    write!(message, "\n{} {}", symbol, name).unwrap();
                    symbol
                } else {
                    name
                },
            )
        })
        .chain([(-1, "Exit")])
        .collect();
    // On Windows, the buttons are laid out from right to left, but users
    // will presumably expect the opposite. Do in-place reverse so the order
    // of lines in the message isn't affected.
    if cfg!(target_os = "windows") {
        app_buttons.reverse();
    }

    loop {
        match window::show_message_with_options(title, &message, is_error, &app_buttons) {
            Some(app_idx @ 0..) => return Ok(app_paths[app_idx as usize].clone()),
            None | Some(-1) => return Err("No app was selected".to_string()),
            _ => unreachable!(),
        }
    }
}

pub fn main<T: Iterator<Item = String>>(mut args: T) -> Result<(), String> {
    let long_title = format!("touchHLE {} — https://touchhle.org/", VERSION);

    echo!("{}", long_title);
    echo!();

    {
        let base_path = paths::user_data_base_path().to_str().unwrap();
        if !base_path.is_empty() {
            log!("Base path for touchHLE files: {}", base_path);
        }
    }

    let _ = args.next().unwrap(); // skip argv[0]

    let mut bundle_path: Option<PathBuf> = None;
    let mut just_info = false;
    let mut option_args = Vec::new();

    for arg in args {
        if arg == "--help" {
            echo!("{}", USAGE);
            echo!("{}", options::DOCUMENTATION);
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
            echo!("{}", USAGE);
            echo!("{}", options::DOCUMENTATION);
            return Err(format!("Unexpected argument: {:?}", arg));
        }
    }

    let bundle_path = if let Some(bundle_path) = bundle_path {
        bundle_path
    } else {
        echo!(
            "No app specified, opening app picker. Use the --help flag to see command-line usage."
        );
        app_picker(&long_title)?
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
    let (bundle, fs) = match bundle::Bundle::new_bundle_and_fs_from_host_path(bundle_data) {
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
        let (major, _minor_etc) = version.split_once('.').unwrap();
        let major: u32 = major.parse().unwrap();
        if major > 2 {
            echo!("Warning: app requires OS version {}. Only iPhone OS 2 apps are currently supported.", version);
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

    let mut env = Environment::new(bundle, fs, options)?;
    env.run();
    Ok(())
}
