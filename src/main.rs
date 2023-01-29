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
mod mach_o;
mod mem;
mod objc;
mod stack;
mod window;

use std::path::PathBuf;

const USAGE: &str = "\
Usage:
    touchHLE path/to/some.app

General options:
    --help
        Print this help text.

Debugging options:
    --breakpoint=...
        This option sets a primitive breakpoint at a provided memory address.
        The target instruction will be overwritten shortly after the binary is
        loaded, and executing the instruction will cause touchHLE to panic.

        The address is hexadecimal and can have an optional '0x' prefix.
        If the target instruction is a Thumb instruction, either the lowest bit
        of the address must be set, or the address should be prefixed with 'T',
        e.g. 'T0xF00' or 'TF00'.

        To set multiple breakpoints, use several '--breakpoint=' arguments.
";

fn main() -> Result<(), String> {
    let mut args = std::env::args();
    let _ = args.next().unwrap(); // skip argv[0]

    let mut bundle_path: Option<PathBuf> = None;
    let mut breakpoints = Vec::new();
    for arg in args {
        if arg == "--help" {
            println!("{}", USAGE);
            return Ok(());
        } else if bundle_path.is_none() {
            bundle_path = Some(PathBuf::from(arg));
        } else if let Some(addr) = arg.strip_prefix("--breakpoint=") {
            let is_thumb = addr.starts_with('T');
            let addr = addr.strip_prefix('T').unwrap_or(addr);
            let addr = addr.strip_prefix("0x").unwrap_or(addr);
            let addr = u32::from_str_radix(addr, 16)
                .map_err(|_| "Incorrect breakpoint syntax".to_string())?;
            breakpoints.push(if is_thumb { addr | 0x1 } else { addr });
        } else {
            eprintln!("{}", USAGE);
            return Err(format!("Unexpected argument: {:?}", arg));
        }
    }

    let Some(bundle_path) = bundle_path else {
        eprintln!("{}", USAGE);
        return Err("Path to bundle must be specified".to_string());
    };

    // When PowerShell does tab-completion on a directory, for some reason it
    // expands it to `'..\My Bundle.app\'` and that trailing \ seems to
    // get interpreted as escaping a double quotation mark? Let's just tolerate
    // this.
    #[cfg(windows)]
    let bundle_path = if let Some(fixed) = bundle_path.to_str().and_then(|s| s.strip_suffix('"')) {
        log!("Assuming bundle path was meant to be {:?}.", fixed);
        PathBuf::from(fixed)
    } else {
        bundle_path
    };

    let mut env = Environment::new(bundle_path, breakpoints)?;
    env.run();
    Ok(())
}

/// Index into the [Vec] of threads. Thread 0 is always the main thread.
type ThreadID = usize;

struct Thread {
    /// Once a thread finishes, this is set to false.
    active: bool,
    /// Context object containing the CPU state for this thread.
    ///
    /// There should always be `(threads.len() - 1)` contexts in existence.
    /// When a thread is currently executing, its state is stored directly in
    /// the CPU, rather than in a context object. In that case, this field is
    /// None. See also: [std::mem::take] and [cpu::Cpu::swap_context].
    context: Option<cpu::CpuContext>,
    /// Address range of this thread's stack, used to check if addresses are in
    /// range while producing a stack trace.
    stack: std::ops::RangeInclusive<u32>,
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
}

impl Environment {
    /// Loads the binary and sets up the emulator.
    fn new(bundle_path: PathBuf, breakpoints: Vec<u32>) -> Result<Environment, String> {
        let startup_time = std::time::Instant::now();

        let (bundle, fs) = match bundle::Bundle::new_bundle_and_fs_from_host_path(bundle_path) {
            Ok(bundle) => bundle,
            Err(err) => {
                return Err(format!("Application bundle error: {}. Check that the path is to a .app directory. If this is a .ipa file, you need to extract it as a ZIP file to get the .app directory.", err));
            }
        };

        let icon = fs
            .read(bundle.icon_path())
            .map_err(|_| "Could not read icon file".to_string())?;
        let icon = image::Image::from_bytes(&icon)
            .map_err(|_| "Could not parse icon image".to_string())?;

        let launch_image = fs
            .read(bundle.launch_image_path())
            .ok()
            .and_then(|bytes| image::Image::from_bytes(&bytes).ok());

        let window = window::Window::new(
            &format!("{} (touchHLE)", bundle.display_name()),
            icon,
            launch_image,
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

        for breakpoint in breakpoints {
            dyld.set_breakpoint(&mut mem, breakpoint);
        }

        let cpu = cpu::Cpu::new();

        let main_thread = Thread {
            active: true,
            context: None,
            stack: mem::Mem::MAIN_THREAD_STACK_LOW_END..=0u32.wrapping_sub(1),
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

        // FIXME: call library static initializers too
        if let Some(mod_init_func) = env.bins[0].get_section("__mod_init_func") {
            log_dbg!("Calling static initializers for {:?}", env.bins[0].name);
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
        let stack_range = self.threads[self.current_thread].stack.clone();
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

        let mut context = cpu::CpuContext::new();
        // Save CPU state for current thread to `context` and reset CPU
        // registers to zeroes.
        self.cpu.swap_context(&mut context);
        self.cpu.set_cpsr(cpu::Cpu::CPSR_USER_MODE);
        self.cpu.regs_mut()[cpu::Cpu::SP] = stack_high_addr;
        self.cpu.regs_mut()[0] = user_data.to_bits();
        self.cpu
            .branch_with_link(start_routine, self.dyld.return_to_host_routine());
        // Restore CPU state of current thread, get our new thread's state.
        self.cpu.swap_context(&mut context);

        self.threads.push(Thread {
            active: true,
            context: Some(context),
            stack: stack_alloc.to_bits()..=(stack_high_addr - 1),
        });
        let new_thread_id = self.threads.len() - 1;

        log_dbg!("Created new thread {} with stack {:#x}–{:#x}, will execute function {:?} with data {:?}", new_thread_id, stack_alloc.to_bits(), (stack_high_addr - 1), start_routine, user_data);

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
        self.run_inner(false)
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
            self.window.poll_for_events();

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
                            // Normal callback return
                            if !root && self.current_thread == initial_thread {
                                return;
                            // Secondary thread init function return
                            // TODO: Test this actually works.
                            // TODO: Use a different SVC for this, the double
                            // meaning here seems dangerous.
                            } else if self.current_thread != 0 {
                                log_dbg!("Thread {} init finished", self.current_thread);
                                break;
                            } else {
                                panic!("Unexpected return-to-host");
                            }
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
                if !self.threads[next].active {
                    continue;
                }
                let Some(mut context) = self.threads[next].context.take() else {
                    continue;
                };
                // Candidate found, switch to it.
                log_dbg!("Switching thread: {} => {}", self.current_thread, next);
                self.cpu.swap_context(&mut context);
                assert!(self.threads[self.current_thread].context.is_none());
                self.threads[self.current_thread].context = Some(context);
                self.current_thread = next;
                break;
            }
        }
    }
}
