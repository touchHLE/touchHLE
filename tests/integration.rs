use std::env;
use std::env::current_dir;
use std::io::Write;
use std::process::Command;

// adapted from `assert_cmd` crate
fn target_dir() -> std::path::PathBuf {
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

#[test]
fn run_test_app() -> Result<(), Box<dyn std::error::Error>> {
    let binary_name = "touchHLE";
    let binary_path = target_dir().join(format!("{}{}", binary_name, env::consts::EXE_SUFFIX));

    let mut cmd = Command::new(binary_path);

    let mut test_app_path = current_dir()?;
    test_app_path.push("tests");
    test_app_path.push("TestApp.app");

    let output = cmd
        .arg(test_app_path)
        .output()
        .expect("failed to execute process");

    std::io::stdout().write_all(&output.stdout).unwrap();
    std::io::stderr().write_all(&output.stderr).unwrap();

    assert!(output.status.success());
    // sanity check: check that emulation actually happened
    assert_ne!(
        find_subsequence(output.stdout.as_slice(), b"CPU emulation begins now."),
        None
    );

    Ok(())
}
