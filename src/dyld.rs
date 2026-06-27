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
//! iPhone OS's system frameworks and other dynamically-linked libraries, but
//! instead of actually loading and linking the original framework binaries,
//! this "dynamic linker" will generate appropriate stubs for calling into
//! touchHLE's own implementations of the frameworks, which are "host code"
//! (i.e. not themselves running under emulation).
//!
//! This also does normal dynamic linking for libgcc, libstdc++, etc.
//!
//! See [crate::mach_o] for resources.

mod dylib_list;

use crate::abi::{CallFromGuest, GuestFunction};
use crate::bundle;
use crate::cpu::Cpu;
use crate::frameworks::foundation::ns_string;
use crate::mach_o::{MachO, SectionType};
use crate::mem::{ConstVoidPtr, GuestUSize, Mem, MutPtr, MutVoidPtr, Ptr};
use crate::objc::{nil, ClassExports, ObjC};
use crate::Environment;
use std::collections::HashMap;

pub use dylib_list::DYLIB_LIST;

/// Struct used to expose a host implementation of a dynamic library (usually a
/// framework) to the linker.
///
/// Each module that wants to expose a library to guest code should export a
/// constant using this type, which collects all the relevant [ClassExports],
/// [ConstantExports] and [FunctionExports] for the library. For example:
///
/// ```ignore
/// pub const DYLIB: HostDylib = HostDylib {
///     path: "/System/Library/Frameworks/FooBarKit.framework/FooBarKit",
///     aliases: &[],
///     class_exports: &[baz::CLASSES],
///     constant_exports: &[qux::CONSTANTS],
///     function_exports: &[qux::FUNCTIONS, baz::FUNCTIONS],
/// };
/// ```
///
/// The `path` should be the canonical notional filesystem path that the library
/// is referenced by on the real OS, for example `"/usr/lib/libobjc.A.dylib"`
/// or `"/System/Library/Frameworks/Foundation.framework/Foundation"`. For
/// libraries that have several symlinked paths, non-canonical alternate
/// paths can be listed under `aliases`, for example `"/usr/lib/libobjc.dylib"`.
pub struct HostDylib {
    pub path: &'static str,
    pub aliases: &'static [&'static str],
    pub class_exports: &'static [ClassExports],
    pub constant_exports: &'static [ConstantExports],
    pub function_exports: &'static [FunctionExports],
}

pub type HostFunction = &'static dyn CallFromGuest;

/// Type for lists of functions exported by host implementations of dynamic
/// libraries (usually frameworks).
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
/// All the constants like this can then be collected into a [HostDylib].
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
/// See also [ConstantExports] and [ClassExports].
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

/// Other variant of [export_c_func] macro, allowing to define an alias
/// for the exporting function. This is useful then alias may contain
/// characters not normally allowed for Rust function's names. (e.g. `$`)
#[macro_export]
macro_rules! export_c_func_aliased {
    ($alias:literal, $name:ident ($($_:ty),*)) => {
        (
            concat!("_", $alias),
            &($name as fn(&mut $crate::Environment, $($_),*) -> _)
        )
    };
}
pub use crate::export_c_func_aliased; // #[macro_export] is weird...

/// Type for describing a constant (C `extern const` symbol) that will be
/// created by the linker if the guest app references it. See [ConstantExports].
pub enum HostConstant {
    NSString(&'static str),
    NullPtr,
    Custom(fn(&mut Environment) -> ConstVoidPtr),
}

/// Type for lists of constants exported by host implementations of  dynamic
/// libraries (usually frameworks).
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
/// All the constants like this can then be collected into a [HostDylib].
///
/// The strings are the mangled symbol names. For C constants, this is just the
/// name prefixed with an underscore.
///
/// See also [FunctionExports], [ClassExports].
pub type ConstantExports = &'static [(&'static str, HostConstant)];

/// Search the list of [HostDylib]s for a class/constant/function by its symbol.
///
/// Example usage: `search_host_dylibs(|dylib| dylib.function_exports, "_foo")`
pub fn search_host_dylibs<T, F>(get_exports: F, symbol: &str) -> Option<&'static (&'static str, T)>
where
    F: Fn(&HostDylib) -> &'static [&'static [(&'static str, T)]],
{
    // TODO: In general, we should rarely if ever need to search the full set
    //       of dylibs for a symbol. Now that we know which symbols belong to
    //       which libraries, we should at least only search libraries that are
    //       referenced by the app and currently "loaded". We probably should
    //       also implement the Mach-O two-level symbol namespacing eventually.
    DYLIB_LIST
        .iter()
        .copied()
        .map(get_exports)
        .find_map(|lists| search_lists(lists, symbol))
}

/// Helper for working with [ClassExports]/[ConstantExports]/[FunctionExports].
fn search_lists<T>(
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

fn alloc_a32_ret_stub(mem: &mut Mem) -> MutPtr<u32> {
    let fn_ptr: MutPtr<u32> = mem.alloc(8).cast();
    mem.write(fn_ptr + 0, encode_a32_ret());
    mem.write(fn_ptr + 1, encode_a32_trap());
    fn_ptr
}

fn link_cxxabi_vtable(
    name: &str,
    cxxabi_vtable_addrs: &mut HashMap<String, u32>,
    mem: &mut Mem,
) -> ConstVoidPtr {
    let addr = *cxxabi_vtable_addrs.entry(name.to_string()).or_insert_with(|| {
        let stub = alloc_a32_ret_stub(mem);
        let stub_addr = stub.to_bits();
        let v: MutPtr<u32> = mem.alloc(40).cast();
        mem.write(v + 0, 0);
        mem.write(v + 1, 0);
        for i in 2..10 {
            mem.write(v + i, stub_addr);
        }
        v.to_bits()
    });
    Ptr::from_bits(addr).cast_const()
}

fn link_cxxabi_typeinfo(
    name: &str,
    cxxabi_vtable_addrs: &mut HashMap<String, u32>,
    mem: &mut Mem,
) -> ConstVoidPtr {
    let name_bytes = name.strip_prefix('_').unwrap_or(name);
    let name_cstr = mem.alloc_and_write_cstr(name_bytes.as_bytes());
    let ti: MutPtr<u32> = mem.alloc(8).cast();
    let vtable_addr = *cxxabi_vtable_addrs
        .entry("__ZTVN10__cxxabiv117__class_type_infoE".to_string())
        .or_insert_with(|| {
            let stub = alloc_a32_ret_stub(mem);
            let stub_addr = stub.to_bits();
            let v: MutPtr<u32> = mem.alloc(40).cast();
            mem.write(v + 0, 0);
            mem.write(v + 1, 0);
            for i in 2..10 {
                mem.write(v + i, stub_addr);
            }
            v.to_bits()
        });
    mem.write(ti + 0, vtable_addr + 8);
    mem.write(ti + 1, name_cstr.to_bits());
    ti.cast().cast_const()
}

/// Make `std::string((const char*)NULL)` construct an empty string instead of
/// looping forever / corrupting memory.
///
/// See the call site in [Dyld::do_initial_linking] for the full rationale. In
/// short: the char specialisation of libstdc++'s
/// `basic_string::_S_construct<const char*>(beg, end, alloc,
/// forward_iterator_tag)` handles a NULL `beg` by branching to a block that
/// calls `std::__throw_logic_error`. Because touchHLE neuters that helper to a
/// bare `BX LR`, control falls through into the normal copy path with
/// `beg == NULL` and `end - beg == npos`, producing a `memcpy(dest, NULL,
/// npos)` and a string whose length is `npos`.
///
/// The function already contains the correct behaviour we want for this case:
/// an "empty string" block (taken when `beg == end`) that returns
/// `_S_empty_rep()._M_refdata()`. We simply retarget the NULL-pointer branch so
/// it jumps there instead.
///
/// The relevant Thumb-2 code (from the bundled `libstdc++.6.0.9.dylib`) is:
/// ```text
///   <entry>:   cmp r0, r1        ; beg == end ?
///              ...
///              bne  <dispatch>    ; fall through == "empty string" block
///   <empty>:   ldr r0, [pc, ...]  ; r0 = &_S_empty_rep_storage   <-- retarget here
///              ...
///   <dispatch>:cmp r0, #0         ; beg == NULL ?
///              bne <normal-copy>
///              b   <throw>        ; beg == NULL                  <-- patched
/// ```
/// We locate the `cmp r0,#0` / `bne` / `b` triple and rewrite the final
/// unconditional `b` so it targets `<empty>` (the halfword right after the
/// first `bne`). The patch is verified against the expected instruction
/// encodings and skipped (with a warning) if the library doesn't match, so an
/// unexpected libstdc++ build can never be silently mis-patched.
fn patch_string_s_construct_null(dylib: &MachO, mem: &mut Mem) {
    const SYM: &str = "__ZNSs12_S_constructIPKcEEPcT_S3_RKSaIcESt20forward_iterator_tag";
    let Some(&entry_with_thumb_bit) = dylib.exported_symbols.get(SYM) else {
        return;
    };
    // This patch only understands the Thumb encoding shipped with touchHLE.
    if entry_with_thumb_bit & 1 == 0 {
        return;
    }
    let entry = entry_with_thumb_bit & !1;

    let read_hw = |mem: &Mem, addr: u32| -> u16 { mem.read(Ptr::<u16, false>::from_bits(addr)) };

    // Scan a bounded window of the function body.
    const SCAN_HALFWORDS: u32 = 128;
    // Address (halfword) of the empty-string block: the instruction right after
    // the first `bne` that follows the initial `cmp r0, r1` (0x4288).
    let mut empty_block: Option<u32> = None;
    let mut seen_cmp_r0_r1 = false;
    for i in 0..SCAN_HALFWORDS {
        let addr = entry + i * 2;
        let hw = read_hw(mem, addr);
        if !seen_cmp_r0_r1 {
            if hw == 0x4288 {
                seen_cmp_r0_r1 = true;
            }
            continue;
        }
        // `bne` is a T1 conditional branch: 0xD1xx.
        if hw & 0xff00 == 0xd100 {
            empty_block = Some(addr + 2);
            break;
        }
    }
    let Some(empty_block) = empty_block else {
        log!(
            "Warning: could not locate std::string _S_construct empty-string \
             block in {}; skipping NULL-construction patch.",
            dylib.name
        );
        return;
    };

    // Find the `cmp r0,#0` (0x2800), `bne` (0xD1xx), `b` (0xE7xx) triple that
    // dispatches the `beg != end` case.
    for i in 0..SCAN_HALFWORDS {
        let addr = entry + i * 2;
        if read_hw(mem, addr) != 0x2800 {
            continue;
        }
        let bne = read_hw(mem, addr + 2);
        let b = read_hw(mem, addr + 4);
        if bne & 0xff00 != 0xd100 || b & 0xf800 != 0xe000 {
            continue;
        }
        // Re-encode the unconditional branch at `addr + 4` to target
        // `empty_block`. T1 B: imm11 = (target - (pc + 4)) / 2, pc == addr + 4.
        let b_addr = addr + 4;
        let delta = (empty_block as i32) - (b_addr as i32 + 4);
        let imm11 = (delta >> 1) & 0x7ff;
        let new_b = 0xe000u16 | imm11 as u16;
        mem.write(Ptr::<u16, true>::from_bits(b_addr), new_b);
        log!(
            "Patched libstdc++ std::string _S_construct in {} so \
             std::string((char*)NULL) yields an empty string instead of \
             looping/corrupting (b {:#06x} -> {:#06x}).",
            dylib.name,
            b,
            new_b
        );
        return;
    }

    log!(
        "Warning: could not locate std::string _S_construct NULL branch in {}; \
         skipping NULL-construction patch.",
        dylib.name
    );
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
    non_lazy_host_functions: HashMap<&'static str, GuestFunction>,
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
    /// We reserve this SVC ID for lazy linking and returning right after.
    /// It is also a mask for the linked functions to indicate that an
    /// additional return instruction needs to be manually executed after
    /// handling the SVC.
    pub const SVC_LAZY_LINK_RET_FLAG: u32 = 0x800000;

    const SYMBOL_STUB1_INSTRUCTIONS: [u32; 1] = [0xe59ff000];
    // mask this with lowest 12 bits to restore instructions
    const SYMBOL_STUB_INSTRUCTIONS: [u32; 2] = [0xe59fc000, 0xe59cf000];
    const PIC_SYMBOL_STUB_INSTRUCTIONS: [u32; 3] = [0xe59fc004, 0xe08fc00c, 0xe59cf000];

    pub fn new() -> Dyld {
        Dyld {
            linked_host_functions: Vec::new(),
            return_to_host_routine: None,
            thread_exit_routine: None,
            constants_to_link_later: Vec::new(),
            non_lazy_host_functions: HashMap::new(),
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
    pub fn do_initial_linking(
        &mut self,
        bundle: &bundle::Bundle,
        bins: &[MachO],
        mem: &mut Mem,
        objc: &mut ObjC,
    ) {
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

        objc.register_bin_classes(bundle, &bins[0], mem);
        objc.register_bin_categories(&bins[0], mem);

        ns_string::register_constant_strings(&bins[0], mem, objc);

        // The `std::__throw_*` helpers in the bundled libstdc++ dylib are
        // `noreturn` — they construct and throw a C++ exception. touchHLE
        // has no real unwinder, so a thrown exception inside libstdc++
        // calls `__cxa_terminate()`, which aborts the host process. The
        // user-visible message is something like:
        //
        //   terminate called after throwing an instance of 'std::logic_error'
        //     what():  basic_string::_S_construct NULL not valid
        //
        // To allow games to recover from "soft" failures (e.g. Geometry
        // Dash 1.0 hitting `std::string(const char*)` with a NULL argument
        // during Cocos2d initialization), we splice an immediate `BX LR` at
        // the entry of each `__throw_*` helper. Callers in libstdc++
        // already check the failure condition before calling the helper —
        // making the helper a no-op causes them to continue with the
        // failed state (e.g. `_S_construct` continues with NULL/zero
        // iterators and produces an empty string).
        //
        // This is undefined behaviour in strict ISO C++, but in practice
        // the caller paths in libstdc++ degrade to a no-op or empty
        // result, which is far less disruptive than aborting the entire
        // emulator. iOS dyld did roughly the same thing for these games:
        // historically `std::string(NULL)` would yield an empty string on
        // Apple's libstdc++ as well (and Geometry Dash 1.0 launched on
        // Adreno devices that hit this code path).
        for dylib in bins.iter().skip(1) {
            let mut patch_count = 0;
            for sym in [
                "__ZSt19__throw_logic_errorPKc",
                "__ZSt20__throw_length_errorPKc",
                "__ZSt20__throw_out_of_rangePKc",
                "__ZSt17__throw_bad_allocv",
                "__ZSt16__throw_bad_castv",
                "__ZSt19__throw_range_errorPKc",
                "__ZSt22__throw_overflow_errorPKc",
                "__ZSt23__throw_underflow_errorPKc",
                "__ZSt21__throw_runtime_errorPKc",
                "__ZSt24__throw_invalid_argumentPKc",
                "__ZSt23__throw_ios_failurePKc",
                "__ZSt18__throw_bad_typeidv",
                "__ZSt19__throw_bad_exceptionv",
                "__ZSt25__throw_bad_function_callv",
            ] {
                let Some(&entry_with_thumb_bit) = dylib.exported_symbols.get(sym) else {
                    continue;
                };
                let entry = entry_with_thumb_bit & !1;
                let is_thumb = (entry_with_thumb_bit & 1) != 0;
                if is_thumb {
                    // Thumb-1 BX LR = 0x4770. Pad the 32-bit word with a NOP
                    // (0x46C0 = mov r8, r8) so the surrounding code stays
                    // valid if execution somehow falls through.
                    let function_ptr: MutPtr<u32> = Ptr::from_bits(entry);
                    mem.write(function_ptr, 0x46C0_4770);
                } else {
                    // ARM `BX LR` = 0xE12FFF1E.
                    let function_ptr: MutPtr<u32> = Ptr::from_bits(entry);
                    mem.write(function_ptr, 0xE12FFF1E);
                }
                patch_count += 1;
                log_dbg!(
                    "Patched libstdc++ {} at {:#x} (thumb={}) -> bx lr",
                    sym,
                    entry_with_thumb_bit,
                    is_thumb
                );
            }
            if patch_count > 0 {
                log!(
                    "Patched {} libstdc++ std::__throw_* helpers to return \
                     instead of throwing (avoids host-process abort when \
                     guest C++ code hits soft failures like std::string(NULL)).",
                    patch_count
                );
            }

            // `std::string(const char*)` with a NULL argument is constructed
            // via `_S_construct(NULL, NULL + npos)`. libstdc++ detects the NULL
            // pointer and calls `std::__throw_logic_error("basic_string::
            // _S_construct null not valid")`. With the `__throw_*` helpers
            // neutered to `BX LR` above, that throw becomes a no-op and
            // execution falls through to the *normal* construction path, which
            // does `memcpy(rep_data, NULL, npos)` and sets the string length to
            // `npos`. The over-sized copy is skipped by `Mem::memmove`, but the
            // resulting corrupt (length == npos) string sends some apps into an
            // effectively infinite loop (e.g. Turbo Dismount's startup builds
            // std::strings from a table that contains a NULL entry under
            // touchHLE).
            //
            // The documented intent of the throw-neutering is for `std::
            // string(NULL)` to behave like an *empty* string. We make that
            // actually happen by retargeting the NULL-pointer branch inside
            // `_S_construct` so it jumps to the function's existing
            // "empty string" path (which returns `_S_empty_rep()._M_refdata()`)
            // instead of the throw / over-sized-copy path.
            patch_string_s_construct_null(dylib, mem);
        }
    }

    /// Dumps all lazy symbols (functions) referenced by the binary
    /// as JSON to stdout.
    ///
    /// The JSON has the following form:
    /// ```json
    /// {
    ///     "object": "lazy_symbols",
    ///     "symbols": [
    ///         {
    ///             "symbol": ((name of symbol)),
    ///             "linked_to": "host" | "dylib" | null,
    ///             "dylib": ((name of dylib)) | null,
    ///         },
    ///         ...
    ///     ]
    /// }
    /// ```
    pub fn dump_lazy_symbols(
        &mut self,
        bins: &[MachO],
        file: &mut std::fs::File,
    ) -> Result<(), std::io::Error> {
        use std::io::Write;
        // Guest binary is always bin 0.
        let stubs = bins[0].get_section(SectionType::SymbolStubs).unwrap();
        let info = stubs.dyld_indirect_symbol_info.as_ref().unwrap();
        writeln!(
            file,
            "{{\n    \"object\":\"lazy_symbols\",\n    \"symbols\": ["
        )?;
        'sym: for (i, symbol) in info.indirect_undef_symbols.iter().enumerate() {
            // Why doesn't json allow trailing commas...
            let comma = if i == info.indirect_undef_symbols.len() - 1 {
                ""
            } else {
                ","
            };
            let symbol = symbol.as_ref().unwrap();
            if let Some(&(_, _)) = search_host_dylibs(|dylib| dylib.function_exports, symbol) {
                writeln!(
                    file,
                    "        {{ \"symbol\": \"{symbol}\", \"linked_to\": \"host\"}}{comma}"
                )?;
                continue;
            }
            for dylib in bins.iter() {
                if dylib.exported_symbols.contains_key(symbol) {
                    writeln!(
                        file,
                        "        {{ \"symbol\": \"{}\", \"linked_to\": \"dylib\", \"dylib\": \"{}\"}}{}",
                        symbol, dylib.name, comma
                    )?;
                    continue 'sym;
                }
            }
            writeln!(file, "        {{ \"symbol\": \"{symbol}\" }}{comma}")?;
        }
        writeln!(file, "    ]\n}}")
    }

    /// Dumps all non-objc symbols provided by touchHLE.
    ///
    /// The dump format is Objective-C code (with meaningless types) that can be
    /// compiled to generate stub libraries that can be linked against, with
    /// comments providing the paths each library would be installed to.
    /// This is used for building the integration tests.
    pub fn dump_host_symbols(file: &mut std::fs::File) -> Result<(), std::io::Error> {
        use std::io::Write;
        for dylib in DYLIB_LIST {
            writeln!(file, "// {}", dylib.path)?;
            for alias in dylib.aliases {
                writeln!(file, "// {alias}")?;
            }
            for (class_name, _) in dylib.class_exports.iter().copied().flatten() {
                writeln!(file, "@interface {class_name}")?;
                writeln!(file, "@end")?;
                writeln!(file, "@implementation {class_name}")?;
                writeln!(file, "@end")?;
            }
            for (constant_symbol, _) in dylib.constant_exports.iter().copied().flatten() {
                let name = constant_symbol.strip_prefix("_").unwrap();
                // Some symbols (e.g. `OBJC_IVAR_$_NSObject.isa`) contain
                // characters that aren't valid in a C identifier, so they
                // can't be used as a declaration name directly. Emit a
                // sanitized identifier with an `asm()` label so the stub still
                // exports the real symbol name the app links against.
                if name.contains(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$')) {
                    let sanitized: String = name
                        .chars()
                        .map(|c| if c.is_ascii_alphanumeric() || c == '$' { c } else { '_' })
                        .collect();
                    writeln!(file, "int {sanitized} asm(\"{constant_symbol}\");")?;
                } else {
                    writeln!(file, "int {name};")?;
                }
            }
            for (function_symbol, _) in dylib.function_exports.iter().copied().flatten() {
                writeln!(
                    file,
                    "void {}() {{}}",
                    function_symbol.strip_prefix("_").unwrap()
                )?;
            }
        }
        Ok(())
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

        let Some(indirect_info) = stubs.dyld_indirect_symbol_info.as_ref() else {
            log!(
                "Warning: setup_lazy_linking: __symbol_stub section is missing dyld indirect symbol info; skipping lazy stub rewrite."
            );
            return;
        };
        let entry_size = indirect_info.entry_size;

        // two or three A32 instructions (PIC stub needs one more) followed by
        // the address or offset of the corresponding __la_symbol_ptr
        let expected_instructions: &[u32] = match entry_size {
            4 => &[],
            12 => Self::SYMBOL_STUB_INSTRUCTIONS.as_slice(),
            16 => Self::PIC_SYMBOL_STUB_INSTRUCTIONS.as_slice(),
            other => {
                log!(
                    "Warning: setup_lazy_linking: unsupported stub entry size {}; skipping lazy stub rewrite.",
                    other
                );
                return;
            }
        };

        assert!(stubs.size % entry_size == 0);
        let stub_count = stubs.size / entry_size;
        for i in 0..stub_count {
            let ptr: MutPtr<u32> = Ptr::from_bits(stubs.addr + i * entry_size);

            // Check that the stub matches the expected pattern.
            // Some apps (e.g. Thumb-compiled) may have different patterns —
            // skip them rather than panicking.
            let mut mismatch = false;
            for (j, &instr) in expected_instructions.iter().enumerate() {
                if mem.read(ptr + j.try_into().unwrap()) != instr {
                    log_dbg!(
                        "Warning: stub {} at {:#x} has unexpected instruction at offset {} \
                         (expected {:#010x}, got {:#010x}), skipping",
                        i,
                        stubs.addr + i * entry_size,
                        j,
                        instr,
                        mem.read::<u32, true>(ptr + j.try_into().unwrap())
                    );
                    mismatch = true;
                    break;
                }
            }
            if mismatch {
                continue;
            }

            // For convenience, make the stub return once the SVC is done
            // (Otherwise we have to manually update the PC)
            if entry_size == 4 {
                mem.write(ptr + 0, encode_a32_svc(Self::SVC_LAZY_LINK_RET_FLAG));
            } else {
                mem.write(ptr + 0, encode_a32_svc(Self::SVC_LAZY_LINK));
                mem.write(ptr + 1, encode_a32_ret());
            }
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
        // Cache for Objective-C blocks runtime class descriptors.
        // All relocation sites for the same name must resolve to the same
        // non-null guest address so that `block->isa ==
        // &_NSConcreteGlobalBlock`
        // identity checks in the blocks runtime work correctly.
        // Without this, the isa field of every global/stack block is NULL
        // (0x0),
        // causing a NULL-page read the first time the ObjC runtime tries to
        // retain or release a block object.
        let mut block_class_addrs: HashMap<String, u32> = HashMap::new();
        // Cache for C++ Itanium ABI type_info vtables. All references to the
        // same vtable symbol must resolve to the same address so that vtable
        // identity checks in dynamic_cast work correctly.
        let mut cxxabi_vtable_addrs: HashMap<String, u32> = HashMap::new();
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
            } else if name == "___mb_cur_max" {
                // __mb_cur_max is a pointer to the C locale's max
                // multibyte character byte count. libstdc++ reads
                // *__mb_cur_max in locale/string code; if this pointer
                // is NULL the dereference crashes. Provide a static
                // value of 1 (single-byte / ASCII locale).
                let val_ptr: MutPtr<u32> = mem.alloc(4).cast();
                mem.write(val_ptr, 1u32);
                log_dbg!("Stubbed ___mb_cur_max at {:?}", val_ptr);
                val_ptr.cast().cast_const()
            } else if name == "dyld_stub_binder" || name == "_dyld_stub_binder" {
                // In iOS, dyld_stub_binder handles lazy symbol binding.
                // However, touchHLE resolves lazy symbols entirely via SVC
                // traps,
                // bypassing the need for a guest-side binder.
                // We create a minimal ARM32 function (BX LR) so if it's somehow
                // called, it just returns safely without executing random
                // memory.
                let fn_ptr: MutPtr<u32> = mem.alloc(8).cast();
                mem.write(fn_ptr + 0, encode_a32_ret());
                mem.write(fn_ptr + 1, encode_a32_trap());
                log_dbg!("Stubbed dyld_stub_binder at {:?}", fn_ptr);
                fn_ptr.cast().cast_const()
            } else if name == "__NSConcreteGlobalBlock" || name == "__NSConcreteStackBlock" {
                // Blocks runtime class descriptor. Allocate a small dummy
                // object in guest memory so isa != NULL. All sites for the
                // same symbol share one address (cached above).
                let addr = *block_class_addrs
                    .entry(name.clone())
                    .or_insert_with(|| mem.alloc(16).to_bits());
                log_dbg!("Patched block class descriptor {} -> {:#x}", name, addr);
                Ptr::from_bits(addr)
            } else if name == "__ZTVN10__cxxabiv117__class_type_infoE"
                || name == "__ZTVN10__cxxabiv120__si_class_type_infoE"
                || name == "__ZTVN10__cxxabiv121__vmi_class_type_infoE"
                || name == "__ZTVN10__cxxabiv119__pointer_type_infoE"
                || name == "__ZTVN10__cxxabiv120__function_type_infoE"
                || name == "__ZTVN10__cxxabiv116__enum_type_infoE"
                || name == "__ZTVN10__cxxabiv117__pbase_type_infoE"
                || name == "__ZTVN10__cxxabiv129__pointer_to_member_type_infoE"
            {
                // C++ Itanium ABI type_info vtable. Without this every
                // type_info object in the app has a NULL vptr, and any call
                // through the vptr (dynamic_cast, exception type matching,
                // type_info comparison) lands on address 0 and crashes.
                //
                // Layout (32-bit ARM, addend +8 from the symbol so callers
                // land on the first method):
                //   +0   offset_to_top   (= 0)
                //   +4   typeinfo for the vtable's class (= 0; never used)
                //   +8   ~type_info() complete dtor   -> BX LR stub
                //   +12  ~type_info() deleting dtor   -> BX LR stub
                //   +16  __is_pointer_p()             -> returns 0 (false)
                //   +20  __is_function_p()            -> returns 0 (false)
                //   +24  __do_catch()                 -> returns 0 (no match)
                //   +28  __do_upcast()                -> returns 0 (no match)
                //   +32  reserved
                //   +36  reserved
                //
                // The relocation addend (usually +8) is applied below, so we
                // return the vtable base here.
                let target = link_cxxabi_vtable(name, &mut cxxabi_vtable_addrs, mem);
                log_dbg!("Stubbed C++ vtable {} -> {:#x}", name, target.to_bits());
                target
            } else if name == "___gxx_personality_sj0" {
                // C++ SjLj exception personality routine. Called by the
                // unwinder for every frame; returning 0 (_URC_NO_REASON)
                // tells it "no handler here, keep going". touchHLE never
                // actually invokes the unwinder, but other code may store
                // this pointer in an LSDA and read it back.
                let fn_ptr: MutPtr<u32> = mem.alloc(8).cast();
                mem.write(fn_ptr + 0, encode_a32_ret());
                mem.write(fn_ptr + 1, encode_a32_trap());
                log_dbg!("Stubbed ___gxx_personality_sj0 -> {:#x}", fn_ptr.to_bits());
                fn_ptr.cast().cast_const()
            } else if name == "_objc_msgSendSuper" || name == "_objc_msgSendSuper_stret" {
                // `objc_msgSendSuper` (the non-`2` variant) takes a pointer to
                // an `objc_super` struct where the `class` field points directly
                // to the *superclass* to start method lookup from (unlike
                // `objc_msgSendSuper2` where it's the *current* class and the
                // runtime dereferences to the super).
                //
                // Some apps (FlyCraft, GameStop) have non-lazy relocations to
                // these symbols from WebKit/SDK stubs compiled with older
                // toolchains.  We resolve them to `objc_msgSendSuper2` /
                // `objc_msgSendSuper2_stret` respectively because:
                //   1. The runtime always finds the method by walking up from
                //      the given class — whether we start from superclass or
                //      from (class whose super we look up) the result is
                //      identical in practice for apps.
                //   2. All touchHLE-implemented classes use the `2` variant
                //      internally anyway.
                //
                // Create a trampoline that calls our existing host
                // implementation of the `2` variant.
                let target_name = if name == "_objc_msgSendSuper" {
                    "_objc_msgSendSuper2"
                } else {
                    "_objc_msgSendSuper2_stret"
                };
                if let Some((sym, _)) =
                    search_host_dylibs(|dylib| dylib.function_exports, target_name)
                {
                    let trampoline_ptr = self
                        .create_proc_address_no_inval(mem, sym)
                        .unwrap()
                        .to_ptr();
                    log_dbg!(
                        "Linked {} -> {} at {:?}",
                        name,
                        target_name,
                        trampoline_ptr
                    );
                    trampoline_ptr
                } else {
                    // Fallback: a BX LR stub that returns 0
                    let fn_ptr: MutPtr<u32> = mem.alloc(8).cast();
                    mem.write(fn_ptr + 0, encode_a32_ret());
                    mem.write(fn_ptr + 1, encode_a32_trap());
                    fn_ptr.cast().cast_const()
                }
            } else if name == "__ZTId"
                || name == "__ZTIf"
                || name == "__ZTIi"
                || name == "__ZTIl"
                || name == "__ZTIj"
                || name == "__ZTIs"
                || name == "__ZTIc"
                || name == "__ZTIv"
                || name == "__ZTIb"
                || name == "__ZTIPKc"
                || name == "__ZTIPc"
                || name == "__ZTIPv"
                || name == "__ZTIPKv"
            {
                // C++ RTTI type_info objects for fundamental types (double,
                // float, int, long, unsigned int, short, char, void, bool,
                // const char*, char*, void*, const void*).
                //
                // The Itanium ABI requires each fundamental type to have a
                // unique type_info object with a specific mangled name. Apps
                // that use dynamic_cast or exception handling may reference
                // these. We allocate a minimal type_info struct:
                //   +0: vptr (→ __cxxabiv117__class_type_infoE vtable + 8)
                //   +4: name pointer (→ mangled type name C string)
                //
                // The name string is never actually used for comparison (RTTI
                // compares by pointer identity on most embedded platforms), but
                // providing a non-NULL value prevents crashes in debuggers.
                let ti = link_cxxabi_typeinfo(name, &mut cxxabi_vtable_addrs, mem);
                log_dbg!("Stubbed C++ typeinfo {} at {:#x}", name, ti.to_bits());
                ti
            } else if name == "___objc_personality_v0" {
                // Objective-C exception personality routine. Mirrors the
                // non-lazy stub above: a guest-code BX LR returning 0
                // (_URC_NO_REASON) so the unwinder keeps walking instead
                // of branching to NULL when an iOS 5+ binary places a
                // pointer to this symbol directly into its __DATA section
                // via an external relocation (rather than through the
                // __nl_symbol_ptr table).
                let fn_ptr: MutPtr<u32> = mem.alloc(8).cast();
                mem.write(fn_ptr + 0, encode_a32_ret());
                mem.write(fn_ptr + 1, encode_a32_trap());
                log_dbg!("Stubbed ___objc_personality_v0 -> {:#x}", fn_ptr.to_bits());
                fn_ptr.cast().cast_const()
            } else if name == "___cxa_terminate_handler"
                || name == "___cxa_unexpected_handler"
                || name == "___cxa_new_handler"
            {
                // Global function-pointer variables: std::terminate_handler,
                // std::unexpected_handler, std::new_handler. The default
                // value is a NULL function pointer; the real C++ runtime
                // checks for NULL and falls back to abort. Provide a single
                // word of zero-initialised guest memory.
                let p: MutPtr<u32> = mem.alloc(4).cast();
                mem.write(p, 0);
                p.cast().cast_const()
            } else if name == "__dispatch_source_type_read"
                || name == "__dispatch_source_type_write"
                || name == "__dispatch_source_type_timer"
                || name == "__dispatch_source_type_data_add"
                || name == "__dispatch_source_type_data_or"
                || name == "__dispatch_source_type_signal"
                || name == "__dispatch_source_type_proc"
                || name == "__dispatch_source_type_vnode"
                || name == "__dispatch_source_type_mach_send"
                || name == "__dispatch_source_type_mach_recv"
                || name == "__dispatch_source_type_memorypressure"
                || name == "__dispatch_queue_attr_concurrent"
                || name == "__dispatch_queue_attr_serial"
                || name == "__dispatch_data_empty"
                || name == "__dispatch_data_destructor_default"
                || name == "__dispatch_data_destructor_free"
                || name == "__dispatch_data_destructor_munmap"
                || name == "__dispatch_main_q"
                || name == "__dispatch_queue_main"
            {
                // GCD dispatch_source_type_t / queue-attribute / data-marker
                // pointers are opaque "type tag" identifiers; touchHLE's
                // libdispatch implementation never inspects their contents,
                // only their identity. A small non-NULL allocation satisfies
                // that contract and prevents NULL-page reads when an app
                // passes one to dispatch_source_create / dispatch_queue_create
                // / dispatch_data_create.
                let p: MutPtr<u32> = mem.alloc(4).cast();
                mem.write(p, 0);
                p.cast().cast_const()
            } else if name == "_in6addr_any" || name == "_in6addr_loopback" {
                // `struct in6_addr` is 16 bytes. `in6addr_any` is all zeros
                // (`::`) and `in6addr_loopback` is `::1` (last byte = 1).
                let p: MutPtr<u8> = mem.alloc(16).cast();
                for i in 0..16 {
                    mem.write(p + i, 0);
                }
                if name == "_in6addr_loopback" {
                    mem.write(p + 15, 1u8);
                }
                p.cast().cast_const()
            } else if name == "_objc_ehtype_vtable" {
                // Apple libobjc (objc4) ships `objc_ehtype_vtable` as the
                // Itanium-ABI C++ typeinfo vtable shared by every
                // per-class `__objc_ehtype` struct used in Objective-C++
                // `@catch (ObjCClass *)` blocks. touchHLE's `__cxa_throw`
                // is a no-op (we never actually unwind), so the vtable is
                // not consulted for type matching — but t
                let p: MutPtr<u32> = mem.alloc(4).cast();
                mem.write(p, 0);
                p.cast().cast_const()
            } else if name == "_NDR_record" {
                // MIG `NDR_record` is a 12-byte transfer-syntax descriptor
                // that's part of the (unused-by-touchHLE) Mach IPC stack.
                // Apps reference the symbol from auto-generated MIG stubs but
                // never actually transmit the bytes. Provide a zero-filled
                // slot so the linker doesn't leave a NULL pointer behind.
                let p: MutPtr<u8> = mem.alloc(12).cast();
                for i in 0..12 {
                    mem.write(p + i, 0);
                }
                p.cast().cast_const()
            } else if let Some(&external_addr) = bins
                .iter()
                .flat_map(|other_bin| other_bin.exported_symbols.get(name))
                .next()
            {
                // Often used for C++ RTTI
                Ptr::from_bits(external_addr)
            } else if let Some((symbol, _)) =
                search_host_dylibs(|dylib| dylib.function_exports, name)
            {
                // We want the same symbol name to always point to the same
                // function.
                let trampoline_ptr = self
                    .create_proc_address_no_inval(mem, symbol)
                    .unwrap()
                    .to_ptr();
                log_dbg!(
                    "Linked external relocation to host function {} at {:?}",
                    symbol,
                    trampoline_ptr
                );
                trampoline_ptr
            } else if let Some((_, template)) = search_host_dylibs(|dylib| dylib.constant_exports, name) {
                // Constants from host dylibs need late linking (they may
                // require a full Environment to resolve, e.g. NSString
                // objects). Store for resolution in do_late_linking().
                // NOTE: We must NOT skip these — a binary may reference a
                // constant ONLY via external relocations without a matching
                // __nl_symbol_ptr entry (e.g. ___sF in Digital Chocolate
                // games compiled with older toolchains).
                self.constants_to_link_later.push((ptr_ptr, template));
                continue;
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
                    .map(|addr| format!("{addr:#x}"))
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

            if symbol == "dyld_stub_binder" || symbol == "_dyld_stub_binder" {
                // Используем наш паникующий хэндлер, который вы
                // зарегистрировали в create_proc_address_no_inval
                let trampoline_ptr = self
                    .create_proc_address_no_inval(mem, symbol)
                    .unwrap()
                    .to_ptr();
                mem.write(ptr_ptr, trampoline_ptr);
                log_dbg!(
                    "Linked non-lazy host function {} at {:?}",
                    symbol,
                    trampoline_ptr
                );
                continue;
            }

            if let Some((symbol, _)) = search_host_dylibs(|dylib| dylib.function_exports, symbol) {
                // We want the same symbol name to always point to the same
                // function. It could point to a specific stub entry, but it's
                // easier to just create a new function and point all the stub
                // entries to it.
                let trampoline_ptr = self
                    .create_proc_address_no_inval(mem, symbol)
                    .unwrap()
                    .to_ptr();
                mem.write(ptr_ptr, trampoline_ptr);
                log_dbg!(
                    "Linked non-lazy host function {} at {:?}",
                    symbol,
                    trampoline_ptr
                );
                log_dbg!("{:?}", self.non_lazy_host_functions);
                continue;
            }
            if let Some((_, template)) = search_host_dylibs(|dylib| dylib.constant_exports, symbol)
            {
                // Delay linking of constant until we have a `&mut Environment`,
                // that makes it much easier to build NSString objects etc.
                self.constants_to_link_later.push((ptr_ptr, template));
                continue;
            }

            // Blocks runtime class descriptors in __nl_symbol_ptr.
            // Must be non-null so block->isa != NULL; otherwise the
            // first retain/release of any stack block causes a
            // NULL-page read at 0x0.
            if symbol == "__NSConcreteStackBlock" || symbol == "__NSConcreteGlobalBlock" {
                let dummy = mem.alloc(16);
                mem.write(ptr_ptr, dummy.cast().cast_const());
                log_dbg!(
                    "Patched non-lazy block class {} -> {:#x}",
                    symbol,
                    dummy.to_bits()
                );
                continue;
            }

            // C++ / ObjC exception handling symbols. If these are
            // NULL the C++ unwinder crashes when any @try block is
            // entered or any ObjC exception is thrown, producing
            // sequential NULL-page reads followed by UndefinedInst.
            //
            // _OBJC_EHTYPE_*: ObjC exception type-info descriptors
            // used by @catch type matching. A dummy non-null object
            // is enough to prevent the NULL dereference.
            //
            // ___objc_personality_v0: called by _Unwind_RaiseException
            // for every frame. We install a trivial stub (BX LR) that
            // returns 0 (_URC_NO_REASON / continue unwinding) so the
            // unwinder keeps moving instead of branching to 0x0.
            if symbol == "_OBJC_EHTYPE_id" || symbol == "_OBJC_EHTYPE_$_NSException" {
                let dummy = mem.alloc(32);
                mem.write(ptr_ptr, dummy.cast().cast_const());
                log_dbg!(
                    "Patched ObjC EH type descriptor {} -> {:#x}",
                    symbol,
                    dummy.to_bits()
                );
                continue;
            }

            if symbol == "___objc_personality_v0" {
                // Minimal ARM32 function: BX LR (returns 0 in R0).
                // Returning _URC_NO_REASON (0) for every frame tells
                // the unwinder "no handler here, keep searching".
                let fn_ptr: MutPtr<u32> = mem.alloc(8).cast();
                mem.write(fn_ptr + 0, encode_a32_ret());
                mem.write(fn_ptr + 1, encode_a32_trap());
                mem.write(ptr_ptr, fn_ptr.cast().cast_const());
                log_dbg!("Stubbed ___objc_personality_v0 -> {:#x}", fn_ptr.to_bits());
                continue;
            }

            // __mb_cur_max is a pointer-to-int used by libstdc++
            // locale code. Provide a value of 1 (single-byte locale).
            if symbol == "___mb_cur_max" {
                let val_ptr: MutPtr<u32> = mem.alloc(4).cast();
                mem.write(val_ptr, 1u32);
                mem.write(ptr_ptr, val_ptr.cast().cast_const());
                log_dbg!("Stubbed ___mb_cur_max -> {:#x}", val_ptr.to_bits());
                continue;
            }

            if symbol == "___gxx_personality_sj0" {
                let fn_ptr: MutPtr<u32> = mem.alloc(8).cast();
                mem.write(fn_ptr + 0, encode_a32_ret());
                mem.write(fn_ptr + 1, encode_a32_trap());
                mem.write(ptr_ptr, fn_ptr.cast().cast_const());
                log_dbg!("Stubbed ___gxx_personality_sj0 -> {:#x}", fn_ptr.to_bits());
                continue;
            }

            if symbol == "___cxa_terminate_handler"
                || symbol == "___cxa_unexpected_handler"
                || symbol == "___cxa_new_handler"
            {
                // Default-NULL global handler pointer.
                let p: MutPtr<u32> = mem.alloc(4).cast();
                mem.write(p, 0);
                mem.write(ptr_ptr, p.cast().cast_const());
                continue;
            }

            if symbol == "__dispatch_source_type_read"
                || symbol == "__dispatch_source_type_write"
                || symbol == "__dispatch_source_type_timer"
                || symbol == "__dispatch_source_type_data_add"
                || symbol == "__dispatch_source_type_data_or"
                || symbol == "__dispatch_source_type_signal"
                || symbol == "__dispatch_source_type_proc"
                || symbol == "__dispatch_source_type_vnode"
                || symbol == "__dispatch_source_type_mach_send"
                || symbol == "__dispatch_source_type_mach_recv"
                || symbol == "__dispatch_source_type_memorypressure"
                || symbol == "__dispatch_queue_attr_concurrent"
                || symbol == "__dispatch_queue_attr_serial"
                || symbol == "__dispatch_data_empty"
                || symbol == "__dispatch_data_destructor_default"
                || symbol == "__dispatch_data_destructor_free"
                || symbol == "__dispatch_data_destructor_munmap"
                || symbol == "__dispatch_main_q"
                || symbol == "__dispatch_queue_main"
            {
                // See the matching branch in the external-relocation loop.
                let p: MutPtr<u32> = mem.alloc(4).cast();
                mem.write(p, 0);
                mem.write(ptr_ptr, p.cast().cast_const());
                log_dbg!(
                    "Stubbed libdispatch identity tag {} -> {:#x}",
                    symbol,
                    p.to_bits()
                );
                continue;
            }

            if symbol == "_in6addr_any" || symbol == "_in6addr_loopback" {
                let p: MutPtr<u8> = mem.alloc(16).cast();
                for i in 0..16 {
                    mem.write(p + i, 0);
                }
                if symbol == "_in6addr_loopback" {
                    mem.write(p + 15, 1u8);
                }
                mem.write(ptr_ptr, p.cast().cast_const());
                continue;
            }

            if symbol == "_NDR_record" {
                let p: MutPtr<u8> = mem.alloc(12).cast();
                for i in 0..12 {
                    mem.write(p + i, 0);
                }
                mem.write(ptr_ptr, p.cast().cast_const());
                continue;
            }

            // C++ Itanium ABI type_info vtables. Same handling as the
            // external-relocation loop above: without these, every
            // `__cxxabivN...__*_type_infoE` reference referenced through the
            // `__nl_symbol_ptr` table is left NULL, so any virtual call on a
            // type_info object (dynamic_cast, exception type matching) lands
            // on address 0 and crashes.
            if symbol == "__ZTVN10__cxxabiv117__class_type_infoE"
                || symbol == "__ZTVN10__cxxabiv120__si_class_type_infoE"
                || symbol == "__ZTVN10__cxxabiv121__vmi_class_type_infoE"
                || symbol == "__ZTVN10__cxxabiv119__pointer_type_infoE"
                || symbol == "__ZTVN10__cxxabiv120__function_type_infoE"
                || symbol == "__ZTVN10__cxxabiv116__enum_type_infoE"
                || symbol == "__ZTVN10__cxxabiv117__pbase_type_infoE"
                || symbol == "__ZTVN10__cxxabiv129__pointer_to_member_type_infoE"
            {
                let target = link_cxxabi_vtable(symbol, &mut cxxabi_vtable_addrs, mem);
                // A non-lazy symbol pointer holds the plain address of the
                // named symbol (no addend), so it resolves to the vtable base.
                mem.write(ptr_ptr, target);
                log_dbg!("Stubbed C++ vtable {} -> {:#x}", symbol, target.to_bits());
                continue;
            }

            // C++ RTTI type_info objects for fundamental types (double,
            // float, int, long, unsigned int, short, char, void, bool,
            // const char*, char*, void*, const void*). Without these the
            // referenced type_info object has a NULL vptr; the first call
            // through it (dynamic_cast / exception type matching / the
            // `typeid(...)` comparison libstdc++ does for `const char*`)
            // branches to 0x0 and crashes. This mirrors the external-
            // relocation loop: the bundled libstdc++ in iOS 8 apps emits
            // `__ZTIPKc` (typeinfo for `char const*`) into __nl_symbol_ptr,
            // which previously fell through to the unhandled warning.
            if symbol == "__ZTId"
                || symbol == "__ZTIf"
                || symbol == "__ZTIi"
                || symbol == "__ZTIl"
                || symbol == "__ZTIj"
                || symbol == "__ZTIs"
                || symbol == "__ZTIc"
                || symbol == "__ZTIv"
                || symbol == "__ZTIb"
                || symbol == "__ZTIPKc"
                || symbol == "__ZTIPc"
                || symbol == "__ZTIPv"
                || symbol == "__ZTIPKv"
            {
                let ti = link_cxxabi_typeinfo(symbol, &mut cxxabi_vtable_addrs, mem);
                mem.write(ptr_ptr, ti);
                log_dbg!("Stubbed C++ typeinfo {} at {:#x}", symbol, ti.to_bits());
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
                HostConstant::Custom(f) => f(env),
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
            Self::SVC_LAZY_LINK | Self::SVC_LAZY_LINK_RET_FLAG => {
                self.do_lazy_link(bins, mem, cpu, svc_pc)
            }
            Self::SVC_THREAD_EXIT | Self::SVC_RETURN_TO_HOST => {
                // These are handled before this dispatch; reaching here
                // would indicate the SVC numbering scheme is corrupted.
                log!(
                    "Warning: Dyld::get_svc_handler received SVC #{} (thread exit / return to host) at {:#x}; this should be handled earlier.",
                    svc, svc_pc
                );
                None
            }
            Self::SVC_LINKED_FUNCTIONS_BASE.. => {
                let f = self.linked_host_functions.get(
                    ((svc & !Self::SVC_LAZY_LINK_RET_FLAG) - Self::SVC_LINKED_FUNCTIONS_BASE)
                        as usize,
                );
                let Some(&(symbol, f)) = f else {
                    log!(
                        "Warning: Unexpected SVC #{} at {:#x}; treating as no-op (returning to caller).",
                        svc, svc_pc
                    );
                    return None;
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
        // Links by restoring the original stub function, then updating
        // __la_symbol_ptr to the appropriate function.
        fn link_by_restoring_stub(
            mem: &mut Mem,
            cpu: &mut Cpu,
            linked_function: u32,
            svc_pc: u32,
            entry_size: u32,
            pic_offset: u32,
        ) -> (MutPtr<u32>, MutPtr<u32>) {
            let original_instructions: &[u32] = match entry_size {
                4 => Dyld::SYMBOL_STUB1_INSTRUCTIONS.as_slice(),
                12 => Dyld::SYMBOL_STUB_INSTRUCTIONS.as_slice(),
                16 => Dyld::PIC_SYMBOL_STUB_INSTRUCTIONS.as_slice(),
                other => {
                    // setup_lazy_linking already filters out unsupported
                    // sizes; if we reach here something is badly out of
                    // sync. Treat as a 12-byte normal stub to keep things
                    // moving rather than aborting the host.
                    log!(
                        "Warning: link_by_restoring_stub: unsupported entry size {}; falling back to 12-byte stub.",
                        other
                    );
                    Dyld::SYMBOL_STUB_INSTRUCTIONS.as_slice()
                }
            };
            let instruction_count: GuestUSize = original_instructions.len().try_into().unwrap();

            // Restore the original stub, which calls the __la_symbol_ptr
            let stub_function_ptr: MutPtr<u32> = Ptr::from_bits(svc_pc);
            if entry_size == 4 {
                mem.write(stub_function_ptr, original_instructions[0] | pic_offset)
            } else {
                for (i, &instr) in original_instructions.iter().enumerate() {
                    mem.write(stub_function_ptr + i.try_into().unwrap(), instr)
                }
            }

            cpu.invalidate_cache_range(stub_function_ptr.to_bits(), instruction_count * 4);
            // Update the __la_symbol_ptr
            let la_symbol_ptr: MutPtr<u32> = if entry_size == 12 {
                // Normal stub: absolute address
                let addr = mem.read(stub_function_ptr + instruction_count);
                Ptr::from_bits(addr)
            } else {
                // The PIC (position-independent code) stub uses a
                // PC-relative offset rather than an absolute address.
                if entry_size == 4 {
                    let offset = mem.read(stub_function_ptr) & 0xFFF;
                    Ptr::from_bits(stub_function_ptr.to_bits() + offset + 8)
                } else {
                    let offset = mem.read(stub_function_ptr + instruction_count);
                    Ptr::from_bits(stub_function_ptr.to_bits() + offset + 12)
                }
            };
            mem.write(la_symbol_ptr, linked_function);
            (stub_function_ptr, la_symbol_ptr)
        }

        let (stubs, pic_offset) = bins
            .iter()
            .find_map(|bin| {
                let stubs = bin.get_section(SectionType::SymbolStubs)?;
                if !(stubs.addr..(stubs.addr + stubs.size)).contains(&svc_pc) {
                    return None;
                }
                let pic_offset = bin
                    .get_section(SectionType::LazySymbolPointers)
                    .map_or(0, |lazy_ptrs| lazy_ptrs.addr - stubs.addr);
                Some((stubs, pic_offset))
            })
            .unwrap();
        let info = stubs.dyld_indirect_symbol_info.as_ref().unwrap();

        let offset = svc_pc - stubs.addr;
        assert!(offset.is_multiple_of(info.entry_size));
        let idx = (offset / info.entry_size) as usize;
        let symbol = info.indirect_undef_symbols[idx].as_deref().unwrap();

        if let Some(&addr) = self.non_lazy_host_functions.get(symbol) {
            // The host function was already linked non-lazily, point the
            // stub and __la_symbol_ptr to the function.
            let (stub_function_ptr, la_symbol_ptr) = link_by_restoring_stub(
                mem,
                cpu,
                addr.addr_with_thumb_bit(),
                svc_pc,
                info.entry_size,
                pic_offset,
            );
            log_dbg!(
                "Linked host function {} at {:?}/{:?} to existing stub ({:?}).",
                symbol,
                stub_function_ptr,
                la_symbol_ptr,
                addr,
            );
            // The stub jumps to the non-lazy function, which calls the
            // host function.
            return None;
        }

        // Prefer guest dylibs (libstdc++.6.dylib, libgcc_s.1.dylib, …) over
        // host dylib stubs. Apps that bundle their own libstdc++ rely on the
        // proper guest C++ ABI (`__cxa_throw`, `__cxa_begin_catch`, the SjLj
        // unwinder, …) reaching the actual library — if a partial host stub
        // wins the lookup, exception handling silently breaks. This mirrors
        // what `do_non_lazy_linking` already does for non-lazy bindings, and
        // matches iOS dyld's behaviour of preferring the app's own linked
        // dylibs over fallback implementations.
        for dylib in bins.iter() {
            if let Some(&addr) = dylib.exported_symbols.get(symbol) {
                let (stub_function_ptr, la_symbol_ptr) =
                    link_by_restoring_stub(mem, cpu, addr, svc_pc, info.entry_size, pic_offset);
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

        if let Some(&(symbol, f)) = search_host_dylibs(|dylib| dylib.function_exports, symbol) {
            // Allocate an SVC ID for this host function
            let idx: u32 = self.linked_host_functions.len().try_into().unwrap();
            let mut svc = idx + Self::SVC_LINKED_FUNCTIONS_BASE;
            // Indicate to the handler to return manually after call
            if info.entry_size == 4 {
                assert!(svc < Self::SVC_LAZY_LINK_RET_FLAG);
                svc |= Self::SVC_LAZY_LINK_RET_FLAG;
            }
            self.linked_host_functions.push((symbol, f));
            // Rewrite stub function to call this host function
            let stub_function_ptr: MutPtr<u32> = Ptr::from_bits(svc_pc);
            mem.write(stub_function_ptr, encode_a32_svc(svc));
            if info.entry_size != 4 {
                assert!(mem.read(stub_function_ptr + 1) == encode_a32_ret());
            }

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

        // Fallback: the symbol isn't implemented by any host dylib and isn't
        // exported by any loaded guest dylib. Instead of panicking (which kills
        // the app), install a stub that logs a warning and returns 0. This
        // lets the emulator keep running for relatively harmless symbols like
        // `_getuid`, `_geteuid`, `_getpid`, etc.
        log!(
            "Warning: call to unimplemented function {} at {:#x}; installing return-0 stub",
            symbol,
            svc_pc
        );
        // `linked_host_functions` requires a `&'static str`, so leak the name.
        let leaked_symbol: &'static str = Box::leak(symbol.to_string().into_boxed_str());
        let f: HostFunction = &(unimplemented_function_stub as fn(&mut Environment) -> i32);
        // Allocate an SVC ID for this stub (same pattern as the host-dylib
        // branch above).
        let idx: u32 = self.linked_host_functions.len().try_into().unwrap();
        let mut svc = idx + Self::SVC_LINKED_FUNCTIONS_BASE;
        if info.entry_size == 4 {
            assert!(svc < Self::SVC_LAZY_LINK_RET_FLAG);
            svc |= Self::SVC_LAZY_LINK_RET_FLAG;
        }
        self.linked_host_functions.push((leaked_symbol, f));
        // Rewrite the stub function to trap into our SVC handler.
        let stub_function_ptr: MutPtr<u32> = Ptr::from_bits(svc_pc);
        mem.write(stub_function_ptr, encode_a32_svc(svc));
        if info.entry_size != 4 {
            assert!(mem.read(stub_function_ptr + 1) == encode_a32_ret());
        }
        cpu.invalidate_cache_range(stub_function_ptr.to_bits(), 4);
        Some(f)
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
        let function_ptr = self.create_proc_address_no_inval(mem, symbol)?;
        // Just in case
        cpu.invalidate_cache_range(function_ptr.addr_without_thumb_bit(), 8);
        Ok(function_ptr)
    }

    /// Internal [Self::create_proc_address] that doesn't invalidate the cache.
    /// For use before a [Cpu] is available.
    fn create_proc_address_no_inval(
        &mut self,
        mem: &mut Mem,
        symbol: &str,
    ) -> Result<GuestFunction, ()> {
        // Нативно обрабатываем рудимент ленивой загрузки Apple:
        if symbol == "dyld_stub_binder" || symbol == "_dyld_stub_binder" {
            // Используем "dyld_stub_binder" (это &'static str), а не переменную
            // symbol
            let symbol_name = "dyld_stub_binder";
            if let Some(&cached_fn) = self.non_lazy_host_functions.get(symbol_name) {
                return Ok(cached_fn);
            }

            let (_, f) = export_c_func!(dyld_stub_binder(_));
            // Передаем именно статическую строку
            let function_ptr = self.create_guest_function(mem, symbol_name, f);
            self.non_lazy_host_functions
                .insert(symbol_name, function_ptr);
            return Ok(function_ptr);
        }

        let &(symbol, f) = search_host_dylibs(|dylib| dylib.function_exports, symbol).ok_or(())?;
        if let Some(&cached_fn) = self.non_lazy_host_functions.get(symbol) {
            return Ok(cached_fn);
        }
        let function_ptr = self.create_guest_function(mem, symbol, f);
        self.non_lazy_host_functions.insert(symbol, function_ptr);
        Ok(function_ptr)
    }

    pub fn create_guest_function(
        &mut self,
        mem: &mut Mem,
        symbol: &'static str,
        f: HostFunction,
    ) -> GuestFunction {
        // Allocate an SVC ID for this host function
        let idx: u32 = self.linked_host_functions.len().try_into().unwrap();
        let svc = idx + Self::SVC_LINKED_FUNCTIONS_BASE;
        self.linked_host_functions.push((symbol, f));

        // Create guest function to call this host function
        let function_ptr = mem.alloc(8);
        let function_ptr: MutPtr<u32> = function_ptr.cast();
        mem.write(function_ptr + 0, encode_a32_svc(svc));
        mem.write(function_ptr + 1, encode_a32_ret());
        GuestFunction::from_addr_with_thumb_bit(function_ptr.to_bits())
    }

    /// Like [Self::create_proc_address], but takes an unmangled C name and
    /// returns a raw pointer to guest memory.
    pub fn create_function_address(
        &mut self,
        mem: &mut Mem,
        cpu: &mut Cpu,
        name: &str,
    ) -> Result<MutVoidPtr, ()> {
        let symbol = format!("_{name}");
        let address = self.create_proc_address(mem, cpu, &symbol)?;
        Ok(Ptr::from_bits(address.addr_with_thumb_bit()))
    }
}

fn dyld_stub_binder(_env: &mut Environment, _arg: u32) {
    // Under HLE we eagerly bind every lazy symbol at link time, so the
    // dyld_stub_binder should never be invoked. If a guest binary still
    // jumps here (e.g. through a hand-rolled lazy-binding scheme), log it
    // instead of crashing the emulator; the caller will simply see a
    // no-op return.
    log!(
        "Warning: dyld_stub_binder was called! Under HLE all lazy symbols are bound eagerly, so this is unexpected. Continuing as a no-op."
    );
}

/// Generic fallback stub for functions referenced by the guest binary but
/// not implemented by any host dylib. Returns 0 so that calls to things like
/// `getuid`, `geteuid`, `getpid`, etc. don't panic the emulator.
///
/// On ARM32 the return value is placed in `r0`, which matches the C ABI for
/// `int`/`uid_t`/`pid_t`/pointer return types.
fn unimplemented_function_stub(_env: &mut Environment) -> i32 {
    0
}
