use std::env;
use std::env::current_dir;
use std::error::Error;
use std::ffi::OsString;
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
    fn make_path_and_check(
        tests_dir: &Path,
        path: &[&str],
        object: &str,
        is_executable: bool,
    ) -> PathBuf {
        let mut buf = tests_dir.to_path_buf();
        if is_executable {
            for part in &path[..(path.len() - 1)] {
                buf.push(part);
            }
            buf.push(format!(
                "{}{}",
                path[path.len() - 1],
                env::consts::EXE_SUFFIX
            ));
        } else {
            for part in path {
                buf.push(part);
            }
        }

        if !buf.exists() {
            panic!(
                "Couldn't find {} at {}. Please see {} for more details.",
                object,
                buf.display(),
                tests_dir.join("README.md").display()
            );
        }
        buf
    }
    let clang_path = make_path_and_check(tests_dir, &["llvm", "bin", "clang"], "Clang", true);

    let linker_path = make_path_and_check(
        tests_dir,
        &["common-3.0-sdk", "common-3.0.sdk", "usr", "bin", "ld"],
        "ld",
        true,
    );

    let sdk_path = make_path_and_check(
        tests_dir,
        &["common-3.0-sdk", "common-3.0.sdk"],
        "SDK sysroot",
        true,
    );

    let sdk_lib_path = make_path_and_check(
        tests_dir,
        &["common-3.0-sdk", "common-3.0.sdk", "usr", "lib"],
        "SDK lib directory",
        true,
    );

    let mut linker_arg = OsString::from("-fuse-ld=");
    linker_arg.push(linker_path);
    let mut sdk_arg = OsString::from("--sysroot=");
    sdk_arg.push(sdk_path);
    let mut libs_arg = OsString::from("-L");
    libs_arg.push(sdk_lib_path);

    let test_bin_path = tests_dir
        .join(format!("{}.app", test_app_name))
        .join(test_app_name);
    eprintln!("Building {} for iPhone OS 3...", test_bin_path.display());
    std::io::stderr().flush().unwrap();
    let mut cmd = Command::new(clang_path);
    let output = cmd
        // Uncomment for verbose output (useful for debugging search path
        // issues)
        .arg("-v")
        // Uncomment for verbose linker output
        //.arg("-Wl,-v")
        // Target iPhone OS 2
        .args(["-target", "armv6-apple-ios3"])
        // Don't search the deafult MacOS directories.
        .arg("-Z")
        // If enabled, the stack protection causes a null pointer crash in some
        // functions. This is probably because ___stack_chk_guard isn't linked.
        .arg("-fno-stack-protector")
        .arg("-DPRODUCT_iPhone")
        .arg(linker_arg)
        .arg(libs_arg)
        .arg(sdk_arg)
        .args(extra_compile_args)
        // Input files.
        .args(sources.iter().map(|file| {
            tests_dir
                .join(format!("{}_source", test_app_name))
                .join(file)
        }))
        // Write the output to the bundle.
        .arg("-o")
        .arg(test_bin_path)
        .output()
        .expect("failed to execute Clang process");
    eprintln!("Running {:?}", cmd);
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
    let libs_dir = "-L".to_owned() + current_dir()?.join("touchHLE_dylibs").to_str().unwrap();
    let extra_compile_args = [
        "-Wno-expansion-to-defined",
        libs_dir.as_str(),
        "-ObjC",
        "-fno-objc-exceptions",
        // ARC is not available until IOS 5, so it can't be used.
        "-fno-objc-arc",
        "-fno-objc-arc-exceptions",
    ];
    let sources = ["main.m", "SyncTester.m"].map(|file| Path::new(file));
    let tests_dir = current_dir()?.join("tests");
    let test_app_name = "TestApp";
    run_test_app(&tests_dir, &test_app_name, &sources, &extra_compile_args)
}
