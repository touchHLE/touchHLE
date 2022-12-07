//! touchHLE is a high-level emulator (HLE) for iPhone OS applications.
//!
//! In various places, the terms "guest" and "host" are used to distinguish
//! between the emulated application (the "guest") and the emulator itself (the
//! "host"), and more generally, their different environments.
//! For example:
//! - The guest is a 32-bit application, so a "guest pointer" is 32 bits.
//! - The host is a 64-bit application, so a "host pointer" is 64 bits.
//! - The guest can only directly access "guest memory".
//! - The host can access both "guest memory" and "host memory".

// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]

mod bundle;
mod cpu;
mod image;
mod mach_o;
mod memory;
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

    let mut mem = memory::Memory::new();

    let mach_o = mach_o::MachO::load_from_file(bundle.executable_path(), &mut mem)
        .map_err(|e| format!("Could not load executable: {}", e))?;

    let entry_point_addr = mach_o.entry_point_addr.ok_or(
        "Mach-O file has no 'start' symbol, perhaps it is not an executable?".to_string(),
    )?;

    println!("Address of start function: {:#x}", entry_point_addr);

    let mut cpu = cpu::Cpu::new();

    // Basic integration test, TODO: run the actual app code
    mem.write(memory::Ptr::from_bits(0), 0xE0800001u32); // A32: add r0, r0, r1
    mem.write(memory::Ptr::from_bits(4), 0xEF000001u32); // A32: svc 0
    let a = 1;
    let b = 2;
    cpu.regs_mut()[0] = a;
    cpu.regs_mut()[1] = b;
    cpu.regs_mut()[15] = 0; // PC = 0
    cpu.run(&mut mem);
    let res = cpu.regs()[0];
    println!("According to dynarmic, {} + {} = {}!", a, b, res);

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
