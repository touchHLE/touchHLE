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
//! - A "guest function" is emulated Arm code, usually from the app binary.
//! - A "host function" is a Rust function that is part of this emulator.

// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]

mod abi;
mod bundle;
mod cpu;
mod dyld;
mod image;
mod mach_o;
mod memory;
mod objc;
mod stack;
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

    let mut env = Environment::new(bundle_path)?;
    env.run();
    Ok(())
}

/// The struct containing the entire emulator state.
pub struct Environment {
    _bundle: bundle::Bundle,
    window: window::Window,
    mem: memory::Memory,
    executable: mach_o::MachO,
    dyld: dyld::Dyld,
    cpu: cpu::Cpu,
}

impl Environment {
    /// Loads the binary and sets up the emulator.
    fn new(bundle_path: PathBuf) -> Result<Environment, String> {
        let bundle = match bundle::Bundle::from_host_path(bundle_path) {
            Ok(bundle) => bundle,
            Err(err) => {
                return Err(format!("Application bundle error: {}. Check that the path is to a .app directory. If this is a .ipa file, you need to extract it as a ZIP file to get the .app directory.", err));
            }
        };

        let icon = image::Image::from_file(bundle.icon_path())
            .map_err(|_| "Could not load icon".to_string())?;
        let launch_image = image::Image::from_file(bundle.launch_image_path()).ok();

        let window = window::Window::new(
            &format!("{} (touchHLE)", bundle.display_name()),
            icon,
            launch_image,
        );

        let mut mem = memory::Memory::new();

        let executable = mach_o::MachO::load_from_file(bundle.executable_path(), &mut mem)
            .map_err(|e| format!("Could not load executable: {}", e))?;

        let entry_point_addr = executable.entry_point_addr.ok_or_else(|| {
            "Mach-O file has no 'start' symbol, perhaps it is not an executable?".to_string()
        })?;

        println!("Address of start function: {:#x}", entry_point_addr);

        let dyld = dyld::Dyld::new();
        dyld.do_initial_linking(&executable, &mut mem);

        let mut cpu = cpu::Cpu::new();

        {
            // FIXME: use actual app name
            let fake_guest_path =
                "/User/Applications/00000000-0000-0000-0000-000000000000/Foo.app/Foo";
            let fake_guest_path_apple_key =
                "executable_path=/User/Applications/00000000-0000-0000-0000-000000000000/Foo.app/Foo";

            let argv = &[fake_guest_path];
            let envp = &[];
            let apple = &[fake_guest_path_apple_key];
            stack::prep_stack_for_start(&mut mem, &mut cpu, argv, envp, apple);
        }

        println!("CPU emulation begins now.");

        cpu.regs_mut()[cpu::Cpu::PC] = entry_point_addr;

        Ok(Environment {
            _bundle: bundle,
            window,
            mem,
            executable,
            dyld,
            cpu,
        })
    }

    /// Run the emulator. This is the main loop, it won't return until app exit.
    fn run(&mut self) {
        let mut events = Vec::new(); // re-use each iteration for efficiency
        loop {
            self.window.poll_for_events(&mut events);
            for event in events.drain(..) {
                match event {
                    window::Event::Quit => {
                        println!("User requested quit, exiting.");
                        return;
                    }
                }
            }

            let mut ticks = 100;
            while ticks > 0 {
                // I'm not sure if this actually is unwind-safe, but considering
                // the emulator will always crash, maybe this is okay.
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    match self.cpu.run(&mut self.mem, &mut ticks) {
                        cpu::CpuState::Normal => (),
                        cpu::CpuState::Svc(svc) => {
                            // the program counter is one instruction ahead
                            let current_instruction = self.cpu.regs()[cpu::Cpu::PC] - 4;
                            let f = self.dyld.get_svc_handler(
                                &self.executable,
                                &mut self.mem,
                                current_instruction,
                                svc,
                            );
                            f.call_from_guest(self);
                        }
                    }
                }));
                if let Err(e) = res {
                    eprintln!(
                        "Panic at PC {:#x}, LR {:#x}",
                        self.cpu.regs()[cpu::Cpu::PC],
                        self.cpu.regs()[cpu::Cpu::LR]
                    );
                    std::panic::resume_unwind(e);
                }
            }
            println!("{} ticks elapsed", ticks);
        }
    }
}
