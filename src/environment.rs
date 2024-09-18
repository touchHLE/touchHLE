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
mod nullable_box;

use crate::abi::CallFromHost;
use crate::libc::semaphore::sem_t;
use crate::mem::{GuestUSize, MutPtr, MutVoidPtr};
use crate::{
    abi, bundle, cpu, dyld, frameworks, fs, gdb, image, libc, mach_o, mem, objc, options, stack,
    window,
};
use std::cell::Cell;
use std::collections::HashMap;
use std::net::TcpListener;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::libc::pthread::cond::pthread_cond_t;
use corosensei::{Coroutine, Yielder};
pub use mutex::{MutexId, MutexType, PTHREAD_MUTEX_DEFAULT};
use nullable_box::NullableBox;

/// Index into the [Vec] of threads. Thread 0 is always the main thread.
pub type ThreadId = usize;

pub type HostContext = Coroutine<Environment, Environment, Environment>;

/// Bookkeeping for a thread.
pub struct Thread {
    /// Once a thread finishes, this is set to false.
    pub active: bool,
    /// If this is not [ThreadBlock::NotBlocked], the thread is not executing
    /// until a certain condition is fufilled.
    pub blocked_by: ThreadBlock,
    // BEFOREMERGE: Note: POSIX seems to specify that you can
    // cancel/pthread_exit the main thread, so there's no reason
    // to keep in_start_routine around.
    //
    /// After a secondary thread finishes, this is set to the returned value.
    return_value: Option<MutVoidPtr>,
    /// Context object containing the CPU state for this thread.
    ///
    /// There should always be `(threads.len() - 1)` contexts in existence.
    /// When a thread is currently executing, its state is stored directly in
    /// the CPU, rather than in a context object. In that case, this field is
    /// None. See also: [std::mem::take] and [cpu::Cpu::swap_context].
    guest_context: Option<Box<cpu::CpuContext>>,
    // The coroutine associated with this thread.
    //
    // In more typical rust, this is equivalent to
    host_context: Option<HostContext>,
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
    // BEFOREMERGE: Note: these are all split up instead of there being one
    // Environment inside a NullableBox since rust doesn't  (somewhat
    // intentionally) understand "split borrows" behind a Deref type.
    // This can't change without a large amount of code refactoring or
    // proc macro magic.
    //
    /// Reference point for various timing functions.
    pub startup_time: Instant,
    pub bundle: NullableBox<bundle::Bundle>,
    pub fs: NullableBox<fs::Fs>,
    /// The window is only absent when running in headless mode.
    pub window: Option<Box<window::Window>>,
    pub mem: NullableBox<mem::Mem>,
    /// Loaded binaries. Index `0` is always the app binary, other entries are
    /// dynamic libraries.
    pub bins: Vec<mach_o::MachO>,
    pub objc: NullableBox<objc::ObjC>,
    pub dyld: NullableBox<dyld::Dyld>,
    pub cpu: NullableBox<cpu::Cpu>,
    pub current_thread: ThreadId,
    pub threads: Vec<Thread>,
    pub libc_state: NullableBox<libc::State>,
    pub framework_state: NullableBox<frameworks::State>,
    pub mutex_state: NullableBox<mutex::MutexState>,
    pub options: NullableBox<options::Options>,
    gdb_server: Option<Box<gdb::GdbServer>>,
    pub env_vars: HashMap<Vec<u8>, MutPtr<u8>>,
    yielder: *const Yielder<Environment, Environment>,
    // The amount of ticks to run for Some(value), or single-stepping for None.
    // BEFOREMERGE: Note: Sadly, setting ticks to 1 does not step properly, so
    // Option is required.
    remaining_ticks: Option<u64>,
    // BEFOREMERGE: document
    // BEFOREMERGE: this has to be pub because of the app picker, maybe we can
    // move the app picker into the env namespace? I don't like having it in
    // pub.
    pub panic_cell: Rc<Cell<Option<Environment>>>,
}

/// What to do next when executing this thread.
enum ThreadNextAction {
    /// Continue CPU emulation.
    Continue,
    /// Return to host.
    ReturnToHost,
    /// Debug the current CPU error.
    DebugCpuError(cpu::CpuError),
}

/// If/what a thread is blocked by.
#[derive(Debug, Clone)]
pub enum ThreadBlock {
    // Default state. (thread is not blocked)
    NotBlocked,
    // Thread is sleeping. (until Instant)
    Sleeping(Instant),
    // Thread is waiting for a mutex to unlock.
    Mutex(MutexId),
    // Thread is waiting on a semaphore.
    Semaphore(MutPtr<sem_t>),
    // Thread is wating on a condition variable
    Condition(pthread_cond_t),
    // Thread is waiting for another thread to finish (joining).
    Joining(ThreadId, MutPtr<MutVoidPtr>),
    // Thread has hit a cpu error, and is waiting to be debugged.
    WaitingForDebugger(Option<cpu::CpuError>),
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
            let mem = env_for_salvage.salvage();
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

            Some(Box::new(window::Window::new(
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
            )))
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

        let entry_point_addr = executable
            .entry_point_pc
            .ok_or_else(|| {
                "Mach-O file does not specify an entry point PC, perhaps it is not an executable?"
                    .to_string()
            })
            .unwrap();
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

        let main_thread_init_routine = Coroutine::new(move |yielder, mut env: Environment| {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                env.run_with_yielder(yielder, move |env| {
                    // Static initializers for libraries must be run before the
                    // initializer in the app binary.
                    //
                    // BEFOREMERGE: Note: Doing it this way shifts when static
                    // initializers are run from Environment::new() to
                    // Environment::run() (since the coroutine isn't run until
                    // the main run loop is reached). This is cleaner imo, but
                    // there's room for disagreement.
                    // TODO: once we support more libraries, replace this
                    // hard-coded order with e.g. a topological sort.
                    assert!(env.bins.len() <= 3);
                    for bin_idx in [1, 2, 0] {
                        let Some(bin) = env.bins.get(bin_idx) else {
                            continue;
                        };
                        let Some(section) =
                            bin.get_section(mach_o::SectionType::ModInitFuncPointers)
                        else {
                            continue;
                        };

                        log_dbg!("Calling static initializers for {:?}", bin.name);
                        assert!(section.size % 4 == 0);
                        let base: mem::ConstPtr<abi::GuestFunction> =
                            mem::Ptr::from_bits(section.addr);
                        let count = section.size / 4;
                        for i in 0..count {
                            let func = env.mem.read(base + i);
                            () = func.call_from_host(env, ());
                        }
                        log_dbg!("Static initialization done");
                    }

                    env.cpu.branch(entry_point_addr);
                    env.run_inner();

                    panic!("Main function exited unexpectedly!");
                })
            }));
            if let Err(e) = res {
                let panic_cell = env.panic_cell.clone();
                panic_cell.set(Some(env));
                std::panic::resume_unwind(e);
            }
            env
        });
        let main_thread = Thread {
            active: true,
            blocked_by: ThreadBlock::NotBlocked,
            return_value: None,
            guest_context: None,
            host_context: Some(main_thread_init_routine),
            stack: Some(mem::Mem::MAIN_THREAD_STACK_LOW_END..=0u32.wrapping_sub(1)),
        };

        let mut env = Environment {
            startup_time,
            bundle: NullableBox::new(bundle),
            fs: NullableBox::new(fs),
            window,
            mem: NullableBox::new(mem),
            bins,
            objc: NullableBox::new(objc),
            dyld: NullableBox::new(dyld),
            cpu: NullableBox::new(cpu),
            current_thread: 0,
            threads: vec![main_thread],
            libc_state: Default::default(),
            mutex_state: Default::default(),
            framework_state: Default::default(),
            options: NullableBox::new(options),
            gdb_server: None,
            env_vars: Default::default(),
            yielder: std::ptr::null(),
            remaining_ticks: None,
            panic_cell: Rc::new(Cell::new(None)),
        };

        env.set_up_initial_env_vars();

        dyld::Dyld::do_late_linking(&mut env);

        {
            let bin_path = env.bundle.executable_path();

            let envp_list: Vec<String> = env
                .env_vars
                .clone()
                .iter_mut()
                .map(|tuple| {
                    [
                        std::str::from_utf8(tuple.0).unwrap(),
                        "=",
                        env.mem.cstr_at_utf8(*tuple.1).unwrap(),
                    ]
                    .concat()
                })
                .collect();
            let envp_ref_list: Vec<&str> =
                envp_list.iter().map(|keyvalue| keyvalue.as_str()).collect();

            let bin_path_apple_key = format!("executable_path={}", bin_path.as_str());

            let argv = &[bin_path.as_str()];
            let envp = envp_ref_list.as_slice();
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
            env.gdb_server = Some(Box::new(gdb_server));
        }

        echo!("CPU emulation begins now.");

        Ok(env)
    }

    /// Set up the emulator environment without loading an app binary.
    ///
    /// This is a special mode that only exists to support the app picker, which
    /// uses the emulated environment to draw its UI and process input. Filling
    /// some of the fields with fake data is a hack, but it means the frameworks
    /// do not need to be aware of the app picker's peculiarities, so it is
    /// cleaner than the alternative!
    pub fn new_without_app(
        options: options::Options,
        icon: image::Image,
    ) -> Result<Environment, String> {
        let bundle = bundle::Bundle::new_fake_bundle();
        let fs = fs::Fs::new_fake_fs();

        let startup_time = Instant::now();

        let launch_image = None;

        assert!(!options.headless);
        let window = Some(Box::new(window::Window::new(
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
            Some(icon),
            launch_image,
            &options,
        )));

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
            guest_context: None,
            host_context: None,
            stack: Some(mem::Mem::MAIN_THREAD_STACK_LOW_END..=0u32.wrapping_sub(1)),
        };

        let mut env = Environment {
            startup_time,
            bundle: NullableBox::new(bundle),
            fs: NullableBox::new(fs),
            window,
            mem: NullableBox::new(mem),
            bins,
            objc: NullableBox::new(objc),
            dyld: NullableBox::new(dyld),
            cpu: NullableBox::new(cpu),
            current_thread: 0,
            threads: vec![main_thread],
            libc_state: Default::default(),
            mutex_state: Default::default(),
            framework_state: Default::default(),
            options: NullableBox::new(options),
            gdb_server: None,
            env_vars: Default::default(),
            yielder: std::ptr::null(),
            remaining_ticks: None,
            panic_cell: Rc::new(Cell::new(None)),
        };

        env.set_up_initial_env_vars();

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

    // BEFOREMERGE: Document why this is needed & safety
    unsafe fn new_fake() -> Self {
        Self {
            startup_time: Instant::now(),
            bundle: NullableBox::null(),
            fs: NullableBox::null(),
            window: None,
            mem: NullableBox::null(),
            bins: Vec::new(),
            objc: NullableBox::null(),
            dyld: NullableBox::null(),
            cpu: NullableBox::null(),
            current_thread: 0,
            threads: Vec::new(),
            libc_state: NullableBox::null(),
            framework_state: NullableBox::null(),
            mutex_state: NullableBox::null(),
            options: NullableBox::null(),
            gdb_server: None,
            env_vars: HashMap::new(),
            yielder: std::ptr::null(),
            remaining_ticks: None,
            panic_cell: Rc::new(Cell::new(None)),
        }
    }

    // BEFOREMERGE: uhh this is unsound, need to figure out how to deal w/ this
    pub fn run_with_yielder<F, T>(
        &mut self,
        yielder: &Yielder<Environment, Environment>,
        block: F,
    ) -> T
    where
        F: FnOnce(&mut Environment) -> T + 'static,
        T: 'static,
    {
        self.yielder = yielder;
        // We need to ensure panic safety here, so make sure to reset the
        // yielder if the inner function panics.
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| block(self)));
        self.yielder = std::ptr::null();
        match res {
            Ok(ret) => ret,
            Err(e) => {
                std::panic::resume_unwind(e);
            }
        }
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
            echo_no_panic!("Attempting to produce stack trace for main thread:");
        } else {
            echo_no_panic!(
                "Attempting to produce stack trace for thread {}:",
                self.current_thread
            );
        }
        let Some(stack_range) = self.threads[self.current_thread].stack.clone() else {
            echo_no_panic!("Could not get stack range for thread!");
            return;
        };
        echo_no_panic!(
            " 0. {:#x} (PC)",
            self.cpu.pc_with_thumb_bit().addr_with_thumb_bit()
        );
        let regs = self.cpu.regs();
        let mut lr = regs[cpu::Cpu::LR];
        let return_to_host_routine_addr = self.dyld.return_to_host_routine().addr_with_thumb_bit();
        if lr == return_to_host_routine_addr {
            echo_no_panic!(" 1. [host function] (LR)");
        } else {
            echo_no_panic!(" 1. {:#x} (LR)", lr);
        }
        let mut i = 2;
        let mut fp: mem::ConstPtr<u8> = mem::Ptr::from_bits(regs[abi::FRAME_POINTER]);
        loop {
            if !stack_range.contains(&fp.to_bits()) {
                echo_no_panic!("Next FP ({:?}) is outside the stack.", fp);
                break;
            }
            lr = self.mem.read((fp + 4).cast());
            fp = self.mem.read(fp.cast());
            if lr == return_to_host_routine_addr {
                echo_no_panic!("{:2}. [host function]", i);
            } else {
                echo_no_panic!("{:2}. {:#x}", i, lr);
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
        stack_size: GuestUSize,
    ) -> ThreadId {
        let stack_alloc = self.mem.alloc(stack_size);
        let stack_high_addr = stack_alloc.to_bits() + stack_size;
        assert!(stack_high_addr % 4 == 0);

        let thread_routine = Coroutine::new(move |yielder, mut env: Environment| {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                env.run_with_yielder(yielder, move |env| {
                    let regs = env.cpu.regs_mut();
                    regs[cpu::Cpu::SP] = stack_high_addr;
                    regs[0] = user_data.to_bits();

                    env.cpu.set_cpsr(
                        cpu::Cpu::CPSR_USER_MODE
                            | ((start_routine.is_thumb() as u32) * cpu::Cpu::CPSR_THUMB),
                    );
                    // Should this be moved?
                    let return_value: mem::MutVoidPtr =
                        start_routine.call_from_host(env, (user_data,));
                    let curr_thread = &mut env.threads[env.current_thread];
                    curr_thread.return_value = Some(return_value);
                    curr_thread.active = false;
                });
            }));
            if let Err(e) = res {
                let panic_cell = env.panic_cell.clone();
                panic_cell.set(Some(env));
                std::panic::resume_unwind(e);
            }
            env
        });

        self.threads.push(Thread {
            active: true,
            blocked_by: ThreadBlock::NotBlocked,
            return_value: None,
            guest_context: Some(Box::new(cpu::CpuContext::new())),
            host_context: Some(thread_routine),
            stack: Some(stack_alloc.to_bits()..=(stack_high_addr - 1)),
        });

        let new_thread_id = self.threads.len() - 1;

        log_dbg!("Created new thread {} with stack {:#x}–{:#x}, will execute function {:?} with data {:?}", new_thread_id, stack_alloc.to_bits(), (stack_high_addr - 1), start_routine, user_data);

        new_thread_id
    }

    // BEFOREMERGE: Are these functions (sleep to sem_increment) needed anymore?
    // Most of them are very light wrappers around yield_thread, and the ones
    // that aren't are probably better served by having their logic moved back
    // to the calling function.
    //
    /// Put the current thread to sleep for some duration, running other threads
    /// in the meantime as appropriate. Functions that call sleep right before
    /// they return back to the main run loop ([Environment::run]) should set
    /// `tail_call`.
    pub fn sleep(&mut self, duration: Duration) {
        log_dbg!(
            "Thread {} is going to sleep for {:?}.",
            self.current_thread,
            duration
        );
        let until = Instant::now().checked_add(duration).unwrap();
        self.yield_thread(ThreadBlock::Sleeping(until));
    }

    /// Block the current thread until the given mutex unlocks.
    ///
    /// Other threads also blocking on this mutex may get access first.
    /// Also note that like [Self::sleep], this only takes effect after the host
    /// function returns to the main run loop ([Environment::run]).
    pub fn block_on_mutex(&mut self, mutex_id: MutexId) {
        log_dbg!(
            "Thread {} blocking on mutex #{}.",
            self.current_thread,
            mutex_id
        );
        self.yield_thread(ThreadBlock::Mutex(mutex_id));
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
            log_dbg!(
                "Thread {} is blocking on semaphore {:?}",
                self.current_thread,
                sem
            );
            host_sem.waiting.insert(self.current_thread);
            std::mem::drop(host_sem);
            self.yield_thread(ThreadBlock::Semaphore(sem));
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
        log_dbg!(
            "Thread {} waiting for thread {} to finish.",
            self.current_thread,
            joinee_thread
        );
        self.yield_thread(ThreadBlock::Joining(joinee_thread, ptr));
    }

    /// Run the emulator. This is the main loop and won't return until app exit.
    /// Only `main.rs` should call this.
    //
    // BEFOREMERGE: This takes by value - since it never returns right now,
    // it's ok but it might be better to change?
    pub fn run(mut self) {
        let mut curr_host_context = self.threads[0].host_context.take().unwrap();
        let panic_cell = self.panic_cell.clone();
        let mut stepping = false;
        loop {
            if stepping {
                self.remaining_ticks = None;
            } else {
                // 100,000 ticks is an arbitrary number. It needs to be
                // reasonably large so we aren't jumping in and out of dynarmic
                // or trying to poll for events too often. At the same time,
                // very large values are bad for responsiveness.
                self.remaining_ticks = Some(100_000);
            }
            let mut kiil_current_thread = false;

            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                curr_host_context.resume(self)
            }));
            self = match res {
                Ok(ret) => match ret {
                    corosensei::CoroutineResult::Yield(env) => env,
                    corosensei::CoroutineResult::Return(env) => {
                        kiil_current_thread = true;
                        env
                    }
                },
                Err(e) => {
                    let Some(mut env) = panic_cell.take() else {
                        log_no_panic!("Did not recieve env from coroutine unwind, must abort!");
                        std::process::exit(-1)
                    };
                    echo!("Register state immediately after panic:");
                    env.cpu.dump_regs();
                    env.stack_trace();

                    // Put the host context back before resuming, the env will clean it up on drop.
                    let Some(thread) = env.threads.get_mut(env.current_thread) else {
                        log_no_panic!("Bad current_thread, must abort!");
                        std::process::exit(-1)
                    };
                    thread.host_context = Some(curr_host_context);
                    std::panic::resume_unwind(e);
                }
            };

            let mut old_context = if kiil_current_thread {
                log_dbg!("Killing thread {}", self.current_thread);
                panic_cell.set(Some(self));
                std::mem::drop(curr_host_context);
                let Some(env) = panic_cell.take() else {
                    log_no_panic!("Did not get env back from coroutine after drop, must abort!");
                    std::process::exit(-1);
                };
                self = env;
                None
            } else {
                Some(curr_host_context)
            };

            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // To maintain responsiveness when moving the window and so on,
                // we need to poll for events occasionally, even if the app
                // isn't actively processing them.
                // Polling for events can be quite expensive, so we shouldn't do
                // this until after we've done some amount of work on the guest
                // thread, lest every single callback call pay this cost.
                if let Some(ref mut window) = self.window {
                    window.poll_for_events(&self.options);
                }
                let curr_thread_block = self.threads[self.current_thread].blocked_by.clone();
                if stepping || matches!(curr_thread_block, ThreadBlock::WaitingForDebugger(_)) {
                    if old_context.is_none() {
                        let old_thread = self.current_thread;
                        let next_thread = self.schedule_next_thread();
                        self.switch_thread(&mut old_context, next_thread);
                        echo!(
                            "\nGDB WARNING ------- Thread {} has exited - switched thread to {}",
                            old_thread,
                            next_thread
                        );
                    }
                    match self.threads[self.current_thread].blocked_by {
                        ThreadBlock::NotBlocked | ThreadBlock::WaitingForDebugger(_) => {}
                        _ => {
                            let old_thread = self.current_thread;
                            let next_thread = self.schedule_next_thread();
                            self.switch_thread(&mut old_context, next_thread);
                            let block = &self.threads[old_thread].blocked_by;
                            echo!(
                                "\nGDB WARNING ------- Thread {} is blocked by {:?} - switched thread to {}",
                                old_thread,
                                block,
                                next_thread
                            );
                        }
                    }
                    let reason = if let ThreadBlock::WaitingForDebugger(reason) = curr_thread_block
                    {
                        self.threads[self.current_thread].blocked_by = ThreadBlock::NotBlocked;
                        reason.clone()
                    } else {
                        None
                    };
                    let will_step = self.gdb_server.as_deref_mut().unwrap().wait_for_debugger(
                        reason.clone(),
                        self.cpu.as_mut(),
                        self.mem.as_mut(),
                    );
                    if will_step {
                        stepping = true;
                    }
                }
            }));
            match res {
                Ok(_) => {}
                Err(e) => {
                    // Clean up the used host context. The ones inside the env
                    // are cleaned up by the drop handler.
                    let panic_cell = self.panic_cell.clone();
                    if let Some(ctx) = old_context {
                        panic_cell.set(Some(self));
                        std::mem::drop(ctx);
                        self = panic_cell.take().unwrap_or_else(|| {
                            log_no_panic!(
                            "Did not recieve env from coroutine unwind during salvage, must abort!"
                            );
                            std::process::exit(-1)
                        });
                        std::mem::drop(self);
                    };
                    std::panic::resume_unwind(e);
                }
            }

            // Don't switch threads if stepping.
            // BEFOREMERGE: Panic-catch should encapsulate these parts
            if stepping {
                curr_host_context = old_context.unwrap();
                continue;
            }

            stepping = false;

            let next_thread = self.schedule_next_thread();
            if next_thread != self.current_thread {
                self.switch_thread(&mut old_context, next_thread);
            }
            curr_host_context = old_context.unwrap();
        }
    }

    /// Run the emulator until the app returns control to the host. This is for
    /// host-to-guest function calls (see [abi::CallFromHost::call_from_host]).
    ///
    /// Note that this might execute code from other threads while waiting for
    /// the app to return control on the original thread!
    pub fn run_call(&mut self) {
        let old_thread = self.current_thread;
        self.run_inner();
        assert!(self.current_thread == old_thread);
    }

    // BEFOREMERGE: (Re)document
    fn switch_thread(&mut self, old_context: &mut Option<HostContext>, new_thread: ThreadId) {
        assert!(new_thread != self.current_thread);
        assert!(self.threads[new_thread].active);

        log_dbg!(
            "Switching thread: {} => {}",
            self.current_thread,
            new_thread
        );

        let mut guest_ctx = self.threads[new_thread].guest_context.take().unwrap();
        self.cpu.swap_context(&mut guest_ctx);
        assert!(self.threads[self.current_thread].guest_context.is_none());
        assert!(old_context.is_some() || !self.threads[self.current_thread].active);
        self.threads[self.current_thread].guest_context = Some(guest_ctx);

        let new_host_ctx = self.threads[new_thread].host_context.take().unwrap();
        self.threads[self.current_thread].host_context = old_context.take();
        *old_context = Some(new_host_ctx);
        self.current_thread = new_thread;
    }

    #[cold]
    /// Let the debugger handle a CPU error, or panic if there's no debugger
    /// connected. Returns [true] if the CPU should step and then resume
    /// debugging, or [false] if it should resume normal execution.
    fn debug_cpu_error(&mut self, error: cpu::CpuError) {
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
    // BEFOREMERGE: Redoc
    pub fn enter_debugger(&mut self, reason: Option<cpu::CpuError>) {
        // GDB doesn't seem to manage to produce a useful stack trace, so
        // let's print our own.
        self.stack_trace();

        self.yield_thread(ThreadBlock::WaitingForDebugger(reason));
    }

    #[inline(always)]
    /// Respond to the new CPU state (do nothing, execute an SVC or enter
    /// debugging) and decide what to do next.
    fn handle_cpu_state(&mut self, state: cpu::CpuState) -> ThreadNextAction {
        match state {
            cpu::CpuState::Normal => ThreadNextAction::Continue,
            cpu::CpuState::Svc(svc) => {
                // The program counter is pointing at the
                // instruction after the SVC, but we want the
                // address of the SVC itself.
                let svc_pc = self.cpu.regs()[cpu::Cpu::PC] - 4;
                match svc {
                    dyld::Dyld::SVC_RETURN_TO_HOST => {
                        assert!(
                            svc_pc == self.dyld.return_to_host_routine().addr_without_thumb_bit()
                        );
                        // Normal return from host-to-guest call.
                        ThreadNextAction::ReturnToHost
                    }
                    dyld::Dyld::SVC_LAZY_LINK | dyld::Dyld::SVC_LINKED_FUNCTIONS_BASE.. => {
                        if let Some(f) = self.dyld.get_svc_handler(
                            &self.bins,
                            &mut self.mem,
                            &mut self.cpu,
                            svc_pc,
                            svc,
                        ) {
                            f.call_from_guest(self);
                            ThreadNextAction::Continue
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

    fn run_inner(&mut self) {
        let initial_thread = self.current_thread;
        assert!(self.threads[initial_thread].active);
        assert!(self.threads[initial_thread].guest_context.is_none());

        loop {
            while self
                .remaining_ticks
                .is_none_or(|remaining_ticks| remaining_ticks > 0)
            {
                let state = self
                    .cpu
                    .run_or_step(&mut self.mem, self.remaining_ticks.as_mut());

                match self.handle_cpu_state(state) {
                    ThreadNextAction::Continue => {}
                    ThreadNextAction::ReturnToHost => return,
                    ThreadNextAction::DebugCpuError(e) => {
                        self.debug_cpu_error(e);
                    }
                }
                if self.remaining_ticks.is_none() {
                    break;
                }
            }
            self.yield_thread(ThreadBlock::NotBlocked);
        }
    }

    // BEFOREMERGE: redoc
    pub fn yield_thread(&mut self, thread_block: ThreadBlock) {
        assert!(!self.threads[self.current_thread].is_blocked());
        log_dbg!(
            "Thread {} yielding on {:?}",
            self.current_thread,
            thread_block
        );
        unsafe {
            self.threads[self.current_thread].blocked_by = thread_block;
            let yielder = self.yielder.as_ref().unwrap();
            self.yielder = std::ptr::null();
            let panic_cell = self.panic_cell.clone();
            // All global contexts should be saved and restored after yield.
            // Most functions that use global state don't yield anyways, but
            // it's good to make sure.
            //
            // BEFOREMERGE: It would probably be better to have "critical
            // sections" that can't yield for functions that use global state
            // objects instead so we don't have to pay the price of switching
            // contexts for every yield. We'd need to check that state objects
            // are managed properly, though.
            let al_context = crate::audio::openal::alcGetCurrentContext();
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let env = std::mem::replace(self, Self::new_fake());
                yielder.suspend(env)
            }));
            crate::audio::openal::alcMakeContextCurrent(al_context);
            match res {
                Ok(env) => {
                    let _ = std::mem::replace(self, env);
                    self.yielder = yielder;
                }
                Err(payload) => {
                    let Some(env) = panic_cell.take() else {
                        log_no_panic!("Did not recieve env for coroutine unwind, must abort!");
                        std::process::exit(-1)
                    };
                    let _ = std::mem::replace(self, env);
                    self.yielder = yielder;
                    std::panic::resume_unwind(payload);
                }
            }
        }
        assert!(!self.threads[self.current_thread].is_blocked());
    }

    //
    fn schedule_next_thread(&mut self) -> ThreadId {
        loop {
            // Try to find a new thread to execute, starting with the thread
            // following the one currently executing.
            let mut suitable_thread: Option<ThreadId> = None;
            let mut next_awakening: Option<Instant> = None;
            let mut mutex_to_relock: Option<MutexId> = None;
            for i in 0..self.threads.len() {
                let i = (self.current_thread + 1 + i) % self.threads.len();
                let candidate = &mut self.threads[i];

                if !candidate.active {
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
                    ThreadBlock::Condition(cond) => {
                        let host_cond = self
                            .libc_state
                            .pthread
                            .cond
                            .condition_variables
                            .get(&cond)
                            .unwrap();
                        if host_cond.done {
                            log_dbg!(
                                "Thread {} is unblocking on cond var {:?}.",
                                self.current_thread,
                                cond
                            );
                            self.threads[i].blocked_by = ThreadBlock::NotBlocked;
                            suitable_thread = Some(i);
                            let used_mutex =
                                self.libc_state.pthread.cond.mutexes.remove(&cond).unwrap();
                            mutex_to_relock = Some(used_mutex.mutex_id);
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
                                self.mem
                                    .write(ptr, self.threads[joinee_thread].return_value.unwrap());
                            }
                            self.threads[i].blocked_by = ThreadBlock::NotBlocked;
                            suitable_thread = Some(i);
                            break;
                        }
                    }
                    ThreadBlock::NotBlocked => {
                        suitable_thread = Some(i);
                        break;
                    }
                    ThreadBlock::WaitingForDebugger(_) => unreachable!(),
                }
            }

            // There's a suitable thread we can switch to immediately.
            if let Some(suitable_thread) = suitable_thread {
                if let Some(mutex_id) = mutex_to_relock {
                    self.relock_unblocked_mutex_for_thread(suitable_thread, mutex_id);
                }
                return suitable_thread;
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

    fn set_up_initial_env_vars(&mut self) {
        // TODO: Provide all the system environment variables an app might
        // expect to find.

        // Initialize HOME envvar
        let home_value_cstr = self
            .mem
            .alloc_and_write_cstr(self.fs.home_directory().as_str().as_bytes());
        self.env_vars.insert(b"HOME".to_vec(), home_value_cstr);
    }

    // BEFOREMERGE: Document need for function
    fn salvage(mut self) -> mem::Mem {
        if !self
            .threads
            .iter()
            .all(|thread| thread.host_context.is_none())
        {
            let panic_cell = self.panic_cell.clone();
            let threads_len = self.threads.len();
            for i in 0..threads_len {
                let host_context = self.threads[i].host_context.take();
                panic_cell.set(Some(self));
                std::mem::drop(host_context);
                self = panic_cell.take().unwrap_or_else(|| {
                    log_no_panic!(
                        "Did not recieve env from coroutine unwind during salvage, must abort!"
                    );
                    std::process::exit(-1)
                });
            }
        }
        unsafe {
            let mem = std::mem::replace(&mut self.mem, NullableBox::null());
            // Safe to drop env now since all the host contexts are dropped.
            std::mem::drop(self);
            mem.into_inner()
        }
    }

    // BEFOREMERGE: doc
    pub fn on_parent_stack_in_coroutine<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut window::Window, &mut options::Options) -> R + Send,
    {
        struct WindowWrapper<'a> {
            window: &'a mut window::Window,
        }
        unsafe impl Send for WindowWrapper<'_> {}

        if !self.yielder.is_null() {
            unsafe {
                let yielder = self.yielder.as_ref().unwrap();
                let wrapped = WindowWrapper {
                    window: self.window.as_mut().unwrap(),
                };
                yielder.on_parent_stack(|| {
                    let wrapped = wrapped;
                    f(wrapped.window, self.options.as_mut())
                })
            }
        } else {
            f(self.window.as_mut().unwrap(), self.options.as_mut())
        }
    }
}

impl Drop for Environment {
    // Clean up all the remaining HostContexts. This isn't strictly required,
    // since this should only occur after a sucessful panic, but it is a bit
    // cleaner and avoids confucion inside the logs.
    fn drop(&mut self) {
        if self.threads.is_empty()
            || self
                .threads
                .iter()
                .all(|thread| thread.host_context.is_none())
        {
            return;
        }
        unsafe {
            let mut env = std::mem::replace(self, Environment::new_fake());
            let panic_cell = env.panic_cell.clone();
            let threads_len = env.threads.len();
            for i in 0..threads_len {
                let host_context = env.threads[i].host_context.take();
                panic_cell.set(Some(env));
                std::mem::drop(host_context);
                env = panic_cell.take().unwrap_or_else(|| {
                    log_no_panic!(
                        "Did not recieve env from coroutine unwind during drop, must abort!"
                    );
                    std::process::exit(-1)
                });
            }
            *self = env;
        }
    }
}
