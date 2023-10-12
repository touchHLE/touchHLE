/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Dynamic linker.
//!
//! iPhone OS's dynamic linker, `dyld`, is the namesake of this module.
//!
//! This is where the magic of "high-level emulation" can begin to happen.
//! The guest app will reference various functions, constants, classes etc from
//! iPhone OS's system frameworks (i.e. dynamically-linked libraries), but
//! instead of actually loading and linking the original framework binaries,
//! this "dynamic linker" will generate appropriate stubs for calling into
//! touchHLE's own implementations of the frameworks, which are "host code"
//! (i.e. not themselves running under emulation).
//!
//! This also does normal dynamic linking for libgcc and libstdc++. It might
//! eventually support linking other things too.
//!
//! See [crate::mach_o] for resources.

mod constant_lists;
mod function_lists;

use crate::abi::{CallFromGuest, GuestFunction};
use crate::cpu::Cpu;
use crate::frameworks::foundation::ns_string;
use crate::mach_o::{MachO, SectionType};
use crate::mem::{ConstVoidPtr, GuestUSize, Mem, MutPtr, Ptr};
use crate::objc::{nil, ObjC};
use crate::Environment;
use std::collections::HashMap;

type HostFunction = &'static dyn CallFromGuest;

/// Type for lists of functions exported by host implementations of frameworks.
///
/// Each module that wants to expose functions to guest code should export a
/// constant using this type, e.g.:
///
/// ```ignore
/// pub const FUNCTIONS: FunctionExports = &[
///    ("_NSFoo", &/* ... */),
///    ("_NSBar", &/* ... */),
///    /* ... */
/// ];
/// ```
///
/// The strings are the mangled symbol names. For C functions, this is just the
/// name prefixed with an underscore.
///
/// For convenience, use [export_c_func]:
///
/// ```ignore
/// pub const FUNCTIONS: FunctionExports = &[
///     export_c_func!(NSFoo(_, _)),
///     export_c_func!(NSBar()),
/// ];
/// ```
///
/// See also [ConstantExports] and [crate::objc::ClassExports].
pub type FunctionExports = &'static [(&'static str, HostFunction)];

/// Macro for exporting a function with C-style name mangling. See
/// [FunctionExports].
///
/// ```ignore
/// export_c_func!(NSFoo(_, _))
/// ```
///
/// will desugar to:
///
/// ```ignore
/// ("_NSFoo", &(NSFoo as (&mut Environment, _, _) -> _))
/// ```
///
/// The function needs to be explicitly casted because a bare function reference
/// defaults to a different type than a pure fn pointer, which is the type that
/// [CallFromGuest] is implemented on. This macro will do the casting for you,
/// but you will need to supply an underscore for each parameter.
#[macro_export]
macro_rules! export_c_func {
    ($name:ident ($($_:ty),*)) => {
        (
            concat!("_", stringify!($name)),
            &($name as fn(&mut $crate::Environment, $($_),*) -> _)
        )
    };
}
pub use crate::export_c_func; // #[macro_export] is weird...

/// Type for describing a constant (C `extern const` symbol) that will be
/// created by the linker if the guest app references it. See [ConstantExports].
pub enum HostConstant {
    NSString(&'static str),
    NullPtr,
    Custom(fn(&mut Mem) -> ConstVoidPtr),
}

/// Type for lists of constants exported by host implementations of frameworks.
///
/// Each module that wants to expose functions to guest code should export a
/// constant using this type, e.g.:
///
/// ```ignore
/// pub const CONSTANT: ConstantExports = &[
///    ("_kNSFooBar", HostConstant::NSString("NSFooBar")),
///    /* ... */
/// ];
/// ```
///
/// The strings are the mangled symbol names. For C constants, this is just the
/// name prefixed with an underscore.
///
/// See also [FunctionExports], [crate::objc::ClassExports].
pub type ConstantExports = &'static [(&'static str, HostConstant)];

/// Helper for working with symbol lists in the style of [FunctionExports].
pub fn search_lists<T>(
    lists: &'static [&'static [(&'static str, T)]],
    symbol: &str,
) -> Option<&'static (&'static str, T)> {
    lists
        .iter()
        .flat_map(|&n| n)
        .find(|&(sym, _)| *sym == symbol)
}

fn encode_a32_svc(imm: u32) -> u32 {
    assert!(imm & 0xff000000 == 0);
    imm | 0xef000000
}
fn encode_a32_ret() -> u32 {
    0xe12fff1e
}
fn encode_a32_trap() -> u32 {
    0xe7ffdefe
}

fn write_return_to_host_routine(mem: &mut Mem, svc: u32) -> GuestFunction {
    let routine = [
        encode_a32_svc(svc),
        // When a return-to-host occurs, it's the host's responsibility
        // to reset the PC to somewhere else. So something has gone
        // wrong if this is executed.
        encode_a32_trap(),
    ];
    let ptr: MutPtr<u32> = mem.alloc(4 * 2).cast();
    mem.write(ptr + 0, routine[0]);
    mem.write(ptr + 1, routine[1]);
    let ptr = GuestFunction::from_addr_with_thumb_bit(ptr.to_bits());
    assert!(!ptr.is_thumb());
    ptr
}
pub struct Dyld {
    /// List of host functions that have been "linked" and had SVCs assigned.
    ///
    /// The `&'static str` part here is purely for debugging and could be
    /// removed in release builds if it's ever necessary.
    linked_host_functions: Vec<(&'static str, HostFunction)>,
    return_to_host_routine: Option<GuestFunction>,
    thread_exit_routine: Option<GuestFunction>,
    constants_to_link_later: Vec<(MutPtr<ConstVoidPtr>, &'static HostConstant)>,
}

impl Dyld {
    /// We reserve this SVC ID for invoking the lazy linker.
    pub const SVC_LAZY_LINK: u32 = 0;
    /// We reserve this SVC ID for the exit routine for spawned threads.
    pub const SVC_THREAD_EXIT: u32 = 1;
    /// We reserve this SVC ID for the special return-to-host routine.
    pub const SVC_RETURN_TO_HOST: u32 = 2;
    /// The range of SVC IDs `SVC_LINKED_FUNCTIONS_BASE..` is used to reference
    /// [Self::linked_host_functions] entries.
    pub const SVC_LINKED_FUNCTIONS_BASE: u32 = Self::SVC_RETURN_TO_HOST + 1;

    const SYMBOL_STUB_INSTRUCTIONS: [u32; 2] = [0xe59fc000, 0xe59cf000];
    const PIC_SYMBOL_STUB_INSTRUCTIONS: [u32; 3] = [0xe59fc004, 0xe08fc00c, 0xe59cf000];

    pub fn new() -> Dyld {
        Dyld {
            linked_host_functions: Vec::new(),
            return_to_host_routine: None,
            thread_exit_routine: None,
            constants_to_link_later: Vec::new(),
        }
    }

    pub fn return_to_host_routine(&self) -> GuestFunction {
        self.return_to_host_routine.unwrap()
    }

    pub fn thread_exit_routine(&self) -> GuestFunction {
        self.thread_exit_routine.unwrap()
    }

    /// Do linking-related tasks that need doing right after loading the
    /// binaries.
    pub fn do_initial_linking(&mut self, bins: &[MachO], mem: &mut Mem, objc: &mut ObjC) {
        assert!(self.return_to_host_routine.is_none());
        assert!(self.thread_exit_routine.is_none());
        self.return_to_host_routine =
            Some(write_return_to_host_routine(mem, Self::SVC_RETURN_TO_HOST));
        self.thread_exit_routine = Some(write_return_to_host_routine(mem, Self::SVC_THREAD_EXIT));

        // Currently assuming only the app binary contains Objective-C things.

        objc.register_bin_selectors(&bins[0], mem);
        objc.register_host_selectors(mem);

        for bin in bins {
            self.setup_lazy_linking(bin, mem);
            // Must happen before `register_bin_classes`, else superclass
            // pointers will be wrong.
            self.do_non_lazy_linking(bin, bins, mem, objc);
        }

        objc.register_bin_classes(&bins[0], mem);
        objc.register_bin_categories(&bins[0], mem);

        ns_string::register_constant_strings(&bins[0], mem, objc);
    }

    /// [Self::do_initial_linking] but for when this is the app picker's special
    /// environment with no binary (see [crate::Environment::new_without_app]).
    pub fn do_initial_linking_with_no_bins(&mut self, mem: &mut Mem, objc: &mut ObjC) {
        assert!(self.return_to_host_routine.is_none());
        assert!(self.thread_exit_routine.is_none());
        self.return_to_host_routine =
            Some(write_return_to_host_routine(mem, Self::SVC_RETURN_TO_HOST));
        self.thread_exit_routine = Some(write_return_to_host_routine(mem, Self::SVC_THREAD_EXIT));

        objc.register_host_selectors(mem);
    }

    /// Set up lazy-linking stubs for a loaded binary.
    ///
    /// Dynamic linking of functions on iPhone OS usually happens "lazily",
    /// which means that the linking is delayed until the function is first
    /// called. This is achieved by using stub functions. Instead of calling the
    /// external function directly, the app code will call a stub function, and
    /// that stub will either jump to the dynamic linker (which will link in the
    /// external function and then jump to it), or on subsequent calls, jump
    /// straight to the external function.
    ///
    /// These stubs already exist in the binary, but they need to be rewritten
    /// so that they will invoke our dynamic linker.
    fn setup_lazy_linking(&self, bin: &MachO, mem: &mut Mem) {
        let Some(stubs) = bin.get_section(SectionType::SymbolStubs) else {
            return;
        };

        let entry_size = stubs.dyld_indirect_symbol_info.as_ref().unwrap().entry_size;

        // two or three A32 instructions (PIC stub needs one more) followed by
        // the address or offset of the corresponding __la_symbol_ptr
        let expected_instructions = match entry_size {
            12 => Self::SYMBOL_STUB_INSTRUCTIONS.as_slice(),
            16 => Self::PIC_SYMBOL_STUB_INSTRUCTIONS.as_slice(),
            _ => unimplemented!(),
        };

        assert!(stubs.size % entry_size == 0);
        let stub_count = stubs.size / entry_size;
        for i in 0..stub_count {
            let ptr: MutPtr<u32> = Ptr::from_bits(stubs.addr + i * entry_size);

            for (j, &instr) in expected_instructions.iter().enumerate() {
                assert!(mem.read(ptr + j.try_into().unwrap()) == instr);
            }

            mem.write(ptr + 0, encode_a32_svc(Self::SVC_LAZY_LINK));
            // For convenience, make the stub return once the SVC is done
            // (Otherwise we'd have to manually update the PC)
            mem.write(ptr + 1, encode_a32_ret());
            if entry_size == 16 {
                // This is preceded by a return instruction, so if we do execute
                // it, something has gone wrong.
                mem.write(ptr + 2, encode_a32_trap());
            }
            // Leave the __la_symbol_ptr intact in case we want to link it to
            // a real symbol later.
        }
    }

    /// Link non-lazy symbols for a loaded binary.
    ///
    /// These are usually constants, Objective-C classes, or vtable pointers.
    /// Since the linking must be done upfront, we can't in general delay errors
    /// about missing implementations until the point of use. For that reason,
    /// this will spit out a warning to stderr for everything missing, so that
    /// there's at least some indication about why the emulator might crash.
    ///
    /// `bin` is the binary to link non-lazy symbols for, `bins` is the set of
    /// binaries symbols may be looked up in.
    fn do_non_lazy_linking(&mut self, bin: &MachO, bins: &[MachO], mem: &mut Mem, objc: &mut ObjC) {
        let mut unhandled_relocations: HashMap<&str, Vec<u32>> = HashMap::new();
        for &(ptr_ptr, ref name) in &bin.external_relocations {
            let ptr_ptr: MutPtr<ConstVoidPtr> = Ptr::from_bits(ptr_ptr);
            // There will be an existing value at the address, which is an
            // offset that should be applied to the external symbol's address.
            // It is often 0, but not always.
            let offset: u32 = mem.read(ptr_ptr).to_bits();
            let target: ConstVoidPtr = if let Some(name) = name.strip_prefix("_OBJC_CLASS_$_") {
                objc.link_class(name, /* is_metaclass: */ false, mem)
                    .cast()
                    .cast_const()
            } else if let Some(name) = name.strip_prefix("_OBJC_METACLASS_$_") {
                objc.link_class(name, /* is_metaclass: */ true, mem)
                    .cast()
                    .cast_const()
            } else if name == "___CFConstantStringClassReference" {
                // See ns_string::register_constant_strings
                nil.cast().cast_const()
            } else if name == "__objc_empty_vtable" || name == "__objc_empty_cache" {
                // Our Objective-C runtime doesn't use these
                Ptr::null()
            } else if let Some(&external_addr) = bins
                .iter()
                .flat_map(|other_bin| other_bin.exported_symbols.get(name))
                .next()
            {
                // Often used for C++ RTTI
                Ptr::from_bits(external_addr)
            } else {
                unhandled_relocations
                    .entry(name)
                    .or_default()
                    .push(ptr_ptr.to_bits());
                continue;
            };
            // wrapping_add() is used in case the offset is negative. I haven't
            // seen it happen, but it would make sense if that is allowed.
            mem.write(
                ptr_ptr,
                Ptr::from_bits(target.to_bits().wrapping_add(offset)),
            )
        }
        // Collecting unhandled relocations for the same symbol onto one line
        // makes the log output much less spammy.
        for (name, addrs) in unhandled_relocations {
            log!(
                "Warning: unhandled external relocation {:?} in {:?} at {}",
                name,
                bin.name,
                addrs
                    .into_iter()
                    .map(|addr| format!("{:#x}", addr))
                    .collect::<Vec<String>>()
                    .join(", "),
            );
        }

        let Some(ptrs) = bin.get_section(SectionType::NonLazySymbolPointers) else {
            return;
        };
        let info = ptrs.dyld_indirect_symbol_info.as_ref().unwrap();

        let entry_size = info.entry_size;
        assert!(entry_size == 4);
        assert!(ptrs.size % entry_size == 0);
        let ptr_count = ptrs.size / entry_size;
        'ptr_loop: for i in 0..ptr_count {
            let Some(symbol) = info.indirect_undef_symbols[i as usize].as_deref() else {
                continue;
            };

            let ptr_ptr: MutPtr<ConstVoidPtr> = Ptr::from_bits(ptrs.addr + i * entry_size);

            for other_bin in bins {
                if let Some(&addr) = other_bin.exported_symbols.get(symbol) {
                    mem.write(ptr_ptr, Ptr::from_bits(addr));
                    continue 'ptr_loop;
                }
            }

            if let Some((_, template)) = search_lists(constant_lists::CONSTANT_LISTS, symbol) {
                // Delay linking of constant until we have a `&mut Environment`,
                // that makes it much easier to build NSString objects etc.
                self.constants_to_link_later.push((ptr_ptr, template));
                continue;
            }

            log!(
                "Warning: unhandled non-lazy symbol {:?} at {:?} in \"{}\"",
                symbol,
                ptr_ptr,
                bin.name
            );
        }

        // FIXME: check for internal relocations?
    }

    /// Do linking that can only be done once there is a full [Environment].
    /// Not to be confused with lazy linking.
    pub fn do_late_linking(env: &mut Environment) {
        // TODO: do symbols ever appear in __nl_symbol_ptr multiple times?

        let to_link = std::mem::take(&mut env.dyld.constants_to_link_later);
        for (symbol_ptr_ptr, template) in to_link {
            let symbol_ptr: ConstVoidPtr = match template {
                HostConstant::NSString(static_str) => {
                    let string_ptr = ns_string::get_static_str(env, static_str);
                    let string_ptr_ptr = env.mem.alloc_and_write(string_ptr);
                    string_ptr_ptr.cast().cast_const()
                }
                HostConstant::NullPtr => {
                    let null_ptr: ConstVoidPtr = Ptr::null();
                    let null_ptr_ptr = env.mem.alloc_and_write(null_ptr);
                    null_ptr_ptr.cast().cast_const()
                }
                HostConstant::Custom(f) => f(&mut env.mem),
            };
            env.mem.write(symbol_ptr_ptr, symbol_ptr.cast());
        }
    }

    /// Return a host function that can be called to handle an SVC instruction
    /// encountered during CPU emulation. If `None` is returned, the execution
    /// needs to resume at `svc_pc`.
    pub fn get_svc_handler(
        &mut self,
        bins: &[MachO],
        mem: &mut Mem,
        cpu: &mut Cpu,
        svc_pc: u32,
        svc: u32,
    ) -> Option<HostFunction> {
        match svc {
            Self::SVC_LAZY_LINK => self.do_lazy_link(bins, mem, cpu, svc_pc),
            Self::SVC_THREAD_EXIT | Self::SVC_RETURN_TO_HOST => unreachable!(), // don't handle here
            Self::SVC_LINKED_FUNCTIONS_BASE.. => {
                let f = self
                    .linked_host_functions
                    .get((svc - Self::SVC_LINKED_FUNCTIONS_BASE) as usize);
                let Some(&(symbol, f)) = f else {
                    panic!("Unexpected SVC #{} at {:#x}", svc, svc_pc);
                };
                log_dbg!("Call to host function, already linked: {}", symbol);
                Some(f)
            }
        }
    }

    fn do_lazy_link(
        &mut self,
        bins: &[MachO],
        mem: &mut Mem,
        cpu: &mut Cpu,
        svc_pc: u32,
    ) -> Option<HostFunction> {
        let stubs = bins
            .iter()
            .flat_map(|bin| bin.get_section(SectionType::SymbolStubs))
            .find(|stubs| (stubs.addr..(stubs.addr + stubs.size)).contains(&svc_pc))
            .unwrap();

        let info = stubs.dyld_indirect_symbol_info.as_ref().unwrap();

        let offset = svc_pc - stubs.addr;
        assert!(offset % info.entry_size == 0);
        let idx = (offset / info.entry_size) as usize;

        let symbol = info.indirect_undef_symbols[idx].as_deref().unwrap();

        if let Some(&(symbol, f)) = search_lists(function_lists::FUNCTION_LISTS, symbol) {
            // Allocate an SVC ID for this host function
            let idx: u32 = self.linked_host_functions.len().try_into().unwrap();
            let svc = idx + Self::SVC_LINKED_FUNCTIONS_BASE;
            self.linked_host_functions.push((symbol, f));

            // Rewrite stub function to call this host function
            let stub_function_ptr: MutPtr<u32> = Ptr::from_bits(svc_pc);
            mem.write(stub_function_ptr, encode_a32_svc(svc));
            assert!(mem.read(stub_function_ptr + 1) == encode_a32_ret());

            cpu.invalidate_cache_range(stub_function_ptr.to_bits(), 4);

            log_dbg!(
                "Linked {} at {:?} to host implementation",
                symbol,
                stub_function_ptr
            );

            // Return the host function so that we can call it now that we're
            // done.
            return Some(f);
        }

        for dylib in &bins[1..] {
            if let Some(&addr) = dylib.exported_symbols.get(symbol) {
                let original_instructions = match info.entry_size {
                    12 => Self::SYMBOL_STUB_INSTRUCTIONS.as_slice(),
                    16 => Self::PIC_SYMBOL_STUB_INSTRUCTIONS.as_slice(),
                    _ => unreachable!(),
                };
                let instruction_count: GuestUSize = original_instructions.len().try_into().unwrap();

                // Restore the original stub, which calls the __la_symbol_ptr
                let stub_function_ptr: MutPtr<u32> = Ptr::from_bits(svc_pc);
                for (i, &instr) in original_instructions.iter().enumerate() {
                    mem.write(stub_function_ptr + i.try_into().unwrap(), instr)
                }

                cpu.invalidate_cache_range(stub_function_ptr.to_bits(), instruction_count * 4);

                // Update the __la_symbol_ptr
                let la_symbol_ptr: MutPtr<u32> = if info.entry_size == 12 {
                    // Normal stub: absolute address
                    let addr = mem.read(stub_function_ptr + instruction_count);
                    Ptr::from_bits(addr)
                } else {
                    // The PIC (position-independent code) stub uses a
                    // PC-relative offset rather than an absolute address.
                    let offset = mem.read(stub_function_ptr + instruction_count);
                    Ptr::from_bits(stub_function_ptr.to_bits() + offset + 12)
                };
                mem.write(la_symbol_ptr, addr);

                log_dbg!(
                    "Linked {} at {:?}/{:?} to {:#x} from {}",
                    symbol,
                    stub_function_ptr,
                    la_symbol_ptr,
                    addr,
                    dylib.name
                );

                // Tell the caller it needs to restart execution at svc_pc.
                return None;
            }
        }

        panic!("Call to unimplemented function {}", symbol);
    }

    /// Creates a guest function that will call a host function with the name
    /// `symbol`. This can be used to implement "get proc address" functions.
    /// Note that no attempt is made to deduplicate or deallocate these, so
    /// excessive use would create a memory leak.
    ///
    /// The name must be the mangled symbol name. Returns [Err] if there's no
    /// such function.
    pub fn create_proc_address(
        &mut self,
        mem: &mut Mem,
        cpu: &mut Cpu,
        symbol: &str,
    ) -> Result<GuestFunction, ()> {
        let &(symbol, f) = search_lists(function_lists::FUNCTION_LISTS, symbol).ok_or(())?;

        // Allocate an SVC ID for this host function
        let idx: u32 = self.linked_host_functions.len().try_into().unwrap();
        let svc = idx + Self::SVC_LINKED_FUNCTIONS_BASE;
        self.linked_host_functions.push((symbol, f));

        // Create guest function to call this host function
        let function_ptr = mem.alloc(8);
        let function_ptr: MutPtr<u32> = function_ptr.cast();
        mem.write(function_ptr + 0, encode_a32_svc(svc));
        mem.write(function_ptr + 1, encode_a32_ret());

        // Just in case
        cpu.invalidate_cache_range(function_ptr.to_bits(), 4);

        Ok(GuestFunction::from_addr_with_thumb_bit(
            function_ptr.to_bits(),
        ))
    }

    /// Same as `create_proc_address`, but used for internal touchHLE needs.
    /// For example, to create an invocation function for NSThread
    /// implementation.
    ///
    /// The name must be the mangled symbol name. Returns [Err] if there's no
    /// such function in a list of private functions.
    pub fn create_private_proc_address(
        &mut self,
        mem: &mut Mem,
        cpu: &mut Cpu,
        symbol: &str,
    ) -> Result<GuestFunction, ()> {
        let &(symbol, f) = search_lists(function_lists::PRIVATE_FUNCTION_LISTS, symbol).ok_or(())?;

        // Allocate an SVC ID for this host function
        let idx: u32 = self.linked_host_functions.len().try_into().unwrap();
        let svc = idx + Self::SVC_LINKED_FUNCTIONS_BASE;
        self.linked_host_functions.push((symbol, f));

        // Create guest function to call this host function
        let function_ptr = mem.alloc(8);
        let function_ptr: MutPtr<u32> = function_ptr.cast();
        mem.write(function_ptr + 0, encode_a32_svc(svc));
        mem.write(function_ptr + 1, encode_a32_ret());

        // Just in case
        cpu.invalidate_cache_range(function_ptr.to_bits(), 4);

        Ok(GuestFunction::from_addr_with_thumb_bit(
            function_ptr.to_bits(),
        ))
    }
}
