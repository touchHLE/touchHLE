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

fn build_test_app(tests_dir: &Path, test_app_path: &Path) -> Result<(), Box<dyn Error>> {
    let clang_path = tests_dir
        .join("llvm")
        .join("bin")
        .join(format!("clang{}", env::consts::EXE_SUFFIX));

    if !clang_path.exists() {
        panic!(
            "Couldn't find Clang at {}. Please see {} for more details.",
            clang_path.display(),
            tests_dir.join("README.md").display()
        );
    }

    let test_bin_path = test_app_path.join("TestApp");

    eprintln!("Building {} for iPhone OS 2...", test_bin_path.display());

    let mut cmd = Command::new(clang_path);

    let output = cmd
        // Use upstream LLVM linker (not system linker)
        .arg("-fuse-ld=lld")
        // On macOS only, Clang tries to use flags that ld64.lld doesn't
        // support. Perhaps it's confused and thinks it's invoking Apple's ld64?
        // Telling it not to use newer flags like this seems to avoid this, but
        // I suspect there may be a better fix.
        .arg("-mlinker-version=0")
        // Target iPhone OS 2
        .args(["-target", "armv6-apple-ios2"])
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
        // Input
        .arg(tests_dir.join("TestApp_source").join("main.c"))
        // Write the output to the bundle.
        .arg("-o")
        .arg(&test_bin_path)
        .output()
        .expect("failed to execute Clang process");

    std::io::stdout().write_all(&output.stdout).unwrap();
    std::io::stderr().write_all(&output.stderr).unwrap();

    assert!(output.status.success());

    eprintln!("Built successfully.");

    Ok(())
}

#[test]
fn run_test_app() -> Result<(), Box<dyn Error>> {
    let tests_dir = current_dir()?.join("tests");

    let test_app_path = tests_dir.join("TestApp.app");

    build_test_app(&tests_dir, &test_app_path)?;

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
