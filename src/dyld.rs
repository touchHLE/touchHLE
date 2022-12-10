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

use crate::mach_o::MachO;
use crate::memory::{Memory, MutPtr, Ptr};

pub struct Dyld {}

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

impl Dyld {
    pub fn new() -> Dyld {
        Dyld {}
    }

    /// Do linking-related tasks that need doing right after loading a binary.
    pub fn do_initial_linking(&self, bin: &MachO, mem: &mut Memory) {
        self.setup_lazy_linking(bin, mem);
        self.do_non_lazy_linking(bin, mem);
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
    /// These are usually constants or Objective-C classes. Since the linking
    /// must be done upfront, we can't in general delay errors about missing
    /// implementations until the point of use. For that reason, this will spit
    /// out a warning to stderr for everything missing, so that there's at least
    /// some indication about why the emulator might crash.
    fn do_non_lazy_linking(&self, bin: &MachO, _mem: &mut Memory) {
        // TODO: Handle symbols that aren't direct. There seem to be a number of
        // external and even internal references in this section that are stored
        // in a different way.

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

            // TODO: look up symbol in host implementations, write pointer
            eprintln!("Warning: unhandled non-lazy symbol: {:?}", symbol);
        }
    }

    /// Handle an SVC instruction encountered during CPU emulation.
    pub fn handle_svc(&mut self, bin: &MachO, current_instruction: u32, svc: u32) {
        match svc {
            0 => self.do_lazy_link(bin, current_instruction),
            _ => {
                panic!("Unexpected SVC #{} at {:#x}", svc, current_instruction);
            }
        }
    }

    fn do_lazy_link(&mut self, bin: &MachO, current_instruction: u32) {
        let stubs = bin.get_section("__symbol_stub4").unwrap();
        let info = stubs.dyld_indirect_symbol_info.as_ref().unwrap();

        assert!((stubs.addr..(stubs.addr + stubs.size)).contains(&current_instruction));
        let offset = current_instruction - stubs.addr;
        assert!(offset % info.entry_size == 0);
        let idx = (offset / info.entry_size) as usize;

        let symbol = info.indirect_undef_symbols[idx].as_deref().unwrap();

        // TODO: look up symbol in host implementations, write specific SVC
        unimplemented!("Call to {}", symbol);
    }
}
