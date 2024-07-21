/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The core of the emulator: management of state, execution, threading.
//!
//! Unlike its siblings, this module should be considered private and only used
//! via the re-exports one level up.

mod mutex;

use crate::abi::GuestRet;
use crate::libc::semaphore::sem_t;
use crate::mem::{MutPtr, MutVoidPtr};
use crate::{
    abi, bundle, cpu, dyld, frameworks, fs, gdb, image, libc, mach_o, mem, objc, options, stack,
    window,
};
use std::net::TcpListener;
use std::time::{Duration, Instant};

pub use mutex::{MutexId, MutexType, PTHREAD_MUTEX_DEFAULT};

/// Index into the [Vec] of threads. Thread 0 is always the main thread.
pub type ThreadId = usize;

/// Bookkeeping for a thread.
pub struct Thread {
    /// Once a thread finishes, this is set to false.
    pub active: bool,
    /// If this is not [ThreadBlock::NotBlocked], the thread is not executing
    /// until a certain condition is fufilled.
    blocked_by: ThreadBlock,
    /// Set to [true] when a thread is running its startup routine (i.e. the
    /// function pointer passed to `pthread_create`). When it returns to the
    /// host, it should become inactive.
    in_start_routine: bool,
    /// After a secondary thread finishes, this is set to the returned value.
    return_value: Option<MutVoidPtr>,
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

impl Thread {
    fn is_blocked(&self) -> bool {
        !matches!(self.blocked_by, ThreadBlock::NotBlocked)
    }
}

/// The struct containing the entire emulator state. Methods are provided for
/// execution and management of threads.
pub struct Environment {
    /// Reference point for various timing functions.
    pub startup_time: Instant,
    pub bundle: bundle::Bundle,
    pub fs: fs::Fs,
    /// The window is only absent when running in headless mode.
    pub window: Option<window::Window>,
    pub mem: mem::Mem,
    /// Loaded binaries. Index `0` is always the app binary, other entries are
    /// dynamic libraries.
    pub bins: Vec<mach_o::MachO>,
    pub objc: objc::ObjC,
    pub dyld: dyld::Dyld,
    pub cpu: cpu::Cpu,
    pub current_thread: ThreadId,
    pub threads: Vec<Thread>,
    pub libc_state: libc::State,
    pub framework_state: frameworks::State,
    pub mutex_state: mutex::MutexState,
    pub options: options::Options,
    gdb_server: Option<gdb::GdbServer>,
}

/// What to do next when executing this thread.
enum ThreadNextAction {
    /// Continue CPU emulation.
    Continue,
    /// Yield to another thread.
    Yield,
    /// Return to host.
    ReturnToHost,
    /// Debug the current CPU error.
    DebugCpuError(cpu::CpuError),
}

/// If/what a thread is blocked by.
#[derive(Debug, Clone)]
enum ThreadBlock {
    // Default state. (thread is not blocked)
    NotBlocked,
    // Thread is sleeping. (until Instant)
    Sleeping(Instant),
    // Thread is waiting for a mutex to unlock.
    Mutex(MutexId),
    // Thread is waiting on a semaphore.
    Semaphore(MutPtr<sem_t>),
    // Thread is waiting for another thread to finish (joining).
    Joining(ThreadId, MutPtr<MutVoidPtr>),
    // Deferred guest-to-host return
    DeferredReturn,
}

impl Environment {
    /// Loads the binary and sets up the emulator.
    ///
    /// `env_for_salvage` can be used to provide an existing environment (in
    /// practice, the app picker's, created with [Environment::new_without_app])
    /// that is to be destroyed. Certain components may be salvaged from the
    /// old environment, but their states will be reset, so the result should be
    /// "like new". This option exists because touchHLE on Android would crash
    /// when allocating a second [mem::Mem] instance.
    pub fn new(
        bundle: bundle::Bundle,
        fs: fs::Fs,
        options: options::Options,
        env_for_salvage: Option<Environment>,
    ) -> Result<Environment, String> {
        let startup_time = Instant::now();

        // Extract things to salvage from the old environment, and then drop it.
        // This needs to be done before creating a new window, because SDL2 only
        // allows one window at once.
        let mem_for_salvage = if let Some(env_for_salvage) = env_for_salvage {
            let Environment { mem, .. } = env_for_salvage;
            // Everything other than the memory is now dropped.
            Some(mem)
        } else {
            None
        };

        let window = if options.headless {
            None
        } else {
            let icon = bundle.load_icon(&fs);
            if let Err(ref e) = icon {
                log!("Warning: {}", e);
            }

            let launch_image_path = bundle.launch_image_path();
            let launch_image = if fs.is_file(&launch_image_path) {
                let res = fs
                    .read(launch_image_path)
                    .map_err(|_| "Could not read launch image file".to_string())
                    .and_then(|bytes| {
                        image::Image::from_bytes(&bytes)
                            .map_err(|e| format!("Could not parse launch image: {}", e))
                    });
                if let Err(ref e) = res {
                    log!("Warning: {}", e);
                };
                res.ok()
            } else {
                None
            };

            Some(window::Window::new(
                &format!(
                    "{} (touchHLE {}{}{})",
                    bundle.display_name(),
                    super::branding(),
                    if super::branding().is_empty() {
                        ""
                    } else {
                        " "
                    },
                    super::VERSION
                ),
                icon.ok(),
                launch_image,
                &options,
            ))
        };

        let mut mem = if let Some(mem) = mem_for_salvage {
            mem::Mem::refurbish(mem)
        } else {
            mem::Mem::new()
        };

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

        let cpu = cpu::Cpu::new(match options.direct_memory_access {
            true => Some(&mut mem),
            false => None,
        });

        let main_thread = Thread {
            active: true,
            blocked_by: ThreadBlock::NotBlocked,
            return_value: None,
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
            mutex_state: Default::default(),
            framework_state: Default::default(),
            options,
            gdb_server: None,
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

        env.cpu.set_cpsr(cpu::Cpu::CPSR_USER_MODE);

        if let Some(addrs) = env.options.gdb_listen_addrs.take() {
            let listener = TcpListener::bind(addrs.as_slice())
                .map_err(|e| format!("Could not bind to {:?}: {}", addrs, e))?;
            echo!(
                "Waiting for debugger connection on {}...",
                addrs
                    .into_iter()
                    .map(|a| format!("{}", a))
                    .collect::<Vec<String>>()
                    .join(", ")
            );
            let (client, client_addr) = listener
                .accept()
                .map_err(|e| format!("Could not accept connection: {}", e))?;
            echo!("Debugger client connected on {}.", client_addr);
            let mut gdb_server = gdb::GdbServer::new(client);
            let step = gdb_server.wait_for_debugger(None, &mut env.cpu, &mut env.mem);
            assert!(!step, "Can't step right now!"); // TODO?
            env.gdb_server = Some(gdb_server);
        }

        echo!("CPU emulation begins now.");

        // Static initializers for libraries must be run before the initializer
        // in the app binary.
        // TODO: once we support more libraries, replace this hard-coded order
        //       with e.g. a topological sort.
        assert!(env.bins.len() <= 3);
        for bin_idx in [1, 2, 0] {
            let Some(bin) = env.bins.get(bin_idx) else {
                continue;
            };
            let Some(section) = bin.get_section(mach_o::SectionType::ModInitFuncPointers) else {
                continue;
            };

            log_dbg!("Calling static initializers for {:?}", bin.name);
            assert!(section.size % 4 == 0);
            let base: mem::ConstPtr<abi::GuestFunction> = mem::Ptr::from_bits(section.addr);
            let count = section.size / 4;
            for i in 0..count {
                let func = env.mem.read(base + i);
                func.call(&mut env);
            }
            log_dbg!("Static initialization done");
        }

        env.cpu.branch(entry_point_addr);

        Ok(env)
    }

    /// Set up the emulator environment without loading an app binary.
    ///
    /// This is a special mode that only exists to support the app picker, which
    /// uses the emulated environment to draw its UI and process input. Filling
    /// some of the fields with fake data is a hack, but it means the frameworks
    /// do not need to be aware of the app picker's peculiarities, so it is
    /// cleaner than the alternative!
    pub fn new_without_app(options: options::Options) -> Result<Environment, String> {
        let bundle = bundle::Bundle::new_fake_bundle();
        let fs = fs::Fs::new_fake_fs();

        let startup_time = Instant::now();

        let icon = None;
        let launch_image = None;

        assert!(!options.headless);
        let window = Some(window::Window::new(
            &format!(
                "touchHLE {}{}{}",
                super::branding(),
                if super::branding().is_empty() {
                    ""
                } else {
                    " "
                },
                super::VERSION
            ),
            icon,
            launch_image,
            &options,
        ));

        let mut mem = mem::Mem::new();

        let bins = Vec::new();

        let mut objc = objc::ObjC::new();

        let mut dyld = dyld::Dyld::new();
        dyld.do_initial_linking_with_no_bins(&mut mem, &mut objc);

        let cpu = cpu::Cpu::new(match options.direct_memory_access {
            true => Some(&mut mem),
            false => None,
        });

        let main_thread = Thread {
            active: true,
            blocked_by: ThreadBlock::NotBlocked,
            return_value: None,
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
            mutex_state: Default::default(),
            framework_state: Default::default(),
            options,
            gdb_server: None,
        };

        // Dyld::do_late_linking() would be called here, but it doesn't do
        // anything relevant here, so it's skipped.

        {
            let argv = &[];
            let envp = &[];
            let apple = &[];
            stack::prep_stack_for_start(&mut env.mem, &mut env.cpu, argv, envp, apple);
        }

        env.cpu.set_cpsr(cpu::Cpu::CPSR_USER_MODE);

        // GDB server setup would be done here, but there's no need for it.

        // "CPU emulation begins now" would happen here, but there's nothing
        // to emulate. :)

        Ok(env)
    }

    /// Get a shared reference to the window. Panics if touchHLE is running in
    /// headless mode.
    pub fn window(&self) -> &window::Window {
        self.window.as_ref().expect(
            "Tried to do something that needs a window, but touchHLE is running in headless mode!",
        )
    }

    /// Get a mutable reference to the window. Panics if touchHLE is running
    /// in headless mode.
    pub fn window_mut(&mut self) -> &mut window::Window {
        self.window.as_mut().expect(
            "Tried to do something that needs a window, but touchHLE is running in headless mode!",
        )
    }

    fn stack_trace(&self) {
        if self.current_thread == 0 {
            echo!("Attempting to produce stack trace for main thread:");
        } else {
            echo!(
                "Attempting to produce stack trace for thread {}:",
                self.current_thread
            );
        }
        let stack_range = self.threads[self.current_thread].stack.clone().unwrap();
        echo!(
            " 0. {:#x} (PC)",
            self.cpu.pc_with_thumb_bit().addr_with_thumb_bit()
        );
        let regs = self.cpu.regs();
        let mut lr = regs[cpu::Cpu::LR];
        let return_to_host_routine_addr = self.dyld.return_to_host_routine().addr_with_thumb_bit();
        let thread_exit_routine_addr = self.dyld.thread_exit_routine().addr_with_thumb_bit();
        if lr == return_to_host_routine_addr {
            echo!(" 1. [host function] (LR)");
        } else if lr == thread_exit_routine_addr {
            echo!(" 1. [thread exit] (LR)");
            return;
        } else {
            echo!(" 1. {:#x} (LR)", lr);
        }
        let mut i = 2;
        let mut fp: mem::ConstPtr<u8> = mem::Ptr::from_bits(regs[abi::FRAME_POINTER]);
        loop {
            if !stack_range.contains(&fp.to_bits()) {
                echo!("Next FP ({:?}) is outside the stack.", fp);
                break;
            }
            lr = self.mem.read((fp + 4).cast());
            fp = self.mem.read(fp.cast());
            if lr == return_to_host_routine_addr {
                echo!("{:2}. [host function]", i);
            } else if lr == thread_exit_routine_addr {
                echo!("{:2}. [thread exit]", i);
                return;
            } else {
                echo!("{:2}. {:#x}", i, lr);
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
    ) -> ThreadId {
        let stack_size = mem::Mem::SECONDARY_THREAD_STACK_SIZE;
        let stack_alloc = self.mem.alloc(stack_size);
        let stack_high_addr = stack_alloc.to_bits() + stack_size;
        assert!(stack_high_addr % 4 == 0);

        self.threads.push(Thread {
            active: true,
            blocked_by: ThreadBlock::NotBlocked,
            return_value: None,
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
            .branch_with_link(start_routine, self.dyld.thread_exit_routine());
        self.switch_thread(old_thread);

        new_thread_id
    }

    /// Put the current thread to sleep for some duration, running other threads
    /// in the meantime as appropriate. Functions that call sleep right before
    /// they return back to the main run loop ([Environment::run]) should set
    /// `tail_call`.
    pub fn sleep(&mut self, duration: Duration, tail_call: bool) {
        assert!(matches!(
            self.threads[self.current_thread].blocked_by,
            ThreadBlock::NotBlocked
        ));

        log_dbg!(
            "Thread {} is going to sleep for {:?}.",
            self.current_thread,
            duration
        );
        let until = Instant::now().checked_add(duration).unwrap();
        self.threads[self.current_thread].blocked_by = ThreadBlock::Sleeping(until);
        // For non tail-call sleeps (such as in NSRunLoop), we want to poll
        // other threads but can't return back to the run loop, since it would
        // go through the calling function. As such, we have to call into the
        // run loop instead.
        if !tail_call {
            let old_pc = self.cpu.pc_with_thumb_bit();
            self.cpu.branch(self.dyld.return_to_host_routine());
            // Since the current thread is asleep, this will only run other
            // threads until it wakes up, at which point it signals
            // return-to-host and control is returned to this function.
            self.run_call();
            self.cpu.branch(old_pc);
        }
    }

    /// Block the current thread until the given mutex unlocks.
    ///
    /// Other threads also blocking on this mutex may get access first.
    /// Also note that like [Self::sleep], this only takes effect after the host
    /// function returns to the main run loop ([Environment::run]).
    pub fn block_on_mutex(&mut self, mutex_id: MutexId) {
        assert!(matches!(
            self.threads[self.current_thread].blocked_by,
            ThreadBlock::NotBlocked
        ));
        log_dbg!(
            "Thread {} blocking on mutex #{}.",
            self.current_thread,
            mutex_id
        );
        self.threads[self.current_thread].blocked_by = ThreadBlock::Mutex(mutex_id);
    }

    /// Locks a semaphore (decrements value of a semaphore and blocks
    /// if necessary).
    ///
    /// Also note that like [Self::sleep], this only takes effect after the host
    /// function returns to the main run loop ([Environment::run]).
    pub fn sem_decrement(&mut self, sem: MutPtr<sem_t>, wait_on_lock: bool) -> bool {
        let host_sem_rc: &mut _ = self
            .libc_state
            .semaphore
            .open_semaphores
            .get_mut(&sem)
            .unwrap();
        let mut host_sem = (*host_sem_rc).borrow_mut();

        host_sem.value -= 1;
        log_dbg!(
            "sem_decrement: semaphore {:?} is now {}",
            sem,
            host_sem.value
        );

        if !wait_on_lock {
            if host_sem.value < 0 {
                host_sem.value += 1;
                return false;
            }
            return true;
        }

        if host_sem.value < 0 {
            assert!(matches!(
                self.threads[self.current_thread].blocked_by,
                ThreadBlock::NotBlocked
            ));
            log_dbg!(
                "Thread {} is blocking on semaphore {:?}",
                self.current_thread,
                sem
            );
            host_sem.waiting.insert(self.current_thread);
            self.threads[self.current_thread].blocked_by = ThreadBlock::Semaphore(sem);
        }

        true
    }

    /// Unlock a semaphore (increments value of a semaphore)
    ///
    /// Note: Actual thread awaking is done inside [Environment::run_inner] loop
    ///
    /// Also note that like [Self::sleep], this only takes effect after the host
    /// function returns to the main run loop ([Environment::run]).
    pub fn sem_increment(&mut self, sem: MutPtr<sem_t>) {
        let host_sem_rc: &mut _ = self
            .libc_state
            .semaphore
            .open_semaphores
            .get_mut(&sem)
            .unwrap();
        let mut host_sem = (*host_sem_rc).borrow_mut();

        host_sem.value += 1;
        log_dbg!(
            "sem_increment: semaphore {:?} is now {}",
            sem,
            host_sem.value
        );
    }

    /// Blocks the current thread until the thread given finishes, writing its
    /// return value to ptr (if non-null).
    ///
    /// Note that there are no protections against joining with a detached
    /// thread, joining a thread with itself, or deadlocking joins. Callers
    /// should ensure these do not occur!
    ///
    /// Also note that like [Self::sleep], this only takes effect after the host
    /// function returns to the main run loop ([Environment::run]).
    pub fn join_with_thread(&mut self, joinee_thread: ThreadId, ptr: MutPtr<MutVoidPtr>) {
        assert!(matches!(
            self.threads[self.current_thread].blocked_by,
            ThreadBlock::NotBlocked
        ));
        log_dbg!(
            "Thread {} waiting for thread {} to finish.",
            self.current_thread,
            joinee_thread
        );
        self.threads[self.current_thread].blocked_by = ThreadBlock::Joining(joinee_thread, ptr);
    }

    /// Run the emulator. This is the main loop and won't return until app exit.
    /// Only `main.rs` should call this.
    pub fn run(&mut self) {
        // I'm not sure if this actually is unwind-safe, but considering
        // the emulator will crash anyway, maybe this is okay.
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.run_inner(true)));
        if let Err(e) = res {
            echo!("Register state immediately after panic:");
            self.cpu.dump_regs();
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
        let old_thread = self.current_thread;
        self.threads[self.current_thread].in_host_function = false;
        self.run_inner(false);
        assert!(self.current_thread == old_thread);
        self.threads[self.current_thread].in_host_function = was_in_host_function;
    }

    fn switch_thread(&mut self, new_thread: ThreadId) {
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

    #[cold]
    /// Let the debugger handle a CPU error, or panic if there's no debugger
    /// connected. Returns [true] if the CPU should step and then resume
    /// debugging, or [false] if it should resume normal execution.
    fn debug_cpu_error(&mut self, error: cpu::CpuError) -> bool {
        if matches!(error, cpu::CpuError::UndefinedInstruction)
            || matches!(error, cpu::CpuError::Breakpoint)
        {
            // Rewind the PC so that it's at the instruction where the error
            // occurred, rather than the next instruction. This is necessary for
            // GDB to detect its software breakpoints. For some reason this
            // isn't correct for memory errors however.
            let instruction_len = if (self.cpu.cpsr() & cpu::Cpu::CPSR_THUMB) != 0 {
                2
            } else {
                4
            };
            self.cpu.regs_mut()[cpu::Cpu::PC] -= instruction_len;
        }

        if self.gdb_server.is_none() {
            panic!("Error during CPU execution: {:?}", error);
        }

        echo!("Debuggable error during CPU execution: {:?}.", error);
        self.enter_debugger(Some(error))
    }

    /// Used to check whether a debugger is connected, and therefore whether
    /// [Environment::enter_debugger] will do something.
    pub fn is_debugging_enabled(&self) -> bool {
        self.gdb_server.is_some()
    }

    /// Suspend execution and hand control to the connected debugger.
    /// You should precede this call with a log message that explains why the
    /// debugger is being invoked. The return value is the same as
    /// [gdb::GdbServer::wait_for_debugger]'s.
    #[must_use]
    pub fn enter_debugger(&mut self, reason: Option<cpu::CpuError>) -> bool {
        // GDB doesn't seem to manage to produce a useful stack trace, so
        // let's print our own.
        self.stack_trace();
        self.gdb_server
            .as_mut()
            .unwrap()
            .wait_for_debugger(reason, &mut self.cpu, &mut self.mem)
    }

    #[inline(always)]
    /// Respond to the new CPU state (do nothing, execute an SVC or enter
    /// debugging) and decide what to do next.
    fn handle_cpu_state(
        &mut self,
        state: cpu::CpuState,
        initial_thread: ThreadId,
        root: bool,
    ) -> ThreadNextAction {
        match state {
            cpu::CpuState::Normal => ThreadNextAction::Continue,
            cpu::CpuState::Svc(svc) => {
                // the program counter is pointing at the
                // instruction after the SVC, but we want the
                // address of the SVC itself
                let svc_pc = self.cpu.regs()[cpu::Cpu::PC] - 4;
                match svc {
                    dyld::Dyld::SVC_THREAD_EXIT => {
                        assert!(svc_pc == self.dyld.thread_exit_routine().addr_without_thumb_bit());
                        if !self.threads[self.current_thread].in_start_routine {
                            panic!("Non-exiting thread {} exited!", self.current_thread);
                        } else {
                            // Secondary thread finished starting.
                            log_dbg!(
                                "Thread {} finished start routine and became inactive, {}",
                                self.current_thread,
                                initial_thread
                            );
                            let curr_thread = &mut self.threads[self.current_thread];
                            curr_thread.return_value = Some(GuestRet::from_regs(self.cpu.regs()));
                            curr_thread.active = false;
                            let stack = curr_thread.stack.take().unwrap();
                            let stack: mem::MutVoidPtr = mem::Ptr::from_bits(*stack.start());
                            log_dbg!("Freeing thread {} stack {:?}", self.current_thread, stack);
                            self.mem.free(stack);
                            ThreadNextAction::Yield
                        }
                    }
                    dyld::Dyld::SVC_RETURN_TO_HOST => {
                        assert!(
                            svc_pc == self.dyld.return_to_host_routine().addr_without_thumb_bit()
                        );
                        assert!(!root);
                        if self.current_thread == initial_thread {
                            log_dbg!(
                                "Thread {} returned from host-to-guest call",
                                self.current_thread
                            );
                            // Normal return from host-to-guest call.
                            ThreadNextAction::ReturnToHost
                        } else {
                            // FIXME?: A drawback of the current thread model is
                            // that host-to-guest calls affect the host call
                            // stack. This is a problem because it means that
                            // threads have to return in the order they were
                            // called, which means that threads that return
                            // while they aren't at the top of the call stack
                            // have to wait until they can.
                            log_dbg!("Thread {} returned from host-to-guest call but thread {} is top of call stack, deferring!",
                                     self.current_thread,
                                     initial_thread
                            );
                            self.threads[self.current_thread].blocked_by =
                                ThreadBlock::DeferredReturn;
                            ThreadNextAction::Yield
                        }
                    }
                    dyld::Dyld::SVC_LAZY_LINK | dyld::Dyld::SVC_LINKED_FUNCTIONS_BASE.. => {
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
                            // Host function might have put the thread to sleep.
                            if let ThreadBlock::NotBlocked =
                                self.threads[self.current_thread].blocked_by
                            {
                                ThreadNextAction::Continue
                            } else {
                                log_dbg!("Yielding: thread {} is blocked.", self.current_thread);
                                ThreadNextAction::Yield
                            }
                        } else {
                            self.cpu.regs_mut()[cpu::Cpu::PC] = svc_pc;
                            ThreadNextAction::Continue
                        }
                    }
                }
            }
            cpu::CpuState::Error(e) => ThreadNextAction::DebugCpuError(e),
        }
    }

    fn run_inner(&mut self, root: bool) {
        let initial_thread = self.current_thread;
        assert!(self.threads[initial_thread].active);
        assert!(self.threads[initial_thread].context.is_none());

        loop {
            // 100,000 ticks is an arbitrary number. It needs to be reasonably
            // large so we aren't jumping in and out of dynarmic or trying to
            // poll for events too often. At the same time, very large values
            // are bad for responsiveness.
            let mut ticks = if self.threads[self.current_thread].is_blocked() {
                // The current thread might be asleep, in which case we want to
                // immediately switch to another thread. This only happens when
                // called from Self::sleep().
                0
            } else {
                100_000
            };
            let mut step_and_debug = false;
            while ticks > 0 {
                let state = self.cpu.run_or_step(
                    &mut self.mem,
                    if step_and_debug {
                        None
                    } else {
                        Some(&mut ticks)
                    },
                );
                match self.handle_cpu_state(state, initial_thread, root) {
                    ThreadNextAction::Continue => {
                        if step_and_debug {
                            step_and_debug = self.gdb_server.as_mut().unwrap().wait_for_debugger(
                                None,
                                &mut self.cpu,
                                &mut self.mem,
                            );
                        }
                    }
                    ThreadNextAction::Yield => break,
                    ThreadNextAction::ReturnToHost => return,
                    ThreadNextAction::DebugCpuError(e) => {
                        step_and_debug = self.debug_cpu_error(e);
                    }
                }
            }

            // To maintain responsiveness when moving the window and so on, we
            // need to poll for events occasionally, even if the app isn't
            // actively processing them.
            // Polling for events can be quite expensive, so we shouldn't do
            // this until after we've done some amount of work on the guest
            // thread, lest every single callback call pay this cost.
            if let Some(ref mut window) = self.window {
                window.poll_for_events(&self.options);
            }

            loop {
                // Try to find a new thread to execute, starting with the thread
                // following the one currently executing.
                let mut suitable_thread: Option<ThreadId> = None;
                let mut next_awakening: Option<Instant> = None;
                let mut mutex_to_relock: Option<MutexId> = None;
                for i in 0..self.threads.len() {
                    let i = (self.current_thread + 1 + i) % self.threads.len();
                    let candidate = &mut self.threads[i];

                    if !candidate.active || candidate.in_host_function {
                        continue;
                    }
                    match candidate.blocked_by {
                        ThreadBlock::Sleeping(sleeping_until) => {
                            if sleeping_until <= Instant::now() {
                                log_dbg!("Thread {} finished sleeping.", i);
                                candidate.blocked_by = ThreadBlock::NotBlocked;
                                suitable_thread = Some(i);
                                break;
                            } else {
                                next_awakening = match next_awakening {
                                    None => Some(sleeping_until),
                                    Some(other) => Some(other.min(sleeping_until)),
                                };
                            }
                        }
                        ThreadBlock::Mutex(mutex_id) => {
                            if !self.mutex_state.mutex_is_locked(mutex_id) {
                                log_dbg!("Thread {} was unblocked due to mutex #{} unlocking, relocking mutex.", i, mutex_id);
                                self.threads[i].blocked_by = ThreadBlock::NotBlocked;
                                suitable_thread = Some(i);
                                mutex_to_relock = Some(mutex_id);
                                break;
                            }
                        }
                        ThreadBlock::Semaphore(sem) => {
                            let host_sem_rc: &mut _ = self
                                .libc_state
                                .semaphore
                                .open_semaphores
                                .get_mut(&sem)
                                .unwrap();
                            let host_sem = (*host_sem_rc).borrow();

                            if host_sem.value >= 0 {
                                log_dbg!(
                                    "Thread {} has awaken on semaphore {:?} with value {}",
                                    i,
                                    sem,
                                    host_sem.value
                                );
                                self.threads[i].blocked_by = ThreadBlock::NotBlocked;
                                suitable_thread = Some(i);
                                break;
                            }
                        }
                        ThreadBlock::Joining(joinee_thread, ptr) => {
                            if !self.threads[joinee_thread].active {
                                log_dbg!(
                                    "Thread {} joining with now finished thread {}.",
                                    self.current_thread,
                                    joinee_thread
                                );
                                // Write the return value, unless the pointer to
                                // write to is null.
                                if !ptr.is_null() {
                                    self.mem.write(
                                        ptr,
                                        self.threads[joinee_thread].return_value.unwrap(),
                                    );
                                }
                                self.threads[i].blocked_by = ThreadBlock::NotBlocked;
                                suitable_thread = Some(i);
                                break;
                            }
                        }
                        ThreadBlock::DeferredReturn => {
                            if i == initial_thread {
                                log_dbg!("Thread {} is now able to return, returning", i);
                                self.threads[i].blocked_by = ThreadBlock::NotBlocked;
                                // Thread is now top of call stack, should
                                // return
                                self.switch_thread(i);
                                return;
                            }
                        }
                        ThreadBlock::NotBlocked => {
                            suitable_thread = Some(i);
                            break;
                        }
                    }
                }

                // There's a suitable thread we can switch to immediately.
                if let Some(suitable_thread) = suitable_thread {
                    if suitable_thread != self.current_thread {
                        self.switch_thread(suitable_thread);
                    }
                    if let Some(mutex_id) = mutex_to_relock {
                        self.relock_unblocked_mutex(mutex_id);
                    }
                    break;
                // All suitable threads are blocked and at least one is asleep.
                // Sleep until one of them wakes up.
                } else if let Some(next_awakening) = next_awakening {
                    let duration = next_awakening.duration_since(Instant::now());
                    log_dbg!("All threads blocked/asleep, sleeping for {:?}.", duration);
                    std::thread::sleep(duration);
                    // Try again, there should be some thread awake now (or
                    // there will be soon, since timing is approximate).
                    continue;
                } else {
                    // This should hopefully not happen, but if a thread is
                    // blocked on another thread waiting for a deferred return,
                    // it could.
                    panic!("No active threads, program has deadlocked!");
                }
            }
        }
    }
}
