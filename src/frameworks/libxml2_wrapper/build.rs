/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Build script for the libxml2 wrapper.
//!
//! When the `static` feature is enabled (default in the parent crate), this
//! script builds the vendored libxml2 sources at `vendor/libxml2` as a static
//! library and instructs Cargo to link it.  All major libxml2 sub-modules are
//! enabled (parser, tree, xmlreader, xmlwriter, xpath, xpointer, xinclude,
//! schemas, Relax-NG, schematron, catalog, c14n, html, etc.); only the optional
//! third-party dependencies (iconv, icu, lzma, zlib, http, ftp, modules, python)
//! are disabled so the build remains hermetic.
use std::env;
use std::path::{Path, PathBuf};

fn link_search(path: &Path) {
    println!("cargo:rustc-link-search=native={}", path.to_str().unwrap());
}

fn link_lib(lib: &str) {
    if cfg!(feature = "static") {
        println!("cargo:rustc-link-lib=static={lib}");
    } else {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }
}

fn main() {
    let os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS was not set");
    let package_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = package_root.join("../../..");
    let src_dir = workspace_root.join("vendor/libxml2");

    let mut include_dirs: Vec<PathBuf> = Vec::new();

    if cfg!(feature = "static") {
        if !src_dir.join("CMakeLists.txt").exists() {
            panic!(
                "vendor/libxml2 sources are missing.  Run `git submodule update --init` to fetch them."
            );
        }

        let mut build = cmake::Config::new(&src_dir);

        // Some Android cross builds confuse out-dir reuse; isolate it per-OS.
        let out_dir = Path::new(&env::var("OUT_DIR").unwrap()).join(&os);
        if os.eq_ignore_ascii_case("android") {
            let _ = std::fs::remove_dir_all(&out_dir);
        }
        build.out_dir(out_dir);

        // Tell the C compiler we are using libxml2 as a static library so the
        // headers don't try to `__declspec(dllimport)` everything on Windows.
        build.define("BUILD_SHARED_LIBS", "OFF");
        build.define("LIBXML2_WITH_PROGRAMS", "OFF");
        build.define("LIBXML2_WITH_TESTS", "OFF");
        build.define("LIBXML2_WITH_PYTHON", "OFF");
        build.define("LIBXML2_WITH_MODULES", "OFF");
        build.define("LIBXML2_WITH_LEGACY", "ON");

        // Optional third-party dependencies — turn them off so the build is
        // hermetic and produces no link-time references to iconv/icu/zlib/lzma.
        build.define("LIBXML2_WITH_ICONV", "OFF");
        build.define("LIBXML2_WITH_ICU", "OFF");
        build.define("LIBXML2_WITH_LZMA", "OFF");
        build.define("LIBXML2_WITH_ZLIB", "OFF");
        build.define("LIBXML2_WITH_HTTP", "OFF");
        build.define("LIBXML2_WITH_FTP", "OFF");

        // Built-in alternative encodings cover what iPhone OS apps actually
        // need (ASCII/UTF-8/UTF-16/Latin1/ISO-8859-x).
        build.define("LIBXML2_WITH_ISO8859X", "ON");

        // Everything else — every libxml2 sub-module — stays on.
        for var in &[
            "LIBXML2_WITH_C14N",
            "LIBXML2_WITH_CATALOG",
            "LIBXML2_WITH_DEBUG",
            "LIBXML2_WITH_HTML",
            "LIBXML2_WITH_OUTPUT",
            "LIBXML2_WITH_PATTERN",
            "LIBXML2_WITH_PUSH",
            "LIBXML2_WITH_READER",
            "LIBXML2_WITH_REGEXPS",
            "LIBXML2_WITH_SAX1",
            "LIBXML2_WITH_SCHEMAS",
            "LIBXML2_WITH_SCHEMATRON",
            "LIBXML2_WITH_THREADS",
            "LIBXML2_WITH_TREE",
            "LIBXML2_WITH_VALID",
            "LIBXML2_WITH_WRITER",
            "LIBXML2_WITH_XINCLUDE",
            "LIBXML2_WITH_XPATH",
            "LIBXML2_WITH_XPTR",
        ] {
            build.define(*var, "ON");
        }

        // Don't fail builds on harmless warnings in upstream sources.
        build.define("CMAKE_POLICY_VERSION_MINIMUM", "3.5");

        let install_dir = build.build();

        link_search(&install_dir.join("lib"));
        // Some Linux distributions install to lib64.
        link_search(&install_dir.join("lib64"));

        include_dirs.push(install_dir.join("include").join("libxml2"));
        include_dirs.push(install_dir.join("include"));
    } else {
        // Fallback: rely on the system libxml2 (helpful for desktop builds
        // when the static feature is disabled).  pkg-config would be ideal,
        // but we keep this minimal and just point at the conventional path.
        include_dirs.push(PathBuf::from("/usr/include/libxml2"));
    }

    // The library file is named differently on different platforms:
    //   Linux/macOS/Android: libxml2.a (link name "xml2")
    //   Windows MSVC: libxml2s.lib for static builds (CMake adds the trailing s)
    if cfg!(feature = "static") && os.eq_ignore_ascii_case("windows") {
        link_lib("libxml2s");
    } else if os.eq_ignore_ascii_case("windows") {
        link_lib("libxml2");
    } else {
        link_lib("xml2");
    }

    // libxml2's threads code uses pthreads on Unix-likes.
    // On Android, pthread is part of libc (bionic), so there is no separate
    // libpthread to link against — emitting `-lpthread` causes a link failure.
    if !os.eq_ignore_ascii_case("windows") && !os.eq_ignore_ascii_case("android") {
        println!("cargo:rustc-link-lib=dylib=pthread");
    }
    if os.eq_ignore_ascii_case("linux") || os.eq_ignore_ascii_case("android") {
        // libxml2 uses ftime/clock_gettime on Linux which can need -lm/-lrt
        // (rt is folded into libc on modern glibc but Android sometimes needs it).
        println!("cargo:rustc-link-lib=dylib=m");
    }
    if os.eq_ignore_ascii_case("windows") {
        // Required by xmlIO on Windows for ws2_32 (socket fallbacks even when
        // HTTP/FTP are disabled, because of the public xmlNanoHTTP* declarations).
        println!("cargo:rustc-link-lib=dylib=ws2_32");
    }

    // Compile the small C shim against the libxml2 headers.
    let mut cc_build = cc::Build::new();
    cc_build.file(package_root.join("shim.c"));
    for inc in &include_dirs {
        cc_build.include(inc);
    }
    cc_build.define("LIBXML_STATIC", Some("1"));
    cc_build.warnings(false);
    cc_build.compile("touchHLE_libxml2_shim");

    println!(
        "cargo:rerun-if-changed={}",
        package_root.join("shim.c").to_str().unwrap()
    );
    println!(
        "cargo:rerun-if-changed={}",
        package_root.join("build.rs").to_str().unwrap()
    );
}
