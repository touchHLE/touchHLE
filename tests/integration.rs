use std::env;
use std::env::current_dir;
use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

// adapted from `assert_cmd` crate
fn target_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .map(|mut path| {
            path.pop();
            if path.ends_with("deps") {
                path.pop();
            }
            path
        })
        .unwrap()
}

// https://stackoverflow.com/a/35907071/2241008
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn build_test_app(
    tests_dir: &Path,
    test_app_name: &str,
    sources: &[&Path],
    extra_compile_args: &[&str],
) -> Result<(), Box<dyn Error>> {
    let test_bin_path = tests_dir
        .join(format!("{}.app", test_app_name))
        .join(test_app_name);

    eprintln!("Building {} for iPhone OS 2...", test_bin_path.display());

    let output = Command::new("clang")
        .args(extra_compile_args)
        // Target iPhone OS 2
        .args(["-target", "armv6-apple-ios2"])
        // Don't search the deafult MacOS directories.
        .arg("-Z")
        // We don't have a libc to link against, don't try
        .arg("-nostdlib")
        // If enabled, the stack protection causes a null pointer crash in some
        // functions. This is probably because ___stack_chk_guard isn't linked.
        .arg("-fno-stack-protector")
        // Pass four args to the linker:
        // `-e _main` sets the mangled C main() function as the entry point
        // (normally the libc provides an entry point calling main(), but we
        // have no libc)
        // `-undefined dynamic_lookup` makes the linker tolerate undefined
        // references, falling back to dynamic linking instead. This is needed
        // because we have no system libraries/frameworks for it to link to.
        .arg("-Wl,-e,_main,-undefined,dynamic_lookup")
        // Input files.
        .args(sources.iter().map(|file| {
            tests_dir
                .join(format!("{}_source", test_app_name))
                .join(file)
        }))
        // Needed for dyld_stub_binding_helper, which is normally provided
        // in crt1.o but has to be added here.
        .arg(
            tests_dir
                .join(format!("{}_source", test_app_name))
                .join("crt1.c"),
        )
        // Write the output to the bundle.
        .arg("-o")
        .arg(test_bin_path)
        .output()
        .expect("failed to execute Clang process");

    std::io::stdout().write_all(&output.stdout).unwrap();
    std::io::stderr().write_all(&output.stderr).unwrap();

    assert!(output.status.success());

    eprintln!("Built successfully.");

    Ok(())
}

// Note that source files are looked for in the path
// "{tests_dir}/{test_app_name}_source"
// and binaries are output as
// "{tests_dir}/{test_app_name}.app/{test_app_name}".
// You also need crt1.c in tests_dir.
fn run_test_app(
    tests_dir: &Path,
    test_app_name: &str,
    sources: &[&Path],
    extra_compile_args: &[&str],
) -> Result<(), Box<dyn Error>> {
    let test_app_path = tests_dir.join(format!("{}.app", test_app_name));
    build_test_app(&tests_dir, &test_app_name, sources, extra_compile_args)?;

    let binary_name = "touchHLE";
    let binary_path = target_dir().join(format!("{}{}", binary_name, env::consts::EXE_SUFFIX));

    let mut cmd = Command::new(binary_path);

    let output = cmd
        .arg(test_app_path)
        // headless mode avoids a distracting window briefly appearing during
        // testing, and works in CI.
        .arg("--headless")
        .output()
        .expect("failed to execute touchHLE process");

    std::io::stdout().write_all(&output.stdout).unwrap();
    std::io::stderr().write_all(&output.stderr).unwrap();

    assert!(output.status.success());
    // sanity check: check that emulation actually happened
    assert_ne!(
        find_subsequence(output.stderr.as_slice(), b"CPU emulation begins now."),
        None
    );

    Ok(())
}

#[test]
fn test_app() -> Result<(), Box<dyn Error>> {
    // By default this uses the system linker, which is expected to ld for Mac
    // OS. By setting TOUCHHLE_LINKER to point to a ported (Apple) ld, you can
    // also build on linux (and potentially windows).
    let linker_path = env::var("TOUCHHLE_LINKER").map_or("".to_owned(), |linker_path| {
        format!("-fuse-ld={}", linker_path)
    });

    let libs_dir = "-L".to_owned() + current_dir()?.join("touchHLE_dylibs").to_str().unwrap();
    let extra_compile_args = [
        linker_path.as_str(),
        // ARC is not available until IOS 5, so it needs to be edited out.
        "-fno-objc-arc",
        "-fno-objc-arc-exceptions",
        // For some reason, we need to manually patch in the
        // libgcc_s dependency.
        "-lgcc_s.1",
        "-fobjc-link-runtime",
        libs_dir.as_str(),
        "-ObjC",
    ];

    let sources = ["main.m", "SyncTester.m"].map(|file| Path::new(file));

    let tests_dir = current_dir()?.join("tests");

    let test_app_name = "TestApp";
    run_test_app(&tests_dir, &test_app_name, &sources, &extra_compile_args)
}
