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
//! See [crate::mach_o] for resources.

use crate::abi::CallFromGuest;
use crate::mach_o::MachO;
use crate::memory::{Memory, MutPtr, Ptr};
use crate::objc::ObjC;

type HostFunction = &'static dyn CallFromGuest;

/// Type for lists of functions exported by host implementations of frameworks.
///
/// Each module that wants to expose functions to guest code should export a
/// constant using this type, e.g.:
///
/// ```
/// pub const FUNCTIONS: FunctionExports = &[
///    ("_NSFoo", &/* ... */),
///    ("_NSBar", &/* ... */),
///    /* ... */
/// ];
/// ```
///
/// The strings are the mangled symbol names. For C functions, this is just the
/// name prefixed with an underscore.
pub type FunctionExports = &'static [(&'static str, HostFunction)];

/// All the lists of functions that the linker should search through.
const FUNCTION_LISTS: &[FunctionExports] = &[crate::objc::FUNCTIONS];

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

pub struct Dyld {
    linked_host_functions: Vec<HostFunction>,
}

impl Dyld {
    pub fn new() -> Dyld {
        Dyld {
            linked_host_functions: Vec::new(),
        }
    }

    /// Do linking-related tasks that need doing right after loading a binary.
    pub fn do_initial_linking(&self, bin: &MachO, mem: &mut Memory, objc: &mut ObjC) {
        self.setup_lazy_linking(bin, mem);
        self.do_non_lazy_linking(bin, mem, objc);
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
    fn setup_lazy_linking(&self, bin: &MachO, mem: &mut Memory) {
        let Some(stubs) = bin.get_section("__symbol_stub4") else {
            return;
        };

        let entry_size = stubs.dyld_indirect_symbol_info.as_ref().unwrap().entry_size;

        assert!(entry_size == 12); // should be three A32 instructions
        assert!(stubs.size % entry_size == 0);
        let stub_count = stubs.size / entry_size;
        for i in 0..stub_count {
            let ptr: MutPtr<u32> = Ptr::from_bits(stubs.addr + i * entry_size);
            // Let's reserve SVC #0 for calling the dynamic linker
            mem.write(ptr + 0, encode_a32_svc(0));
            // For convenience, make the stub return once the SVC is done
            // (Otherwise we'd have to manually update the PC)
            mem.write(ptr + 1, encode_a32_ret());
            // This is preceded by a return instruction, so if we do execute it,
            // something has gone wrong.
            mem.write(ptr + 2, encode_a32_trap());
        }
    }

    /// Link non-lazy symbols for a loaded binary.
    ///
    /// These are usually constants, Objective-C classes, or vtable pointers.
    /// Since the linking must be done upfront, we can't in general delay errors
    /// about missing implementations until the point of use. For that reason,
    /// this will spit out a warning to stderr for everything missing, so that
    /// there's at least some indication about why the emulator might crash.
    fn do_non_lazy_linking(&self, bin: &MachO, mem: &mut Memory, objc: &mut ObjC) {
        for &(ptr_ptr, ref name) in &bin.external_relocations {
            let ptr = if let Some(name) = name.strip_prefix("_OBJC_CLASS_$_") {
                objc.link_class(name, /* is_metaclass: */ false, mem)
            } else if let Some(name) = name.strip_prefix("_OBJC_METACLASS_$_") {
                objc.link_class(name, /* is_metaclass: */ true, mem)
            } else {
                // TODO: look up symbol in host implementations, write pointer
                eprintln!(
                    "Warning: unhandled external relocation {:?} at {:#x}",
                    name, ptr_ptr
                );
                continue;
            };
            mem.write(Ptr::from_bits(ptr_ptr), ptr)
        }

        let Some(ptrs) = bin.get_section("__nl_symbol_ptr") else {
            return;
        };
        let info = ptrs.dyld_indirect_symbol_info.as_ref().unwrap();

        let entry_size = info.entry_size;
        assert!(entry_size == 4);
        assert!(ptrs.size % entry_size == 0);
        let ptr_count = ptrs.size / entry_size;
        for i in 0..ptr_count {
            let Some(symbol) = info.indirect_undef_symbols[i as usize].as_deref() else {
                continue;
            };

            let ptr = ptrs.addr + i * entry_size;

            // TODO: look up symbol in host implementations, write pointer
            eprintln!(
                "Warning: unhandled non-lazy symbol {:?} at {:#x}",
                symbol, ptr
            );
        }
    }

    /// Return a host function that can be called to handle an SVC instruction
    /// encountered during CPU emulation.
    pub fn get_svc_handler(
        &mut self,
        bin: &MachO,
        mem: &mut Memory,
        current_instruction: u32,
        svc: u32,
    ) -> HostFunction {
        match svc {
            0 => self.do_lazy_link(bin, mem, current_instruction),
            _ => {
                let f = self.linked_host_functions.get((svc - 1) as usize);
                let Some(&f) = f else {
                    panic!("Unexpected SVC #{} at {:#x}", svc, current_instruction);
                };
                f
            }
        }
    }

    fn do_lazy_link(
        &mut self,
        bin: &MachO,
        mem: &mut Memory,
        current_instruction: u32,
    ) -> HostFunction {
        let stubs = bin.get_section("__symbol_stub4").unwrap();
        let info = stubs.dyld_indirect_symbol_info.as_ref().unwrap();

        assert!((stubs.addr..(stubs.addr + stubs.size)).contains(&current_instruction));
        let offset = current_instruction - stubs.addr;
        assert!(offset % info.entry_size == 0);
        let idx = (offset / info.entry_size) as usize;

        let symbol = info.indirect_undef_symbols[idx].as_deref().unwrap();

        let Some(&(_, f)) = FUNCTION_LISTS.iter().flat_map(|&n| n).find(|&(sym, _)| {
            *sym == symbol
        }) else {
            panic!("Call to unimplemented function {}", symbol);
        };

        // Allocate an SVC ID for this host function
        let idx: u32 = self.linked_host_functions.len().try_into().unwrap();
        let svc = idx + 1;
        self.linked_host_functions.push(f);

        // Rewrite stub function to call this host function
        let stub_function_ptr: MutPtr<u32> = Ptr::from_bits(current_instruction);
        mem.write(stub_function_ptr, encode_a32_svc(svc));
        assert!(mem.read(stub_function_ptr + 1) == encode_a32_ret());

        // Return the host function so that we can call it now that we're done.
        f
    }
}
