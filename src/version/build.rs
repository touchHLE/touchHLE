/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::path::{Path, PathBuf};
use std::process::Command;

fn rerun_if_changed(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
}

pub fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let package_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = package_root.join("../..");

    // Try to get the version using `git describe`, otherwise fall back to the
    // Cargo.toml version. This is used in main.rs

    let toml_version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let version = Command::new("git").arg("describe").arg("--always").output();
    let version = match version.as_ref() {
        Ok(version) if version.status.success() => {
            rerun_if_changed(&workspace_root.join(".git/HEAD"));
            rerun_if_changed(&workspace_root.join(".git/refs"));
            let git_version = std::str::from_utf8(&version.stdout)
                .unwrap()
                .trim_end()
                .to_string();
            if git_version
                .strip_prefix('v')
                .is_some_and(|v| !v.starts_with(&toml_version))
                || !git_version.starts_with('v')
            {
                println!("cargo:warning=Cargo.toml version (v{toml_version}) is not a prefix of `git describe` version ({git_version})!");
            }
            git_version
        }
        _ => {
            rerun_if_changed(&workspace_root.join("Cargo.toml"));
            format!("v{toml_version} (git rev. unknown)")
        }
    };
    std::fs::write(out_dir.join("version.txt"), version).unwrap();
}
