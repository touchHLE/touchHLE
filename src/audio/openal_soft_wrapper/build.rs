/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use std::env;
use std::path::Path;

/*fn rerun_if_changed(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
}*/
fn link_search(path: &Path) {
    println!("cargo:rustc-link-search=native={}", path.to_str().unwrap());
}
fn link_lib(lib: &str) {
    if cfg!(feature = "static") {
        println!("cargo:rustc-link-lib=static={}", lib);
    } else {
        println!("cargo:rustc-link-lib=dylib={}", lib);
    }
}

fn link_framework(framework: &str) {
    println!("cargo:rustc-link-lib=framework={}", framework);
}

fn main() {
    if cfg!(feature = "static") {
        let os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS was not set");

        let package_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = package_root.join("../../..");

        let mut build = cmake::Config::new(workspace_root.join("vendor/openal-soft"));

        // Prevent any out dir re-use for Android. For some reason, this causes
        // weird linker errors when cross-compiling on macOS.
        let out_dir = Path::new(&env::var("OUT_DIR").unwrap()).join(&os);
        if os.eq_ignore_ascii_case("android") {
            let _ = std::fs::remove_dir_all(&out_dir);
        }
        build.out_dir(out_dir);

        build.define("LIBTYPE", "STATIC");

        // Don't build extras, we don't need them and they can encounter issues
        // when cross-compiling.
        build.define("ALSOFT_UTILS", "OFF");
        build.define("ALSOFT_NO_CONFIG_UTIL", "ON");
        build.define("ALSOFT_EXAMPLES", "OFF");

        let openal_soft_out = build.build();

        link_search(&openal_soft_out.join("lib"));
        // some Linux systems
        link_search(&openal_soft_out.join("lib64"));

        // Some dependencies of OpenAL Soft.
        if os.eq_ignore_ascii_case("linux") {
            // OpenAL on Linux depends on sndio, needs to be dynamically linked
            println!("cargo:rustc-link-lib=dylib=sndio");
        }
        if os.eq_ignore_ascii_case("android") {
            println!("cargo:rustc-link-lib=dylib=OpenSLES");
        }
        if os.eq_ignore_ascii_case("macos") {
            link_framework("AudioToolbox");
            link_framework("CoreAudio");
            link_framework("CoreFoundation");
        }
    }

    // see also src/audio/openal.rs
    link_lib(if cfg!(target_os = "windows") {
        "OpenAL32"
    } else {
        "openal"
    });
    // rerun-if-changed seems to not work if pointed to a directory :(
    //rerun_if_changed(&workspace_root.join("vendor/openal-soft"));
}
