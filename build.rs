/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use cargo_license::{get_dependencies_from_cargo_lock, GetDependenciesOpt};
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn rerun_if_changed(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
}

pub fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let package_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Try to get the version using `git describe`, otherwise fall back to the
    // Cargo.toml version. This is used in main.rs

    let version = Command::new("git").arg("describe").arg("--always").output();
    let version = if version.is_ok() && version.as_ref().unwrap().status.success() {
        rerun_if_changed(&package_root.join(".git/HEAD"));
        rerun_if_changed(&package_root.join(".git/refs"));
        format!(
            "{} (git)",
            std::str::from_utf8(&version.unwrap().stdout)
                .unwrap()
                .trim_end()
        )
    } else {
        rerun_if_changed(&package_root.join("Cargo.toml"));
        format!("v{}", std::env::var("CARGO_PKG_VERSION").unwrap())
    };
    std::fs::write(out_dir.join("version.txt"), version).unwrap();

    // Generate a list of dependencies with license and author information.
    // This is used in license.rs

    let deps = get_dependencies_from_cargo_lock(
        Default::default(),
        GetDependenciesOpt {
            // The goal is to get a list of dependencies which are used in the
            // final binary, so we can comply with license terms for binary
            // distribution. Therefore, dev- and build-time dependencies don't
            // matter.
            avoid_dev_deps: true,
            avoid_build_deps: true,
            direct_deps_only: false,
            root_only: false,
        },
    )
    .unwrap();
    let mut deps_string = String::new();
    for dep in deps {
        // Exclude internal packages, they all use the same license and are
        // handled specially.
        if dep.name.starts_with("touchHLE") {
            continue;
        }

        write!(&mut deps_string, "- {} version {}", dep.name, dep.version).unwrap();
        if let Some(authors) = dep.authors {
            let authors: Vec<&str> = authors.split('|').collect();
            write!(&mut deps_string, " by {}", authors.join(", ")).unwrap();
        } else {
            write!(&mut deps_string, " (author unspecified)").unwrap();
        }
        if let Some(license) = dep.license {
            write!(&mut deps_string, ", licensed under {}", license).unwrap();
        } else {
            panic!("Dependency {} has an unspecified license!", dep.name);
        }
        writeln!(&mut deps_string).unwrap();
    }

    std::fs::write(out_dir.join("rust_dependencies.txt"), deps_string).unwrap();

    rerun_if_changed(&package_root.join("Cargo.lock"));

    // Summarise the licensing of Dynarmic

    let dynarmic_readme_path = package_root.join("vendor/dynarmic/README.md");
    let dynarmic_readme = std::fs::read_to_string(&dynarmic_readme_path).unwrap();
    rerun_if_changed(&dynarmic_readme_path);
    let dynarmic_license_path = package_root.join("vendor/dynarmic/LICENSE.txt");
    let dynarmic_license = std::fs::read_to_string(&dynarmic_license_path).unwrap();
    rerun_if_changed(&dynarmic_license_path);

    // Attempt to support Windows where git autocrlf may confuse things.
    let dynarmic_readme = dynarmic_readme.replace("\r\n", "\n");
    let (_, dynarmic_legal) = dynarmic_readme.split_once("\nLegal\n-----\n").unwrap();
    // Strip out the code block start and end lines. They're visual noise when
    // displayed in ASCII and there's one of these that ends up as its own page
    // in the license text viewer!
    let dynarmic_legal = dynarmic_legal.replace("\n```\n", "\n");
    let dynarmic_license_oneline =
        "dynarmic is under a 0BSD license. See LICENSE.txt for more details.";
    assert!(dynarmic_legal.contains(dynarmic_license_oneline));
    let dynarmic_summary = dynarmic_legal.replace(dynarmic_license_oneline, &dynarmic_license);
    std::fs::write(out_dir.join("dynarmic_license.txt"), dynarmic_summary).unwrap();
}
