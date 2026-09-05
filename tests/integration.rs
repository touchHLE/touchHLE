use std::env;
use std::env::current_dir;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
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

/// Makes a path object, and checks that it exists.
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
            path.last().unwrap(),
            env::consts::EXE_SUFFIX
        ));
        println!("{}", buf.iter().last().unwrap().display())
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

fn build_object<I: Iterator<Item = P>, P: AsRef<OsStr>>(
    tests_dir: &Path,
    output_name: &Path,
    sources: I,
    extra_compile_args: &[&str],
) -> Result<(), Box<dyn Error>> {
    let clang_path = make_path_and_check(tests_dir, &["llvm", "bin", "clang"], "Clang", true);

    let bin_path = make_path_and_check(
        tests_dir,
        &["common-3.0.sdk", "usr", "bin"],
        "binary directory",
        false,
    );

    let sdk_path = make_path_and_check(tests_dir, &["common-3.0.sdk"], "SDK sysroot", false);

    let mut linker_arg = OsString::from("-B");
    linker_arg.push(bin_path);
    let mut sdk_arg = OsString::from("--sysroot=");
    sdk_arg.push(sdk_path);

    eprintln!("Building {} for iPhone OS 3...", output_name.display());
    std::io::stderr().flush().unwrap();
    let mut cmd = Command::new(clang_path);
    let output = cmd
        // Uncomment for verbose output (useful for debugging search path
        // issues)
        // .arg("-v")
        // Uncomment for verbose linker output
        // .arg("-Wl,-v")
        // Target iPhone OS 2
        .arg("--target=arm-apple-ios")
        .arg("-miphoneos-version-min=2.0")
        .args(["-arch", "armv6", "-arch", "armv7"])
        // If enabled, the stack protection causes a null pointer crash in some
        // functions. This is probably because ___stack_chk_guard isn't linked.
        .arg("-fno-stack-protector")
        .arg("-DPRODUCT_iPhone")
        // This prevents clang from attempting to link clang_rt on macOS.
        .arg("-nodefaultlibs")
        .arg(linker_arg)
        .arg(sdk_arg)
        .args(extra_compile_args)
        // Input files.
        .args(sources)
        // Write the output to the bundle.
        .arg("-o")
        .arg(output_name)
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
    extra_compile_args: &[&str],
    extra_run_args: &[&str],
) -> Result<(), Box<dyn Error>> {
    let test_app_path = tests_dir.join(format!("{}.app", test_app_name));

    let source_path = tests_dir.join(format!("{}_source", test_app_name));
    let all_sources: Vec<PathBuf> = std::fs::read_dir(&source_path)
        .unwrap()
        .map(|entry| PathBuf::from(entry.unwrap().file_name()))
        .filter(|filename| {
            filename
                .extension()
                .is_some_and(|ext| ext == "m" || ext == "c" || ext == "cpp")
        })
        .map(|entry| source_path.join(entry))
        .collect();

    // C++ files must be compiled separately because the -ObjC flag (needed
    // for Objective-C linking) forces all files to be treated as Objective-C.
    let mut cpp_objects = Vec::new();
    let (cpp_sources, sources): (Vec<_>, Vec<_>) = all_sources
        .into_iter()
        .partition(|p| p.extension().is_some_and(|ext| ext == "cpp"));
    for cpp_src in &cpp_sources {
        let obj_path = tests_dir
            .join(format!("{}.app", test_app_name))
            .join(cpp_src.file_stem().unwrap())
            .with_extension("o");
        // Compile C++ without -ObjC, -fno-objc-exceptions, etc.
        let mut cpp_args: Vec<&str> = extra_compile_args
            .iter()
            .copied()
            .filter(|a| {
                *a != "-ObjC"
                    && *a != "-fno-objc-exceptions"
                    && *a != "-fno-objc-arc"
                    && *a
                        != "-fno-objc-arc-e
          +xceptions"
            })
            .collect();
        cpp_args.push("-c");
        build_object(
            tests_dir,
            &obj_path,
            [cpp_src.as_path()].into_iter(),
            &cpp_args,
        )?;
        cpp_objects.push(obj_path);
    }

    build_object(
        &tests_dir,
        &tests_dir
            .join(format!("{}.app", test_app_name))
            .join(test_app_name),
        sources.iter().chain(cpp_objects.iter()),
        extra_compile_args,
    )?;
    let binary_name = "touchHLE";
    let binary_path = target_dir().join(format!("{}{}", binary_name, env::consts::EXE_SUFFIX));
    let mut cmd = Command::new(binary_path);
    let output = cmd
        .arg(test_app_path)
        // headless mode avoids a distracting window briefly appearing during
        // testing, and works in CI.
        .arg("--headless")
        .args(extra_run_args)
        // Run the automated CLI tests, rather than the manual UIKit tests.
        .arg("--args")
        .arg("--cli-tests")
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
    write!(
        &mut std::io::stdout(),
        "Finished running {}.\n\n\n",
        test_app_name
    )
    .unwrap();
    Ok(())
}

/// Recursively copy a directory's content to a destination skipping symlinks
fn copy_dir_all(source: PathBuf, destination: PathBuf) -> std::io::Result<()> {
    std::fs::create_dir_all(&destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let entry_type = entry.file_type()?;

        if entry_type.is_file() {
            std::fs::copy(entry.path(), destination.join(entry.file_name()))?;
        } else if entry_type.is_dir() {
            copy_dir_all(entry.path(), destination.join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[test]
fn test_app() -> Result<(), Box<dyn Error>> {
    let tests_dir = current_dir()?.join("tests");
    let stubs_dir = tests_dir.join("stubs");
    let stubs_src_dir = stubs_dir.join("src");
    let stubs_lib_dir = stubs_dir.join("lib");
    let stubs_frameworks_dir = stubs_dir.join("Frameworks");

    // Wipe the stubs dir to ensure it is clean.
    let _ = std::fs::remove_dir_all(&stubs_dir);
    // Create the stubs dir
    std::fs::create_dir(&stubs_dir).unwrap();

    let bundled_libs_search_arg =
        "-L".to_owned() + current_dir()?.join("touchHLE_dylibs").to_str().unwrap();
    let stubs_lib_search_arg = "-L".to_owned() + stubs_lib_dir.to_str().unwrap();
    let stubs_frameworks_search_arg = "-F".to_owned() + stubs_frameworks_dir.to_str().unwrap();
    let mut extra_linker_args = Vec::<String>::new();
    let mut extra_compile_args = vec![
        "-mlinker-version=253",
        "-Wno-expansion-to-defined",
        "-Wno-literal-range",
        bundled_libs_search_arg.as_str(),
        stubs_lib_search_arg.as_str(),
        stubs_frameworks_search_arg.as_str(),
        "-ObjC",
        "-fno-objc-exceptions",
        // ARC is not available until IOS 5, so it can't be used.
        "-fno-objc-arc",
        "-fno-objc-arc-exceptions",
    ];

    // Generate symbol stubs.

    let symbols_path = stubs_dir.join("SYMBOLS.txt");
    let dump_file_option = format!("--dump-file={}", symbols_path.to_str().unwrap());
    let dump_run_args = ["--dump=symbols", dump_file_option.as_str(), "--headless"];
    let binary_name = "touchHLE";
    let binary_path = target_dir().join(format!("{}{}", binary_name, env::consts::EXE_SUFFIX));
    let mut cmd = Command::new(binary_path);
    let output = cmd
        .args(dump_run_args)
        .output()
        .expect("failed to execute touchHLE process");
    assert!(output.status.success());

    // Split SYMBOLS.txt into individual source files.

    std::fs::create_dir(&stubs_src_dir).unwrap();
    let mut files_to_compile = Vec::<(String, PathBuf)>::new();
    {
        let mut in_body = false;
        let mut current_file = None::<BufWriter<File>>;
        for line in BufReader::new(File::open(symbols_path).unwrap()).lines() {
            let line = line.unwrap();
            if let Some(dylib_path) = line.strip_prefix("// ") {
                // First comment after a series of non-comment lines, or first
                // comment in the file: this is the canonical name of the dylib.
                if in_body || current_file.is_none() {
                    let dylib_name = dylib_path.rsplit_once("/").unwrap().1;
                    let stub_src_path = stubs_src_dir.join(format!("{}.m", dylib_name));
                    current_file = Some(BufWriter::new(File::create(&stub_src_path).unwrap()));
                    files_to_compile.push((dylib_path.to_string(), stub_src_path));
                    in_body = false;
                }
                // Ignore the non-canonical dylib names for now.
            } else if let Some(ref mut current_file) = current_file {
                in_body = true;
                writeln!(current_file, "{}", line).unwrap();
            }
        }
        // current_file dropping out of scope here flushes it.
    }

    // Build the stub libraries and ensure TestApp will link to them.

    for (dylib_path, stub_src_path) in files_to_compile {
        if dylib_path.starts_with("/.touchHLE") {
            // skip the fake app picker library
            continue;
        }
        let compile_args = [
            "-mlinker-version=253",
            "-fno-builtin",
            "-nostdlib",
            &format!("-Wl,-install_name,{}", dylib_path),
            "-Wno-objc-root-class", // silence clang warning about inheritance
            "-Wl,-dylib",
            stubs_lib_search_arg.as_str(),
            "-lobjc.A",
        ];
        let dylib_name = dylib_path.rsplit_once("/").unwrap().1;
        // - The only non-framework libs should be libSystem and libobjc, which
        //   we expect to appear in the list before all the frameworks, and need
        //   to be compiled first.
        // - The frameworks have bare filenames, and usually need libobjc.
        let (compile_args, out_path) =
            if let Some(framework_path) = dylib_path.strip_prefix("/System/Library/Frameworks/") {
                extra_linker_args.push(format!("-framework"));
                extra_linker_args.push(dylib_name.to_string());
                (&compile_args[..], stubs_frameworks_dir.join(framework_path))
            } else {
                extra_linker_args.push(format!(
                    "-l{}",
                    dylib_name
                        .strip_prefix("lib")
                        .unwrap()
                        .strip_suffix(".dylib")
                        .unwrap()
                ));
                (
                    // skip "-Ltests/stubs/lib/" and "-lobjc.A"
                    &compile_args[..compile_args.len() - 2],
                    stubs_lib_dir.join(dylib_name),
                )
            };
        // Ensure that stubs/Frameworks/FooBarKit/ or stubs/lib/ exists
        std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();

        // Ideally one could provide two framework search paths.
        // In reality only the first matching path is used, so the stub
        // framework needs to provide headers copied from the sdk.
        if dylib_path.starts_with("/System/Library/Frameworks") {
            let stub_headers = out_path.parent().unwrap().join("Headers");
            let framework_dir = dylib_path.rsplit_once("/").unwrap().0;
            let source_headers = tests_dir.join(format!("common-3.0.sdk/{framework_dir}/Headers"));

            if source_headers.exists() {
                copy_dir_all(source_headers, stub_headers)?;
            }
        }
        build_object(&tests_dir, &out_path, [stub_src_path].iter(), &compile_args).unwrap();
    }

    // Link against libstdc++ to support C++ test sources (virtual inheritance
    // tests etc). The bundled libstdc++ dylib path is already in the search
    // path via bundled_libs_search_arg.
    extra_compile_args.push("-lstdc++.6.0.9");

    // Vec<String> -> &[&str] ownership shenanigans
    for arg in &extra_linker_args {
        extra_compile_args.push(&arg);
    }

    // Finally, build TestApp itself.
    run_test_app(&tests_dir, "TestApp", &extra_compile_args, &[])
}
