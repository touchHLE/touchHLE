/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::env;
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
    build.define("DYNARMIC_FRONTENDS", "A32"); // We don't need 64-bit
    build.define("DYNARMIC_WARNINGS_AS_ERRORS", "OFF");
    build.define("DYNARMIC_TESTS", "OFF");
    build.define("DYNARMIC_USE_BUNDLED_EXTERNALS", "ON");
    // This is Windows- and Android-specific because on macOS or Linux, you can
    // easily get Boost with a package manager.
    let os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS was not set");
    let boost_path = workspace_root.join("vendor/boost");
    if (os.eq_ignore_ascii_case("windows") || os.eq_ignore_ascii_case("android"))
        && !boost_path.is_dir()
    {
        panic!("Could not find Boost. Download it from https://www.boost.org/users/download/ and put it at vendor/boost");
    }
    // Allow providing Boost manually regardless of what platform we're on
    // (or whether the target platform was detected correctly…)
    if boost_path.is_dir() {
        build.define("Boost_INCLUDE_DIR", boost_path);
    }
    // Prevent CMake from using macOS-only linker commands when cross-compiling
    // for Android.
    // https://stackoverflow.com/questions/69697715/cross-compiling-c-program-for-android-on-mac-failed-using-ndks-clang
    if os.eq_ignore_ascii_case("android") {
        build.define("CMAKE_SYSTEM_NAME", "Linux");
        build.define("ANDROID", "ON");
    }
    // dynarmic can't be dynamically linked
    let dynarmic_out = build.build();

    if os.eq_ignore_ascii_case("android") {
        // Work around weird issue with the NDK where there are missing
        // references to compiler-rt/libgcc symbols.
        // Translated from: https://github.com/termux/termux-packages/issues/8029#issuecomment-1369150244
        let mut cc_command = cc::Build::new().get_compiler().to_command();
        let libclang_rt_path = cc_command
            .arg("-print-libgcc-file-name")
            .output()
            .unwrap()
            .stdout;
        let libclang_rt_path: &Path = std::str::from_utf8(&libclang_rt_path).unwrap().as_ref();
        link_search(libclang_rt_path.parent().unwrap());
        link_lib(
            libclang_rt_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .trim()
                .strip_prefix("lib")
                .unwrap()
                .strip_suffix(".a")
                .unwrap(),
        );
    }

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
    let arch = env::var("CARGO_CFG_TARGET_ARCH").expect("CARGO_CFG_TARGET_ARCH was not set");
    if arch.eq_ignore_ascii_case("x86_64") {
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
        .std("c++17")
        .include(dynarmic_out.join("include"))
        .compile("dynarmic_wrapper");
    rerun_if_changed(&package_root.join("lib.cpp"));
}
