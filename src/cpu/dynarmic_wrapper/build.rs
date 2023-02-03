/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::path::Path;

fn rerun_if_changed(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
}
fn link_search(path: &Path) {
    println!("cargo:rustc-link-search=native={}", path.to_str().unwrap());
}
fn link_lib(lib: &str) {
    println!("cargo:rustc-link-lib=static={}", lib);
}

// See https://github.com/rust-lang/cc-rs/issues/565
trait CPPVersion {
    fn cpp_version(&mut self, version: &str) -> &mut Self;
}
impl CPPVersion for cc::Build {
    fn cpp_version(&mut self, version: &str) -> &mut Self {
        if self.get_compiler().is_like_msvc() {
            self.flag(&format!("/std:{}", version))
        } else {
            self.flag(&format!("-std={}", version))
        }
    }
}

fn build_type_windows() -> &'static str {
    if cfg!(target_os = "windows") {
        if cfg!(debug_assertions) {
            "Debug"
        } else {
            "Release"
        }
    } else {
        ""
    }
}

fn main() {
    let package_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = package_root.join("../../..");

    let mut build = cmake::Config::new(workspace_root.join("vendor/dynarmic"));
    build.define("DYNARMIC_WARNINGS_AS_ERRORS", "OFF");
    build.define("DYNARMIC_TESTS", "OFF");
    // This is Windows-specific because on macOS or Linux, you can grab
    // Boost with your package manager.
    if cfg!(target_os = "windows") {
        let boost_path = workspace_root.join("vendor/boost");
        if !boost_path.is_dir() {
            panic!("Could not find Boost. Download it from https://www.boost.org/users/download/ and put it at vendor/boost");
        }
        build.define("Boost_INCLUDE_DIR", boost_path);
    }
    let dynarmic_out = build.build();

    link_search(&dynarmic_out.join("lib"));
    link_search(&dynarmic_out.join("lib64")); // some Linux systems
    link_lib("dynarmic");
    link_search(
        &dynarmic_out
            .join("build/externals/fmt")
            .join(build_type_windows()),
    );
    link_lib(if cfg!(debug_assertions) {
        "fmtd"
    } else {
        "fmt"
    });
    link_search(
        &dynarmic_out
            .join("build/externals/mcl/src")
            .join(build_type_windows()),
    );
    link_lib("mcl");
    #[cfg(target_arch = "x86_64")]
    {
        link_search(
            &dynarmic_out
                .join("build/externals/zydis")
                .join(build_type_windows()),
        );
        link_lib("Zydis");
    }
    // rerun-if-changed seems to not work if pointed to a directory :(
    //rerun_if_changed(&workspace_root.join("vendor/dynarmic"));

    cc::Build::new()
        .file(package_root.join("lib.cpp"))
        .cpp(true)
        .cpp_version("c++17")
        .include(dynarmic_out.join("include"))
        .compile("dynarmic_wrapper");
    rerun_if_changed(&package_root.join("lib.cpp"));
}
