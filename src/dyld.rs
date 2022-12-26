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

mod function_lists;

use crate::abi::{CallFromGuest, GuestFunction};
use crate::mach_o::MachO;
use crate::mem::{Mem, MutPtr, Ptr};
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
///
/// For convenience, use [export_c_func]:
///
/// ```
/// pub const FUNCTIONS: FunctionExports = &[
///     export_c_func!(NSFoo(_, _)),
///     export_c_func!(NSBar()),
/// ];
/// ```
///
/// See also [crate::objc::ClassExports].
pub type FunctionExports = &'static [(&'static str, HostFunction)];

/// Macro for exporting a function with C-style name mangling. See [FunctionExports].
///
/// ```rust
/// export_c_func!(NSFoo(_, _))
/// ```
///
/// will desugar to:
///
/// ```rust
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

/// Helper for working with [FunctionExports] and similar symbol lists.
pub fn search_lists<T>(
    lists: &'static [&'static [(&'static str, T)]],
    symbol: &str,
) -> Option<&'static T> {
    lists
        .iter()
        .flat_map(|&n| n)
        .find(|&(sym, _)| *sym == symbol)
        .map(|&(_, ref f)| f)
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

pub struct Dyld {
    linked_host_functions: Vec<HostFunction>,
    return_to_host_routine: Option<GuestFunction>,
}

impl Dyld {
    /// We reserve this SVC ID for invoking the lazy linker.
    const SVC_LAZY_LINK: u32 = 0;
    /// We reserve this SVC ID for the special return-to-host routine.
    pub const SVC_RETURN_TO_HOST: u32 = 1;
    /// The range of SVC IDs `SVC_LINKED_FUNCTIONS_BASE..` is used to reference
    /// [Self::linked_host_functions] entries.
    const SVC_LINKED_FUNCTIONS_BASE: u32 = Self::SVC_RETURN_TO_HOST + 1;

    pub fn new() -> Dyld {
        Dyld {
            linked_host_functions: Vec::new(),
            return_to_host_routine: None,
        }
    }

    pub fn return_to_host_routine(&self) -> GuestFunction {
        self.return_to_host_routine.unwrap()
    }

    /// Do linking-related tasks that need doing right after loading a binary.
    pub fn do_initial_linking(&mut self, bin: &MachO, mem: &mut Mem, objc: &mut ObjC) {
        assert!(self.return_to_host_routine.is_none());
        self.return_to_host_routine = {
            let routine = [
                encode_a32_svc(Self::SVC_RETURN_TO_HOST),
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
            Some(ptr)
        };

        objc.register_bin_selectors(bin, mem);
        objc.register_host_selectors(mem);

        self.setup_lazy_linking(bin, mem);
        // Must happen before `register_bin_classes`, else superclass pointers
        // will be wrong.
        self.do_non_lazy_linking(bin, mem, objc);

        objc.register_bin_classes(bin, mem);
        objc.register_bin_categories(bin, mem);
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
        let Some(stubs) = bin.get_section("__symbol_stub4") else {
            return;
        };

        let entry_size = stubs.dyld_indirect_symbol_info.as_ref().unwrap().entry_size;

        assert!(entry_size == 12); // should be three A32 instructions
        assert!(stubs.size % entry_size == 0);
        let stub_count = stubs.size / entry_size;
        for i in 0..stub_count {
            let ptr: MutPtr<u32> = Ptr::from_bits(stubs.addr + i * entry_size);
            mem.write(ptr + 0, encode_a32_svc(Self::SVC_LAZY_LINK));
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
    fn do_non_lazy_linking(&self, bin: &MachO, mem: &mut Mem, objc: &mut ObjC) {
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
        mem: &mut Mem,
        svc_pc: u32,
        svc: u32,
    ) -> HostFunction {
        match svc {
            Self::SVC_LAZY_LINK => self.do_lazy_link(bin, mem, svc_pc),
            Self::SVC_RETURN_TO_HOST => unreachable!(), // don't handle here
            Self::SVC_LINKED_FUNCTIONS_BASE.. => {
                let f = self
                    .linked_host_functions
                    .get((svc - Self::SVC_LINKED_FUNCTIONS_BASE) as usize);
                let Some(&f) = f else {
                    panic!("Unexpected SVC #{} at {:#x}", svc, svc_pc);
                };
                f
            }
        }
    }

    fn do_lazy_link(&mut self, bin: &MachO, mem: &mut Mem, svc_pc: u32) -> HostFunction {
        let stubs = bin.get_section("__symbol_stub4").unwrap();
        let info = stubs.dyld_indirect_symbol_info.as_ref().unwrap();

        assert!((stubs.addr..(stubs.addr + stubs.size)).contains(&svc_pc));
        let offset = svc_pc - stubs.addr;
        assert!(offset % info.entry_size == 0);
        let idx = (offset / info.entry_size) as usize;

        let symbol = info.indirect_undef_symbols[idx].as_deref().unwrap();

        let Some(&f) = search_lists(function_lists::FUNCTION_LISTS, symbol) else {
            panic!("Call to unimplemented function {}", symbol);
        };

        // Allocate an SVC ID for this host function
        let idx: u32 = self.linked_host_functions.len().try_into().unwrap();
        let svc = idx + Self::SVC_LINKED_FUNCTIONS_BASE;
        self.linked_host_functions.push(f);

        // Rewrite stub function to call this host function
        let stub_function_ptr: MutPtr<u32> = Ptr::from_bits(svc_pc);
        mem.write(stub_function_ptr, encode_a32_svc(svc));
        assert!(mem.read(stub_function_ptr + 1) == encode_a32_ret());

        // Return the host function so that we can call it now that we're done.
        f
    }
}
