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
mod frameworks;
mod image;
mod libc;
mod mach_o;
mod mem;
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

/// Note that currently only a single thread (the main thread) is supported.
type ThreadID = u32;

/// The struct containing the entire emulator state.
pub struct Environment {
    bundle: bundle::Bundle,
    window: window::Window,
    mem: mem::Mem,
    /// Loaded binaries. Index `0` is always the app binary, other entries are
    /// dynamic libraries.
    bins: Vec<mach_o::MachO>,
    objc: objc::ObjC,
    dyld: dyld::Dyld,
    cpu: cpu::Cpu,
    current_thread: ThreadID,
    libc_state: libc::State,
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

        let mut mem = mem::Mem::new();

        let executable = mach_o::MachO::load_from_file(bundle.executable_path(), &mut mem)
            .map_err(|e| format!("Could not load executable: {}", e))?;

        let mut dylibs = Vec::new();
        for dylib in &executable.dynamic_libraries {
            match &**dylib {
                // We have host implementations of these
                "/usr/lib/libSystem.B.dylib" | "/usr/lib/libobjc.A.dylib" => continue,
                // Free Software bundled with touchHLE
                "/usr/lib/libgcc_s.1.dylib"
                | "/usr/lib/libstdc++.6.dylib"
                | "/usr/lib/libstdc++.6.0.4.dylib" => {
                    let dylib = dylib.strip_prefix("/usr/lib/").unwrap();
                    // Resolve symlink
                    let dylib = if dylib == "libstdc++.6.dylib" {
                        "libstdc++.6.0.4.dylib"
                    } else {
                        dylib
                    };

                    let dylib = mach_o::MachO::load_from_file(
                        PathBuf::from("dylibs").join(dylib),
                        &mut mem,
                    )
                    .map_err(|e| format!("Could not load bundled dylib: {}", e))?;
                    dylibs.push(dylib);
                }
                _ => {
                    // System frameworks will have host implementations.
                    // TODO: warn about unimplemented frameworks?
                    if !dylib.starts_with("/System/Library/Frameworks/") {
                        eprintln!(
                            "Warning: app binary depends on unexpected dylib \"{}\"",
                            dylib
                        );
                    }
                }
            }
        }

        let entry_point_addr = *executable.exported_symbols.get("start").ok_or_else(|| {
            "Mach-O file has no 'start' symbol, perhaps it is not an executable?".to_string()
        })?;
        let entry_point_addr = abi::GuestFunction::from_addr_with_thumb_bit(entry_point_addr);

        println!("Address of start function: {:?}", entry_point_addr);

        let mut bins = dylibs;
        bins.insert(0, executable);

        let mut objc = objc::ObjC::new();

        let mut dyld = dyld::Dyld::new();
        dyld.do_initial_linking(&bins, &mut mem, &mut objc);

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

        // FIXME: call various static initializers. libstdc++ in particular has
        // lots of these and eventually we'll hit something that breaks if they
        // aren't run.

        cpu.set_cpsr(cpu::Cpu::CPSR_USER_MODE);
        cpu.branch(entry_point_addr);

        Ok(Environment {
            bundle,
            window,
            mem,
            bins,
            objc,
            dyld,
            cpu,
            current_thread: 0,
            libc_state: Default::default(),
        })
    }

    /// Run the emulator. This is the main loop and won't return until app exit.
    /// Only `main.rs` should call this.
    fn run(&mut self) {
        // I'm not sure if this actually is unwind-safe, but considering
        // the emulator will crash anyway, maybe this is okay.
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.run_inner(true)));
        if let Err(e) = res {
            eprintln!(
                "Panic at PC {:#x}, LR {:#x}",
                self.cpu.regs()[cpu::Cpu::PC],
                self.cpu.regs()[cpu::Cpu::LR]
            );
            std::panic::resume_unwind(e);
        }
    }

    /// Run the emulator until the app returns control to the host. This is for
    /// host-to-guest function calls (see [abi::GuestFunction::call]).
    pub fn run_call(&mut self) {
        self.run_inner(false)
    }

    fn run_inner(&mut self, root: bool) {
        let mut events = Vec::new(); // re-use each iteration for efficiency
        loop {
            self.window.poll_for_events(&mut events);
            #[allow(clippy::never_loop)]
            for event in events.drain(..) {
                #[allow(clippy::single_match)]
                match event {
                    window::Event::Quit => {
                        println!("User requested quit, exiting.");
                        if root {
                            return;
                        } else {
                            panic!("Quit.");
                        }
                    }
                }
            }

            let mut ticks = 100;
            while ticks > 0 {
                match self.cpu.run(&mut self.mem, &mut ticks) {
                    cpu::CpuState::Normal => (),
                    cpu::CpuState::Svc(svc) => {
                        // the program counter is pointing at the
                        // instruction after the SVC, but we want the
                        // address of the SVC itself
                        let svc_pc = self.cpu.regs()[cpu::Cpu::PC] - 4;
                        if svc == dyld::Dyld::SVC_RETURN_TO_HOST {
                            assert!(!root);
                            assert!(
                                svc_pc
                                    == self.dyld.return_to_host_routine().addr_without_thumb_bit()
                            );
                            return;
                        }

                        if let Some(f) = self.dyld.get_svc_handler(
                            &self.bins,
                            &mut self.mem,
                            &mut self.cpu,
                            svc_pc,
                            svc,
                        ) {
                            f.call_from_guest(self)
                        } else {
                            self.cpu.regs_mut()[cpu::Cpu::PC] = svc_pc;
                        }
                    }
                }
            }
        }
    }
}
