/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! In-app updater.
//!
//! On startup, touchHLE/HyperHLE pings GitHub for the latest commit built on
//! the `trunk` branch and compares it against the commit this build was made
//! from (see [crate::VERSION], which is `git describe --always`, i.e. the short
//! commit hash for CI builds). Pull-request builds are excluded: those are built
//! from the `<pr number>/merge` ref (that's what produces the
//! `Built from branch "<pr number>/merge"` line in their logs), so by only
//! looking at successful `push` runs on `trunk` we never offer a PR build as an
//! "update".
//!
//! When a newer commit is found, the user is asked (via an SDL message box)
//! whether they want to update. If they accept, the latest build artifacts are
//! downloaded from nightly.link and extracted over the top of the current
//! installation.

use crate::VERSION;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Repository to check for updates, in `owner/repo` form. Locked to the
/// upstream HyperHLE repository regardless of where this build was produced.
const REPO: &str = "HyperHLE/HyperHLE";

/// The branch official builds are produced from.
const BRANCH: &str = "trunk";

/// The build workflow's file name, without the `.yml` extension. Used both for
/// the GitHub Actions API and for nightly.link artifact URLs.
const WORKFLOW: &str = "HyperHLE_release";

/// The build artifact for the platform we're running on, or [None] on a
/// platform that has no published build. Only the host platform's artifact is
/// runnable here, so that's the only one downloaded.
fn host_artifact() -> Option<&'static str> {
    match std::env::consts::OS {
        "android" => Some("HyperHLE_Android_AArch64"),
        "linux" => Some("HyperHLE_Linux_x86_64"),
        "windows" => Some("HyperHLE_Windows_x86_64"),
        "macos" => Some("HyperHLE_macOS_x86_64"),
        _ => None,
    }
}

/// User-Agent sent with HTTP requests. GitHub's API rejects requests without
/// one.
const USER_AGENT: &str = "HyperHLE-Updater";

/// The commit this build was made from, if it can be determined.
///
/// [VERSION] is the output of `git describe --always`, which is the short
/// commit hash for CI builds (e.g. `8186660`), or something like
/// `v0.2.3-5-g8186660` when annotated tags are reachable. Either way, the
/// trailing component is the abbreviated commit hash. Returns [None] for
/// non-CI builds where the version isn't a commit hash (e.g.
/// `v0.2.3 (git rev. unknown)`), in which case we can't tell whether an update
/// is available and silently skip the check.
fn local_commit() -> Option<String> {
    let version = VERSION.trim();
    // `v0.2.3-5-g8186660` -> `8186660`; bare `8186660` is unchanged.
    let candidate = version.rsplit("-g").next().unwrap_or(version);
    let candidate = candidate.trim_start_matches('v');
    if candidate.len() >= 7 && candidate.bytes().all(|b| b.is_ascii_hexdigit()) {
        Some(candidate.to_ascii_lowercase())
    } else {
        None
    }
}

/// Perform an HTTP GET and return the response body as bytes. Follows redirects
/// (nightly.link responds with a redirect to the actual artifact).
fn http_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let response = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut buf)
        .map_err(|e| e.to_string())?;
    Ok(buf)
}

/// Ask GitHub for the latest commit built on `trunk`, excluding pull-request
/// builds. Returns the full commit SHA of the most recent successful `push`
/// build, if any.
fn latest_trunk_commit() -> Result<Option<String>, String> {
    // Only successful `push` runs on `trunk` are considered. `pull_request`
    // runs (which are built from the `<pr number>/merge` ref) are excluded by
    // the `event=push` filter, so a PR build is never mistaken for an update.
    let url = format!(
        "https://api.github.com/repos/{}/actions/workflows/{}.yml/runs\
         ?branch={}&event=push&status=success&per_page=1",
        REPO, WORKFLOW, BRANCH,
    );
    let body = http_get_bytes(&url)?;
    let json: serde_json::Value = serde_json::from_slice(&body).map_err(|e| e.to_string())?;
    Ok(json["workflow_runs"][0]["head_sha"]
        .as_str()
        .map(|s| s.to_ascii_lowercase()))
}

/// File name the Android update APK is staged under, shared with the Java side
/// (see `MainActivity.installApk`).
#[cfg(target_os = "android")]
const APK_FILE_NAME: &str = "HyperHLE_update.apk";

/// Suffix appended to a file that is moved aside because it couldn't be
/// overwritten in place (see [replace_file]). Leftover files with this suffix
/// are deleted at the start of the next update by [remove_stale_backups].
#[cfg(not(target_os = "android"))]
const BACKUP_SUFFIX: &str = ".hyperhle-old";

/// The directory of the running touchHLE installation (the "main folder"),
/// into which updated files are extracted.
#[cfg(not(target_os = "android"))]
fn main_folder() -> PathBuf {
    // SDL2's base path is the directory containing the executable. This is the
    // root of the bundle on the desktop platforms where self-updating makes
    // sense.
    sdl2::filesystem::base_path()
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// A writable scratch directory for staging a downloaded update before it is
/// applied.
fn staging_dir() -> PathBuf {
    // `std::env::temp_dir()` isn't reliably writable on Android, so stage in the
    // app's own external files directory there.
    #[cfg(target_os = "android")]
    let dir = crate::paths::user_data_base_path().join(".hyperhle_update_tmp");
    #[cfg(not(target_os = "android"))]
    let dir = std::env::temp_dir().join(format!("hyperhle_update_{}", std::process::id()));
    dir
}

/// Download an artifact `.zip` from nightly.link and extract it into `dir`.
fn download_and_extract_to(artifact: &str, dir: &Path) -> Result<(), String> {
    // Format: https://nightly.link/<owner>/<repo>/workflows/<workflow>/<branch>/<artifact>.zip
    let url = format!(
        "https://nightly.link/{}/workflows/{}/{}/{}.zip",
        REPO, WORKFLOW, BRANCH, artifact,
    );
    echo!("Downloading {}...", url);
    let bytes = http_get_bytes(&url)?;

    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        // Use the sanitised path to avoid zip-slip writes outside `dir`.
        let Some(relative) = entry.enclosed_name() else {
            log!("Warning: skipping unsafe path in {} artifact", artifact);
            continue;
        };
        let out_path = dir.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&out_path).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Whether the file at `new` differs from the one at `installed`. A missing
/// `installed` file counts as "differs". Compares by length and then byte
/// content (streamed, so large binaries aren't fully buffered).
#[cfg(not(target_os = "android"))]
fn files_differ(new: &Path, installed: &Path) -> Result<bool, String> {
    let Ok(installed_meta) = std::fs::metadata(installed) else {
        return Ok(true); // not installed yet
    };
    let new_meta = std::fs::metadata(new).map_err(|e| e.to_string())?;
    if new_meta.len() != installed_meta.len() {
        return Ok(true);
    }
    let mut new_file = std::fs::File::open(new).map_err(|e| e.to_string())?;
    let mut installed_file = std::fs::File::open(installed).map_err(|e| e.to_string())?;
    let mut new_buf = [0u8; 8192];
    let mut installed_buf = [0u8; 8192];
    loop {
        let n = new_file.read(&mut new_buf).map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(false); // reached EOF with no difference (equal lengths)
        }
        // The files are the same length and read in lock-step, so the installed
        // file always has at least `n` more bytes available here.
        installed_file
            .read_exact(&mut installed_buf[..n])
            .map_err(|e| e.to_string())?;
        if new_buf[..n] != installed_buf[..n] {
            return Ok(true);
        }
    }
}

/// Copy `from` over `to`, replacing whatever is already there.
///
/// A plain [std::fs::copy] is enough on Unix, but on Windows a file that is
/// currently executing (the running `.exe`) or mapped into the process (a
/// loaded `.dll`) is locked and can't be opened for writing — the copy fails
/// with `os error 32` ("being used by another process"). Such a file *can*
/// still be renamed, though: the running process keeps using the open handle
/// regardless of the file's name. So when the in-place copy fails, move the
/// locked file aside (to a [BACKUP_SUFFIX] name) and retry into the now-free
/// path. The leftover backup is removed by [remove_stale_backups] next time.
#[cfg(not(target_os = "android"))]
fn replace_file(from: &Path, to: &Path) -> Result<(), String> {
    match std::fs::copy(from, to) {
        Ok(_) => Ok(()),
        // No existing file is in the way; the error is something else (e.g. a
        // missing parent directory), so don't bother with the rename dance.
        Err(_) if !to.exists() => std::fs::copy(from, to).map(|_| ()).map_err(|e| e.to_string()),
        Err(copy_err) => {
            let mut backup = to.as_os_str().to_os_string();
            backup.push(BACKUP_SUFFIX);
            let backup = PathBuf::from(backup);
            // A backup from a previous failed/interrupted update may still be
            // there; the rename below would fail on Windows if it is.
            let _ = std::fs::remove_file(&backup);
            std::fs::rename(to, &backup).map_err(|rename_err| {
                format!("{copy_err} (and couldn't move the existing file aside: {rename_err})")
            })?;
            match std::fs::copy(from, to) {
                Ok(_) => Ok(()),
                Err(retry_err) => {
                    // Put the original back so the installation isn't left broken.
                    let _ = std::fs::rename(&backup, to);
                    Err(retry_err.to_string())
                }
            }
        }
    }
}

/// Recursively copy files from `src` into `dest`, writing only the files whose
/// contents differ from (or are missing at) the destination. Returns how many
/// files were created or replaced.
#[cfg(not(target_os = "android"))]
fn sync_changed_files(src: &Path, dest: &Path) -> Result<usize, String> {
    let mut replaced = 0;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            std::fs::create_dir_all(&to).map_err(|e| e.to_string())?;
            replaced += sync_changed_files(&from, &to)?;
        } else if files_differ(&from, &to)? {
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            replace_file(&from, &to)?;
            echo!("Updated {}", to.display());
            replaced += 1;
        }
    }
    Ok(replaced)
}

/// Recursively delete leftover [BACKUP_SUFFIX] files under `dir`. These are the
/// locked originals (e.g. the previous `.exe`) that [replace_file] moved aside
/// during an earlier update; they can only be removed once that older process
/// is no longer running, i.e. on a subsequent launch. Best-effort: anything
/// still locked or otherwise un-removable is just skipped.
#[cfg(not(target_os = "android"))]
fn remove_stale_backups(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            remove_stale_backups(&path);
        } else if path
            .as_os_str()
            .to_str()
            .is_some_and(|s| s.ends_with(BACKUP_SUFFIX))
        {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// Download this platform's build artifact and apply it. On the desktop this
/// replaces the changed files in place; on Android it hands the downloaded APK
/// to the system package installer.
fn perform_update() -> Result<(), String> {
    let artifact = host_artifact().ok_or_else(|| {
        format!(
            "no HyperHLE build is published for {}",
            std::env::consts::OS
        )
    })?;
    #[cfg(target_os = "android")]
    let result = perform_update_android(artifact);
    #[cfg(not(target_os = "android"))]
    let result = perform_update_desktop(artifact);
    result
}

/// Desktop update: extract the artifact into a staging folder, diff it against
/// the installation and replace only the files that actually changed.
#[cfg(not(target_os = "android"))]
fn perform_update_desktop(artifact: &str) -> Result<(), String> {
    let dest = main_folder();

    let temp_dir = staging_dir();
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let outcome = (|| {
        download_and_extract_to(artifact, &temp_dir)?;
        echo!(
            "Comparing the new build against the installation in {}...",
            dest.display()
        );
        sync_changed_files(&temp_dir, &dest)
    })();

    // Always clean up the staging folder, regardless of success.
    let _ = std::fs::remove_dir_all(&temp_dir);

    let replaced = outcome?;
    if replaced == 0 {
        echo!("Already up to date — no files needed replacing.");
    } else {
        echo!("Replaced {} changed file(s).", replaced);
    }
    Ok(())
}

/// Android update: an app can't overwrite its own installed binary, so download
/// the APK, stage it in the app's external files directory, and ask the system
/// package installer (via [install_apk_android]) to install it. The user
/// confirms the installation in the system UI.
#[cfg(target_os = "android")]
fn perform_update_android(artifact: &str) -> Result<(), String> {
    let temp_dir = staging_dir();
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let outcome = (|| {
        download_and_extract_to(artifact, &temp_dir)?;
        let apk = find_apk(&temp_dir)?
            .ok_or_else(|| "no .apk file found in the Android artifact".to_string())?;
        // Stage the APK at the fixed path the Java installer reads from.
        let dest = crate::paths::user_data_base_path().join(APK_FILE_NAME);
        std::fs::copy(&apk, &dest).map_err(|e| e.to_string())?;
        echo!("Staged update APK at {}", dest.display());
        install_apk_android()
    })();

    let _ = std::fs::remove_dir_all(&temp_dir);
    outcome
}

/// Recursively find the first `.apk` file under `dir`.
#[cfg(target_os = "android")]
fn find_apk(dir: &Path) -> Result<Option<PathBuf>, String> {
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_apk(&path)? {
                return Ok(Some(found));
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("apk") {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

/// Invoke `MainActivity.installApk()` on the Java side via JNI, which fires an
/// `ACTION_VIEW` install intent for the staged APK. SDL has already attached
/// the current (main) thread to the JVM, so its `JNIEnv` is valid here.
#[cfg(target_os = "android")]
fn install_apk_android() -> Result<(), String> {
    use jni::JNIEnv;

    extern "C" {
        fn SDL_AndroidGetJNIEnv() -> *mut std::ffi::c_void;
    }

    let env_ptr = unsafe { SDL_AndroidGetJNIEnv() };
    if env_ptr.is_null() {
        return Err("couldn't obtain a JNI environment from SDL".to_string());
    }
    let mut env =
        unsafe { JNIEnv::from_raw(env_ptr as *mut jni::sys::JNIEnv) }.map_err(|e| e.to_string())?;

    env.call_static_method(
        "org/touchhle/android/MainActivity",
        "installApk",
        "()V",
        &[],
    )
    .map_err(|e| e.to_string())?;
    echo!("Requested system installation of the update APK.");
    Ok(())
}

/// Ask the user, via an SDL message box, whether they want to update to
/// `new_commit`. Returns `true` if they chose to update.
fn prompt_user(new_commit: &str) -> bool {
    use sdl2::messagebox;
    let buttons = [
        messagebox::ButtonData {
            flags: messagebox::MessageBoxButtonFlag::RETURNKEY_DEFAULT,
            button_id: 1,
            text: "Update now",
        },
        messagebox::ButtonData {
            flags: messagebox::MessageBoxButtonFlag::ESCAPEKEY_DEFAULT,
            button_id: 0,
            text: "Later",
        },
    ];
    let short = &new_commit[..new_commit.len().min(7)];
    let message = format!(
        "A new version of HyperHLE is available (commit {short}).\n\
         You are currently running {VERSION}.\n\n\
         Would you like to download and install the update now?"
    );
    match messagebox::show_message_box(
        messagebox::MessageBoxFlag::INFORMATION,
        &buttons,
        "HyperHLE update available",
        &message,
        None::<&sdl2::video::Window>,
        None,
    ) {
        Ok(messagebox::ClickedButton::CustomButton(button)) => button.button_id == 1,
        // Closing the dialog or any error means "don't update".
        _ => false,
    }
}

/// Check for an update and, if one is available and the user accepts, perform
/// it. Best-effort: any failure (no network, rate limiting, etc.) is logged and
/// otherwise ignored so it never blocks startup.
pub fn check_for_update() {
    // Clean up any file the previous update moved aside because it was locked
    // (e.g. the old `.exe`). It couldn't be deleted while that process was
    // running, but it can be now.
    #[cfg(not(target_os = "android"))]
    remove_stale_backups(&main_folder());

    let Some(local) = local_commit() else {
        log_dbg!(
            "Skipping update check: build version {:?} is not a commit hash.",
            VERSION
        );
        return;
    };

    let latest = match latest_trunk_commit() {
        Ok(Some(latest)) => latest,
        Ok(None) => {
            log!("Update check: no successful trunk build found.");
            return;
        }
        Err(e) => {
            log!("Update check failed: {}", e);
            return;
        }
    };

    if latest.starts_with(&local) {
        log!("HyperHLE is up to date ({}).", local);
        return;
    }

    echo!(
        "A new HyperHLE version is available: {} (you have {}).",
        &latest[..latest.len().min(7)],
        local
    );

    if !prompt_user(&latest) {
        echo!("Update declined.");
        return;
    }

    match perform_update() {
        Ok(()) => {
            #[cfg(target_os = "android")]
            let message = "The system installer will now open to finish updating HyperHLE.";
            #[cfg(not(target_os = "android"))]
            let message =
                "Update complete\n\nHyperHLE will now close, please reopen it to use the new \
                 version.";
            echo!("{}", message);
            notify_result(true, message);
            // The update only takes effect on relaunch: the running process
            // still holds the old binary in memory, while the on-disk files are
            // now the new version. Exit so the next launch is cleanly updated.
            // (On Android the system installer handles the app lifecycle.)
            #[cfg(not(target_os = "android"))]
            std::process::exit(0);
        }
        Err(e) => {
            log!("Update failed: {}", e);
            notify_result(false, &format!("The update failed:\n\n{e}"));
        }
    }
}

/// Show an informational/error message box reporting the update result.
fn notify_result(success: bool, message: &str) {
    use sdl2::messagebox;
    let flag = if success {
        messagebox::MessageBoxFlag::INFORMATION
    } else {
        messagebox::MessageBoxFlag::ERROR
    };
    let buttons = [messagebox::ButtonData {
        flags: messagebox::MessageBoxButtonFlag::RETURNKEY_DEFAULT,
        button_id: 0,
        text: "OK",
    }];
    let _ = messagebox::show_message_box(
        flag,
        &buttons,
        "HyperHLE update",
        message,
        None::<&sdl2::video::Window>,
        None,
    );
}
