/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
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

#[macro_use]
mod log;
mod abi;
mod audio;
mod bundle;
mod cpu;
mod dyld;
mod font;
mod frameworks;
mod fs;
mod image;
mod libc;
mod licenses;
mod mach_o;
mod mem;
mod objc;
mod options;
mod stack;
mod window;

use std::path::PathBuf;

/// Current version. See `build.rs` for how this is generated.
const VERSION: &str = include_str!(concat!(env!("OUT_DIR"), "/version.txt"));

const USAGE: &str = "\
Usage:
    touchHLE path/to/some.app

General options:
    --help
        Display this help text.

    --copyright
        Display copyright, authorship and license information.

    --info
        Print basic information about the app bundle without running the app.
";

fn main() -> Result<(), String> {
    println!("touchHLE {} — https://touchhle.org/", VERSION);
    println!();

    let mut args = std::env::args();
    let _ = args.next().unwrap(); // skip argv[0]

    let mut bundle_path: Option<PathBuf> = None;
    let mut just_info = false;
    let mut option_args = Vec::new();

    for arg in args {
        if arg == "--help" {
            println!("{}", USAGE);
            println!("{}", options::DOCUMENTATION);
            return Ok(());
        } else if arg == "--copyright" {
            licenses::print();
            return Ok(());
        } else if arg == "--info" {
            just_info = true;
        // Parse an option but discard the value, to test whether it's valid.
        // We don't want to apply it immediately, because then options loaded
        // from a file would take precedence over options from the command line.
        } else if options::Options::default().parse_argument(&arg)? {
            option_args.push(arg);
        } else if bundle_path.is_none() {
            bundle_path = Some(PathBuf::from(arg));
        } else {
            eprintln!("{}", USAGE);
            eprintln!("{}", options::DOCUMENTATION);
            return Err(format!("Unexpected argument: {:?}", arg));
        }
    }

    let Some(bundle_path) = bundle_path else {
        eprintln!("{}", USAGE);
        return Err("Path to bundle must be specified".to_string());
    };

    // When PowerShell does tab-completion on a directory, for some reason it
    // expands it to `'..\My Bundle.app\'` and that trailing \ seems to
    // get interpreted as escaping a double quotation mark?
    #[cfg(windows)]
    if let Some(fixed) = bundle_path.to_str().and_then(|s| s.strip_suffix('"')) {
        log!("Warning: The bundle path has a trailing quotation mark! This often happens accidentally on Windows when tab-completing, because '\\\"' gets interpreted by Rust in the wrong way. Did you meant to write {:?}?", fixed);
    }

    let bundle_data = fs::BundleData::open_any(&bundle_path)
        .map_err(|e| format!("Could not open app bundle: {e}"))?;
    let (bundle, fs) = match bundle::Bundle::new_bundle_and_fs_from_host_path(bundle_data) {
        Ok(bundle) => bundle,
        Err(err) => {
            return Err(format!("Application bundle error: {err}. Check that the path is to an .app directory or an .ipa file."));
        }
    };

    let app_id = bundle.bundle_identifier();

    println!("App bundle info:");
    println!("- Display name: {}", bundle.display_name());
    println!("- Version: {}", bundle.bundle_version());
    println!("- Identifier: {}", app_id);
    println!("- Internal name: {}.app", bundle.canonical_bundle_name());
    println!();

    if just_info {
        return Ok(());
    }

    let mut options = options::Options::default();

    // Apply options from files
    for filename in [options::DEFAULTS_FILENAME, options::USER_FILENAME] {
        match options::get_options_from_file(filename, app_id) {
            Ok(Some(options_string)) => {
                println!(
                    "Using options from {} for this app: {}",
                    filename, options_string
                );
                for option_arg in options_string.split_ascii_whitespace() {
                    match options.parse_argument(option_arg) {
                        Ok(true) => (),
                        Ok(false) => return Err(format!("Unknown option {:?}", option_arg)),
                        Err(err) => {
                            return Err(format!("Invalid option {:?}: {}", option_arg, err))
                        }
                    }
                }
            }
            Ok(None) => {
                println!("No options found for this app in {}", filename);
            }
            Err(e) => {
                eprintln!("Warning: {}", e);
            }
        }
    }
    println!();

    // Apply command-line options
    for option_arg in option_args {
        let parse_result = options.parse_argument(&option_arg);
        assert!(parse_result == Ok(true));
    }

    let mut env = Environment::new(bundle, fs, options)?;
    env.run();
    Ok(())
}

/// Index into the [Vec] of threads. Thread 0 is always the main thread.
type ThreadID = usize;

/// Bookkeeping for a thread.
struct Thread {
    /// Once a thread finishes, this is set to false.
    active: bool,
    /// Set to [true] when a thread is running its startup routine (i.e. the
    /// function pointer passed to `pthread_create`). When it returns to the
    /// host, it should become inactive.
    in_start_routine: bool,
    /// Set to [true] when a thread is currently waiting for a host function
    /// call to return.
    ///
    /// This is needed when a guest function calls a host function, and that
    /// host function calls a guest function on a different thread. While
    /// executing the function on the other thread, [Environment::run_inner]
    /// must ensure it does not switch back to the original thread and execute
    /// guest code, as that thread is still waiting for the host function to
    /// return.
    ///
    /// A host function that is being waited for can call back into guest code
    /// on the same thread, in which case this will be set to [false] for the
    /// duration of that call. This flag only indicates that the top-most "stack
    /// frame" of the thread is a host function, not whether there are any host
    /// functions at all.
    in_host_function: bool,
    /// Context object containing the CPU state for this thread.
    ///
    /// There should always be `(threads.len() - 1)` contexts in existence.
    /// When a thread is currently executing, its state is stored directly in
    /// the CPU, rather than in a context object. In that case, this field is
    /// None. See also: [std::mem::take] and [cpu::Cpu::swap_context].
    context: Option<cpu::CpuContext>,
    /// Address range of this thread's stack, used to check if addresses are in
    /// range while producing a stack trace.
    stack: Option<std::ops::RangeInclusive<u32>>,
}

/// The struct containing the entire emulator state.
pub struct Environment {
    /// Reference point for various timing functions.
    startup_time: std::time::Instant,
    bundle: bundle::Bundle,
    fs: fs::Fs,
    window: window::Window,
    mem: mem::Mem,
    /// Loaded binaries. Index `0` is always the app binary, other entries are
    /// dynamic libraries.
    bins: Vec<mach_o::MachO>,
    objc: objc::ObjC,
    dyld: dyld::Dyld,
    cpu: cpu::Cpu,
    current_thread: ThreadID,
    threads: Vec<Thread>,
    libc_state: libc::State,
    framework_state: frameworks::State,
    options: options::Options,
}

impl Environment {
    /// Loads the binary and sets up the emulator.
    fn new(
        bundle: bundle::Bundle,
        fs: fs::Fs,
        options: options::Options,
    ) -> Result<Environment, String> {
        let startup_time = std::time::Instant::now();

        let icon = fs
            .read(bundle.icon_path())
            .map_err(|_| "Could not read icon file")
            .and_then(|bytes| {
                image::Image::from_bytes(&bytes).map_err(|_| "Could not parse icon image")
            });
        if let Err(e) = icon {
            log!("Warning: {}", e);
        }

        let launch_image = fs
            .read(bundle.launch_image_path())
            .ok()
            .and_then(|bytes| image::Image::from_bytes(&bytes).ok());

        let window = window::Window::new(
            &format!("{} (touchHLE {})", bundle.display_name(), VERSION),
            icon.ok(),
            launch_image,
            &options,
        );

        let mut mem = mem::Mem::new();

        let executable = mach_o::MachO::load_from_file(bundle.executable_path(), &fs, &mut mem)
            .map_err(|e| format!("Could not load executable: {}", e))?;

        let mut dylibs = Vec::new();
        for dylib in &executable.dynamic_libraries {
            if dylib == "/usr/lib/libSystem.B.dylib" || dylib == "/usr/lib/libobjc.A.dylib" {
                // We have host implementations of these
                continue;
            }

            // There are some Free Software libraries bundled with touchHLE and
            // exposed via the guest file system (see Fs::new()).
            if fs.is_file(fs::GuestPath::new(dylib)) {
                let dylib = mach_o::MachO::load_from_file(fs::GuestPath::new(dylib), &fs, &mut mem)
                    .map_err(|e| format!("Could not load bundled dylib: {}", e))?;
                dylibs.push(dylib);
            } else {
                // System frameworks will have host implementations.
                // TODO: warn about unimplemented frameworks?
                if !dylib.starts_with("/System/Library/Frameworks/") {
                    log!(
                        "Warning: app binary depends on unexpected dylib \"{}\"",
                        dylib
                    );
                }
                continue;
            };
        }

        let entry_point_addr = executable.entry_point_pc.ok_or_else(|| {
            "Mach-O file does not specify an entry point PC, perhaps it is not an executable?"
                .to_string()
        })?;
        let entry_point_addr = abi::GuestFunction::from_addr_with_thumb_bit(entry_point_addr);

        log_dbg!("Address of start function: {:?}", entry_point_addr);

        let mut bins = dylibs;
        bins.insert(0, executable);

        let mut objc = objc::ObjC::new();

        let mut dyld = dyld::Dyld::new();
        dyld.do_initial_linking(&bins, &mut mem, &mut objc);

        for &breakpoint in &options.breakpoints {
            dyld.set_breakpoint(&mut mem, breakpoint);
        }

        let cpu = cpu::Cpu::new(match options.direct_memory_access {
            true => Some(&mut mem),
            false => None,
        });

        let main_thread = Thread {
            active: true,
            in_start_routine: false, // main thread never terminates
            in_host_function: false,
            context: None,
            stack: Some(mem::Mem::MAIN_THREAD_STACK_LOW_END..=0u32.wrapping_sub(1)),
        };

        let mut env = Environment {
            startup_time,
            bundle,
            fs,
            window,
            mem,
            bins,
            objc,
            dyld,
            cpu,
            current_thread: 0,
            threads: vec![main_thread],
            libc_state: Default::default(),
            framework_state: Default::default(),
            options,
        };

        dyld::Dyld::do_late_linking(&mut env);

        {
            let bin_path = env.bundle.executable_path();
            let bin_path_apple_key = format!("executable_path={}", bin_path.as_str());

            let argv = &[bin_path.as_str()];
            let envp = &[];
            let apple = &[bin_path_apple_key.as_str()];
            stack::prep_stack_for_start(&mut env.mem, &mut env.cpu, argv, envp, apple);
        }

        println!("CPU emulation begins now.");

        env.cpu.set_cpsr(cpu::Cpu::CPSR_USER_MODE);

        // Static initializers for libraries must be run before the initializer
        // in the app binary.
        // TODO: once we support more libraries, replace this hard-coded order
        //       with e.g. a topological sort.
        assert!(env.bins.len() <= 3);
        for bin_idx in [1, 2, 0] {
            let Some(bin) = env.bins.get(bin_idx) else {
                continue;
            };
            let Some(mod_init_func) = bin.get_section("__mod_init_func") else {
                continue;
            };

            log_dbg!("Calling static initializers for {:?}", bin.name);
            assert!(mod_init_func.size % 4 == 0);
            let base: mem::ConstPtr<abi::GuestFunction> = mem::Ptr::from_bits(mod_init_func.addr);
            let count = mod_init_func.size / 4;
            for i in 0..count {
                let func = env.mem.read(base + i);
                func.call(&mut env);
            }
            log_dbg!("Static initialization done");
        }

        env.cpu.branch(entry_point_addr);

        Ok(env)
    }

    fn stack_trace(&self) {
        let stack_range = self.threads[self.current_thread].stack.clone().unwrap();
        eprintln!(
            " 0. {:#x} (PC)",
            self.cpu.pc_with_thumb_bit().addr_with_thumb_bit()
        );
        let regs = self.cpu.regs();
        let mut lr = regs[cpu::Cpu::LR];
        let return_to_host_routine_addr = self.dyld.return_to_host_routine().addr_with_thumb_bit();
        if lr == return_to_host_routine_addr {
            eprintln!(" 1. [host function] (LR)");
        } else {
            eprintln!(" 1. {:#x} (LR)", lr);
        }
        let mut i = 2;
        let mut fp: mem::ConstPtr<u8> = mem::Ptr::from_bits(regs[abi::FRAME_POINTER]);
        loop {
            if !stack_range.contains(&fp.to_bits()) {
                eprintln!("Next FP ({:?}) is outside the stack.", fp);
                break;
            }
            lr = self.mem.read((fp + 4).cast());
            fp = self.mem.read(fp.cast());
            if lr == return_to_host_routine_addr {
                eprintln!("{:2}. [host function]", i);
            } else {
                eprintln!("{:2}. {:#x}", i, lr);
            }
            i += 1;
        }
    }

    /// Create a new thread and return its ID. The `start_routine` and
    /// `user_data` arguments have the same meaning as the last two arguments to
    /// `pthread_create`.
    pub fn new_thread(
        &mut self,
        start_routine: abi::GuestFunction,
        user_data: mem::MutVoidPtr,
    ) -> ThreadID {
        let stack_size = mem::Mem::SECONDARY_THREAD_STACK_SIZE;
        let stack_alloc = self.mem.alloc(stack_size);
        let stack_high_addr = stack_alloc.to_bits() + stack_size;
        assert!(stack_high_addr % 4 == 0);

        self.threads.push(Thread {
            active: true,
            in_start_routine: true,
            in_host_function: false,
            context: Some(cpu::CpuContext::new()),
            stack: Some(stack_alloc.to_bits()..=(stack_high_addr - 1)),
        });
        let new_thread_id = self.threads.len() - 1;

        log_dbg!("Created new thread {} with stack {:#x}–{:#x}, will execute function {:?} with data {:?}", new_thread_id, stack_alloc.to_bits(), (stack_high_addr - 1), start_routine, user_data);

        let old_thread = self.current_thread;

        // Switch to the new context (all zeroes) and set up the registers
        // (which we can only do by switching). The original thread's state
        // should be the same as before.
        self.switch_thread(new_thread_id);
        self.cpu.set_cpsr(cpu::Cpu::CPSR_USER_MODE);
        self.cpu.regs_mut()[cpu::Cpu::SP] = stack_high_addr;
        self.cpu.regs_mut()[0] = user_data.to_bits();
        self.cpu
            .branch_with_link(start_routine, self.dyld.return_to_host_routine());
        self.switch_thread(old_thread);

        new_thread_id
    }

    /// Run the emulator. This is the main loop and won't return until app exit.
    /// Only `main.rs` should call this.
    fn run(&mut self) {
        // I'm not sure if this actually is unwind-safe, but considering
        // the emulator will crash anyway, maybe this is okay.
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.run_inner(true)));
        if let Err(e) = res {
            eprintln!("Register state immediately after panic:");
            self.cpu.dump_regs();
            if self.current_thread == 0 {
                eprintln!("Attempting to produce stack trace for main thread:");
            } else {
                eprintln!(
                    "Attempting to produce stack trace for thread {}:",
                    self.current_thread
                );
            }
            self.stack_trace();
            std::panic::resume_unwind(e);
        }
    }

    /// Run the emulator until the app returns control to the host. This is for
    /// host-to-guest function calls (see [abi::GuestFunction::call]).
    ///
    /// Note that this might execute code from other threads while waiting for
    /// the app to return control on the original thread!
    pub fn run_call(&mut self) {
        let was_in_host_function = self.threads[self.current_thread].in_host_function;
        self.threads[self.current_thread].in_host_function = false;
        self.run_inner(false);
        self.threads[self.current_thread].in_host_function = was_in_host_function;
    }

    fn switch_thread(&mut self, new_thread: ThreadID) {
        assert!(new_thread != self.current_thread);

        log_dbg!(
            "Switching thread: {} => {}",
            self.current_thread,
            new_thread
        );

        let mut context = self.threads[new_thread].context.take().unwrap();
        self.cpu.swap_context(&mut context);
        assert!(self.threads[self.current_thread].context.is_none());
        self.threads[self.current_thread].context = Some(context);
        self.current_thread = new_thread;
    }

    fn run_inner(&mut self, root: bool) {
        let initial_thread = self.current_thread;
        assert!(self.threads[initial_thread].active);
        assert!(self.threads[initial_thread].context.is_none());

        loop {
            // We need to poll for events occasionally during CPU execution so
            // that the host OS doesn't consider touchHLE unresponsive.
            // This is not free so we should avoid doing it too often.
            // 100,000 ticks is an arbitrary number.
            self.window.poll_for_events(&self.options);

            let mut ticks = 100_000;
            while ticks > 0 {
                match self.cpu.run(&mut self.mem, &mut ticks) {
                    cpu::CpuState::Normal => (),
                    cpu::CpuState::Svc(svc) => {
                        // the program counter is pointing at the
                        // instruction after the SVC, but we want the
                        // address of the SVC itself
                        let svc_pc = self.cpu.regs()[cpu::Cpu::PC] - 4;
                        if svc == dyld::Dyld::SVC_RETURN_TO_HOST {
                            assert!(
                                svc_pc
                                    == self.dyld.return_to_host_routine().addr_without_thumb_bit()
                            );
                            assert!(!root);
                            // FIXME/TODO: How do we handle a return-to-host on
                            // the wrong thread? Defer it somehow?
                            if !root && self.current_thread == initial_thread {
                                // Normal return from host-to-guest call
                                return;
                            } else if self.threads[self.current_thread].in_start_routine {
                                // Secondary thread finished starting
                                // TODO: Having two meanings for this SVC is
                                // dangerous, use a different SVC for this case.
                                log_dbg!(
                                    "Thread {} finished start routine and became inactive",
                                    self.current_thread
                                );
                                self.threads[self.current_thread].active = false;
                                let stack = self.threads[self.current_thread].stack.take().unwrap();
                                let stack: mem::MutVoidPtr = mem::Ptr::from_bits(*stack.start());
                                log_dbg!(
                                    "Freeing thread {} stack {:?}",
                                    self.current_thread,
                                    stack
                                );
                                self.mem.free(stack);
                                break;
                            } else {
                                panic!("Unexpected return-to-host!");
                            }
                        }

                        if let Some(f) = self.dyld.get_svc_handler(
                            &self.bins,
                            &mut self.mem,
                            &mut self.cpu,
                            svc_pc,
                            svc,
                        ) {
                            let was_in_host_function =
                                self.threads[self.current_thread].in_host_function;
                            self.threads[self.current_thread].in_host_function = true;
                            f.call_from_guest(self);
                            self.threads[self.current_thread].in_host_function =
                                was_in_host_function;
                        } else {
                            self.cpu.regs_mut()[cpu::Cpu::PC] = svc_pc;
                        }
                    }
                }
            }

            // Find next thread to execute
            let mut next = self.current_thread;
            loop {
                next = (next + 1) % self.threads.len();
                // Back where we started: couldn't find a suitable new
                // thread.
                if next == self.current_thread {
                    assert!(self.threads[self.current_thread].active);
                    break;
                }
                if !self.threads[next].active || self.threads[next].in_host_function {
                    continue;
                }
                // Candidate found, switch to it.
                self.switch_thread(next);
                break;
            }
        }
    }
}
