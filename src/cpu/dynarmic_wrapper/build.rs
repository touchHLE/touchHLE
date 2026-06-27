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
    println!("cargo:rustc-link-lib=static={lib}");
}

fn build_type_windows() -> &'static str {
    let os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS was not set");
    if os.eq_ignore_ascii_case("windows") {
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
    let dynarmic_root = workspace_root.join("vendor/dynarmic");

    let mut build = cmake::Config::new(&dynarmic_root);
    build.define("DYNARMIC_FRONTENDS", "A32"); // We don't need 64-bit
    build.define("DYNARMIC_WARNINGS_AS_ERRORS", "OFF");
    build.define("DYNARMIC_TESTS", "OFF");
    build.define("DYNARMIC_USE_BUNDLED_EXTERNALS", "ON");
    build.define("CMAKE_POLICY_VERSION_MINIMUM", "3.5");

    // The fmt library bundled with dynarmic (v10.1.0) uses a `consteval`
    // constructor for `basic_format_string` to perform compile-time format
    // string checking. Newer compilers (e.g. AppleClang 21 / Xcode 26) reject
    // this because the constructor's pointer arithmetic isn't a constant
    // expression under their stricter `consteval` evaluation, breaking the
    // build. Predefining FMT_CONSTEVAL to empty makes fmt's `#ifndef
    // FMT_CONSTEVAL` block a no-op, so the constructor is no longer
    // `consteval` and FMT_HAS_CONSTEVAL stays undefined (it gates the
    // compile-time `parse_format_string` call). fmt then falls back to runtime
    // format-string checking. This is harmless on older compilers too.
    build.cxxflag("-DFMT_CONSTEVAL=");

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
        build.define("CMAKE_SYSTEM_NAME", "Android");
        build.define("CMAKE_SYSTEM_VERSION", "21");
        build.define("ANDROID", "ON");
        // Without this, CMake's architecture probe sees 32-bit `__arm__` and
        // builds dynarmic for arm instead of arm64, which breaks 64-bit FP
        // helpers (e.g. FPRecipExponent) when cross-compiling for AArch64.
        build.define("CMAKE_ANDROID_ARCH_ABI", "arm64-v8a");
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
    //rerun_if_changed(&dynarmic_root);
    rerun_if_changed(&workspace_root.join(".git/modules/dynarmic/HEAD"));

    let mut wrapper_build = cc::Build::new();
    wrapper_build
        .file(package_root.join("lib.cpp"))
        .cpp(true)
        .std("c++17")
        .include(dynarmic_out.join("include"));
    if !cfg!(debug_assertions) {
        wrapper_build.define("NDEBUG", "1");
    }
    wrapper_build.compile("dynarmic_wrapper");
    rerun_if_changed(&package_root.join("lib.cpp"));
}
