// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]

mod bundle;
mod image;
mod mach_o;
mod window;

use std::path::PathBuf;

const USAGE: &str = "\
Usage:
    touchHLE path/to/some.app

Options:
    --help
        Print this help text.
";

fn main() -> Result<(), String> {
    let mut args = std::env::args();
    let _ = args.next().unwrap(); // skip argv[0]

    let mut bundle_path: Option<PathBuf> = None;
    for arg in args {
        if arg == "--help" {
            println!("{}", USAGE);
            return Ok(());
        } else if bundle_path.is_none() {
            bundle_path = Some(PathBuf::from(arg));
        } else {
            eprintln!("{}", USAGE);
            return Err(format!("Unexpected argument: {:?}", arg));
        }
    }

    let Some(bundle_path) = bundle_path else {
        eprintln!("{}", USAGE);
        return Err("Path to bundle must be specified".to_string());
    };

    let bundle = match bundle::Bundle::from_host_path(bundle_path) {
        Ok(bundle) => bundle,
        Err(err) => {
            return Err(format!("Application bundle error: {}. Check that the path is to a .app directory. If this is a .ipa file, you need to extract it as a ZIP file to get the .app directory.", err));
        }
    };

    let icon = image::Image::from_file(bundle.icon_path())
        .map_err(|_| "Could not load icon".to_string())?;
    let launch_image = image::Image::from_file(bundle.launch_image_path()).ok();

    let mut window = window::Window::new(
        &format!("{} (touchHLE)", bundle.display_name()),
        icon,
        launch_image,
    );

    let _mach_o = mach_o::MachO::from_file(bundle.executable_path())
        .map_err(|e| format!("Could not load executable: {}", e))?;

    let mut events = Vec::new(); // re-use each iteration for efficiency
    loop {
        window.poll_for_events(&mut events);
        for event in events.drain(..) {
            match event {
                window::Event::Quit => {
                    println!("User requested quit, exiting.");
                    return Ok(());
                }
            }
        }

        // TODO: emulation
    }
}
