/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Reading of Mach-O files, the executable and library format on iPhone OS.
//!
//! Implemented using the mach_object crate. All usage of that crate should be
//! confined to this module. The goal is to read the Mach-O binary exactly once,
//! storing any information we'll need later.
//!
//! Useful resources:
//! - Apple's [Overview of the Mach-O Executable Format](https://developer.apple.com/library/archive/documentation/Performance/Conceptual/CodeFootprint/Articles/MachOOverview.html) explains what "segments" and "sections" are, and provides short descriptions of the purposes of some common sections.
//! - Apple's old "OS X ABI Mach-O File Format Reference", which is mirrored in [various](https://github.com/aidansteele/osx-abi-macho-file-format-reference) [places](https://www.symbolcrash.com/wp-content/uploads/2019/02/ABI_MachOFormat.pdf) online.
//! - Alex Drummond's [Inside a Hello World executable on OS X](https://adrummond.net/posts/macho) is about macOS circa 2017 rather than iPhone OS circa 2008, so not all of what it says applies, but the sections up to and including "9. The indirect symbol table" are helpful.
//! - The LLVM functions [`RuntimeDyldMachO::populateIndirectSymbolPointersSection`](https://github.com/llvm/llvm-project/blob/2e999b7dd1934a44d38c3a753460f1e5a217e9a5/llvm/lib/ExecutionEngine/RuntimeDyld/RuntimeDyldMachO.cpp#L179-L220) and [`MachOObjectFile::getIndirectSymbolTableEntry`](https://github.com/llvm/llvm-project/blob/3c09ed006ab35dd8faac03311b14f0857b01949c/llvm/lib/Object/MachOObjectFile.cpp#L4803-L4808) are references for how to read the indirect symbol table.
//! - `/usr/include/mach-o/reloc.h` in the macOS SDK was the reference for the
//!   format of relocation entries.
//! - The [source code of the mach_object crate](https://docs.rs/mach_object/latest/src/mach_object/commands.rs.html) has useful comments that don't show up in the generated documentation, e.g. around `DySymTab`.

use crate::abi::GuestFunction;
use crate::fs::{Fs, GuestPath};
use crate::mem::{Mem, Ptr};
use mach_object::{
    cpu_subtype_t, vm_prot_t, DyLib, LoadCommand, MachCommand, OFile, Symbol, SymbolIter,
    ThreadState, N_ARM_THUMB_DEF, S_LAZY_SYMBOL_POINTERS, S_MOD_INIT_FUNC_POINTERS,
    S_NON_LAZY_SYMBOL_POINTERS, S_SYMBOL_STUBS,
};
use std::collections::HashMap;
use std::io::{Cursor, Seek, SeekFrom};

const VM_PROT_READ: vm_prot_t = 1;
const VM_PROT_WRITE: vm_prot_t = 2;
#[allow(dead_code)]
const VM_PROT_EXECUTE: vm_prot_t = 4;

#[derive(Debug)]
pub struct MachO {
    /// Name (for debugging purposes)
    pub name: String,
    /// Paths of dynamic libraries referenced by the binary.
    pub dynamic_libraries: Vec<String>,
    /// Metadata related to sections.
    pub sections: Vec<Section>,
    /// Symbols exported by the binary. This is a hashmap so the dynamic linker
    /// can look things up quickly. Thumb function symbols always have the Thumb
    /// bit set.
    pub exported_symbols: HashMap<String, u32>,
    /// List of addresses and names of external relocations for the dynamic
    /// linker to resolve.
    pub external_relocations: Vec<(u32, String)>,
    /// Address/program counter value for the entry point.
    pub entry_point_pc: Option<u32>,
}

#[derive(Debug)]
pub struct Section {
    /// Section name.
    pub name: String,
    /// Section address in memory.
    pub addr: u32,
    /// Section size in bytes.
    pub size: u32,
    /// What type of section is this?
    pub type_: SectionType,
    /// Information specific to special dynamic linker sections, if this is one.
    pub dyld_indirect_symbol_info: Option<DyldIndirectSymbolInfo>,
}

/// Various kinds of sections. We're only interested in the ones used by the
/// dynamic linker for now (see also [DyldIndirectSymbolInfo]).
#[derive(Debug, PartialEq, Eq)]
pub enum SectionType {
    /// Normal section, or some section type we don't care about.
    Normal,
    /// Symbol stub section, usually called `__symbol_stub4` or
    /// `__picsymbolstub4`.
    SymbolStubs,
    /// Lazy symbol pointer section, usually called `__la_symbol_ptr`.
    LazySymbolPointers,
    /// Non-lazy symbol pointer section, usually called `__nl_symbol_ptr`.
    NonLazySymbolPointers,
    /// Initialization function pointer section, usually called
    /// `__mod_init_func`.
    ModInitFuncPointers,
}

/// Information relevant to certain special sections which contain a series of
/// pointers or stub functions for indirectly referencing dynamically-linked
/// symbols (see also [SectionType]).
#[derive(Debug)]
pub struct DyldIndirectSymbolInfo {
    /// The size in bytes of an entry (pointer or stub function) in the section.
    pub entry_size: u32,
    /// A list of symbol names corresponding to the entries.
    pub indirect_undef_symbols: Vec<Option<String>>,
}

/// Helper trait that makes [MachO::get_section] work. Yes, this is overkill. :)
pub trait SectionPredicate {
    fn test(&self, section: &Section) -> bool;
}
impl SectionPredicate for &str {
    fn test(&self, section: &Section) -> bool {
        section.name == *self
    }
}
impl SectionPredicate for SectionType {
    fn test(&self, section: &Section) -> bool {
        section.type_ == *self
    }
}

fn get_sym_by_idx<'a>(
    idx: u32,
    (symoff, nsyms, stroff, strsize): (u32, u32, u32, u32),
    is_bigend: bool,
    is_64bit: bool,
    cursor: &'a mut Cursor<&'a [u8]>,
) -> Option<mach_object::Symbol<'a>> {
    if idx >= nsyms {
        return None;
    }

    let symoff = (symoff + idx * 12) as u64;

    cursor.seek(SeekFrom::Start(symoff)).unwrap();

    // This is not how you're supposed to use SymbolIter but the parse_symbol()
    // method on it requires the bytestring crate, so...
    let mut iter = SymbolIter::new(cursor, Vec::new(), 1, stroff, strsize, is_bigend, is_64bit);
    iter.next()
}

/// Parsed relocation entry
#[derive(Debug)]
enum Reloc {
    External {
        addr: u32,
        sym_idx: u32,
        is_pc_relative: bool,
        size: u32,
        type_: u32,
    },
    #[allow(dead_code)]
    Local {
        addr: u32,
        section_idx: u32,
        is_pc_relative: bool,
        size: u32,
        type_: u32,
    },
    #[allow(dead_code)]
    Scattered {
        offset: u32,
        value: u32,
        is_pc_relative: bool,
        size: u32,
        type_: u32,
    },
}
impl Reloc {
    fn parse(is_bigend: bool, entry: [u8; 8]) -> Self {
        assert!(!is_bigend);

        let word1 = u32::from_le_bytes(entry[..4].try_into().unwrap());
        let word2 = u32::from_le_bytes(entry[4..8].try_into().unwrap());

        if (word1 & 0x80000000) != 0 {
            let bitfield = word1;
            let value = word2;

            let offset = bitfield & 0xffffff;
            let type_ = (bitfield >> 24) & 0xf;
            let size = 1 << ((bitfield >> 28) & 3); // log2-encoded pointer size
            let is_pc_relative = ((bitfield >> 30) & 1) == 1;

            Reloc::Scattered {
                offset,
                value,
                is_pc_relative,
                size,
                type_,
            }
        } else {
            let addr = word1;
            let bitfield = word2;

            let sym_or_section_idx = bitfield & 0xffffff;
            let is_pc_relative = ((bitfield >> 24) & 1) == 1;
            let size = 1 << ((bitfield >> 25) & 3); // log2-encoded pointer size
            let is_external = (bitfield >> 27) & 1;
            let type_ = (bitfield >> 28) & 0xf;

            if is_external == 1 {
                Reloc::External {
                    addr,
                    sym_idx: sym_or_section_idx,
                    is_pc_relative,
                    size,
                    type_,
                }
            } else {
                Reloc::Local {
                    addr,
                    section_idx: sym_or_section_idx,
                    is_pc_relative,
                    size,
                    type_,
                }
            }
        }
    }
}

fn cpu_subtype_to_str(ty: cpu_subtype_t) -> &'static str {
    match ty {
        mach_object::CPU_SUBTYPE_ARM_ALL => "armv???",
        mach_object::CPU_SUBTYPE_ARM_V4T => "armv4t",
        mach_object::CPU_SUBTYPE_ARM_V5TEJ => "armv5tej",
        mach_object::CPU_SUBTYPE_ARM_V6 => "armv6",
        mach_object::CPU_SUBTYPE_ARM_XSCALE => "armxscale",
        mach_object::CPU_SUBTYPE_ARM_V7 => "armv7",
        mach_object::CPU_SUBTYPE_ARM_V7F => "armv7f",
        mach_object::CPU_SUBTYPE_ARM_V7S => "armv7s",
        mach_object::CPU_SUBTYPE_ARM_V7K => "armv7k",
        mach_object::CPU_SUBTYPE_ARM_V8 => "armv8",
        _ => panic!("Unexpected cpu subtype: {:?}", ty),
    }
}

impl MachO {
    /// Load the all the sections from a Mach-O binary (provided as `bytes`)
    /// into the guest memory (`into_mem`), and return a struct containing
    /// metadata (e.g. symbols).
    pub fn load_from_bytes(
        bytes: &[u8],
        into_mem: &mut Mem,
        name: String,
    ) -> Result<MachO, &'static str> {
        log_dbg!("Reading {:?}", name);

        let mut cursor = Cursor::new(bytes);

        let file = OFile::parse(&mut cursor).map_err(|_| "Could not parse Mach-O file")?;

        let (header, commands) = match file {
            OFile::MachFile { header, commands } => (header, commands),
            OFile::FatFile { files, .. } => {
                let mut best_subslice = None;
                let mut best_type = None;
                for (arch, _) in files {
                    if arch.cputype != mach_object::CPU_TYPE_ARM {
                        continue;
                    }
                    if arch.cpusubtype == mach_object::CPU_SUBTYPE_ARM_V7
                        || (arch.cpusubtype == mach_object::CPU_SUBTYPE_ARM_V6
                            && best_type != Some(mach_object::CPU_SUBTYPE_ARM_V7))
                        || best_type.is_none()
                    {
                        best_subslice = Some(
                            &bytes[arch.offset as usize..arch.offset as usize + arch.size as usize],
                        );
                        best_type = Some(arch.cpusubtype);
                    }
                }
                return if let Some(subslice) = best_subslice {
                    MachO::load_from_bytes(subslice, into_mem, name)
                } else {
                    Err("No supported architecture in the fat binary")
                };
            }
            OFile::ArFile { .. } | OFile::SymDef { .. } => {
                return Err("Unexpected Mach-O file kind: not an executable");
            }
        };

        if header.cputype != mach_object::CPU_TYPE_ARM {
            return Err("Executable is not for an ARM CPU!");
        }
        log!(
            "Loading {} slice for {:?}",
            cpu_subtype_to_str(header.cpusubtype),
            name
        );

        let is_bigend = header.is_bigend();
        if is_bigend {
            return Err("Executable is not little-endian!");
        }
        let is_64bit = header.is_64bit();
        if is_64bit {
            return Err("Executable is not 32-bit!");
        }
        // TODO: Check cpusubtype (should be some flavour of ARMv6/ARMv7)

        let split_segs = (header.flags & mach_object::MH_SPLIT_SEGS) != 0;

        // Info used while parsing file
        let mut first_segment_base: Option<u32> = None;
        let mut first_read_write_segment_base: Option<u32> = None;
        let mut text_segment_base: Option<u32> = None;
        let mut all_sections = Vec::new();
        let mut sym_tab_info: Option<(u32, u32, u32, u32)> = None;

        // Info used for the result
        let mut dynamic_libraries = Vec::new();
        let mut exported_symbols = HashMap::new();
        let mut indirect_undef_symbols: Vec<Option<String>> = Vec::new();
        let mut external_relocations: Vec<(u32, String)> = Vec::new();
        let mut entry_point_pc: Option<u32> = None;

        for MachCommand(command, _size) in commands {
            match command {
                LoadCommand::Segment {
                    segname,
                    vmaddr,
                    vmsize,
                    fileoff,
                    filesize,
                    initprot,
                    sections,
                    ..
                } => {
                    let vmaddr: u32 = vmaddr.try_into().unwrap();
                    let vmsize: u32 = vmsize.try_into().unwrap();
                    let filesize: u32 = filesize.try_into().unwrap();

                    if first_segment_base.is_none() {
                        first_segment_base = Some(vmaddr);
                    }
                    if first_read_write_segment_base.is_none()
                        && (initprot & VM_PROT_READ) != 0
                        && (initprot & VM_PROT_WRITE) != 0
                    {
                        first_read_write_segment_base = Some(vmaddr);
                    }

                    let load_me = match &*segname {
                        // Special linker data section, not meant to be loaded.
                        "__LINKEDIT" => false,
                        // Zero page needs to be handled seperately.
                        "__PAGEZERO" => {
                            assert!(vmaddr == 0);
                            assert!(filesize == 0);
                            into_mem.set_null_segment_size(vmsize);
                            false
                        }
                        "__TEXT" => {
                            assert!(text_segment_base.is_none());
                            text_segment_base = Some(vmaddr);
                            true
                        }
                        "__DATA" => true,
                        _ => {
                            log!("Warning: Unexpected segment name: {}", segname);
                            true
                        }
                    };

                    if load_me {
                        into_mem.reserve(vmaddr, vmsize);

                        // If filesize is less than vmsize, the rest of the
                        // segment should be filled with zeroes. We are assuming
                        // the memory is already zeroed!
                        if filesize > 0 {
                            assert!(filesize <= vmsize);

                            let src = &bytes[fileoff..][..filesize as usize];
                            let dst = into_mem.bytes_at_mut(Ptr::from_bits(vmaddr), filesize);
                            dst.copy_from_slice(src);
                        }
                    }

                    all_sections.extend_from_slice(&sections);
                }
                LoadCommand::SymTab {
                    symoff,
                    nsyms,
                    stroff,
                    strsize,
                } => {
                    sym_tab_info = Some((symoff, nsyms, stroff, strsize));
                    if cursor.seek(SeekFrom::Start(symoff.into())).is_ok() {
                        let mut cursor = cursor.clone();
                        let symbols = SymbolIter::new(
                            &mut cursor,
                            all_sections.clone(),
                            nsyms,
                            stroff,
                            strsize,
                            is_bigend,
                            is_64bit,
                        );
                        for symbol in symbols {
                            if let Symbol::Debug { .. } = symbol {
                                continue;
                            }
                            if let Symbol::Defined {
                                name: Some(name),
                                external: true,
                                entry,
                                desc,
                                ..
                            } = symbol
                            {
                                let entry: u32 = entry.try_into().unwrap();
                                let entry = if desc & N_ARM_THUMB_DEF != 0 {
                                    entry | GuestFunction::THUMB_BIT
                                } else {
                                    entry
                                };
                                exported_symbols.insert(name.to_string(), entry);
                            };
                        }
                    }
                }
                LoadCommand::DySymTab {
                    indirectsymoff,
                    nindirectsyms,
                    extreloff,
                    nextrel,
                    ..
                } => {
                    let indirectsyms =
                        &bytes[indirectsymoff as usize..][..nindirectsyms as usize * 4];
                    for idx in indirectsyms.chunks(4) {
                        assert!(!is_bigend);
                        let idx = u32::from_le_bytes(idx.try_into().unwrap());

                        let mut cursor = cursor.clone();
                        let sym = get_sym_by_idx(
                            idx,
                            sym_tab_info.unwrap(),
                            is_bigend,
                            is_64bit,
                            &mut cursor,
                        );
                        indirect_undef_symbols.push(match sym {
                            // apparently used in apps?
                            Some(Symbol::Undefined { name: Some(n), .. }) => Some(String::from(n)),
                            // apparently used in libraries?
                            Some(Symbol::Prebound { name: Some(n), .. }) => Some(String::from(n)),
                            // apparently used within libstdc++ for linking to
                            // itself, e.g. to "__Znwm". might be a PIC thing
                            Some(Symbol::Defined { name: Some(n), .. }) => Some(String::from(n)),
                            None => None,
                            _ => panic!("Unexpected symbol kind {:?}", sym),
                        })
                    }

                    let extrels = &bytes[extreloff as usize..][..nextrel as usize * 8];
                    for entry in extrels.chunks(8) {
                        let reloc = Reloc::parse(is_bigend, entry.try_into().unwrap());
                        let Reloc::External {
                            addr,
                            sym_idx,
                            is_pc_relative: false,
                            size: 4,
                            type_: 0, // generic
                        } = reloc
                        else {
                            panic!("Unhandled extrel: {:?}", reloc)
                        };
                        let addr = if split_segs {
                            addr + first_read_write_segment_base.unwrap()
                        } else {
                            addr + first_segment_base.unwrap()
                        };

                        let mut cursor = cursor.clone();
                        let sym = get_sym_by_idx(
                            sym_idx,
                            sym_tab_info.unwrap(),
                            is_bigend,
                            is_64bit,
                            &mut cursor,
                        );
                        match sym {
                            Some(Symbol::Undefined { name: Some(n), .. }) => {
                                external_relocations.push((addr, String::from(n)));
                            }
                            Some(Symbol::Defined { entry, desc, .. }) => {
                                // Apparently these are used for internal
                                // (intra-binary) relocations, despite being
                                // in the external section?
                                //
                                // Resolve them immediately, there is no value
                                // in passing these on to Dyld.
                                let addr = Ptr::from_bits(addr);
                                let entry = entry as u32;
                                let entry = if desc & N_ARM_THUMB_DEF != 0 {
                                    entry | GuestFunction::THUMB_BIT
                                } else {
                                    entry
                                };
                                into_mem.write(addr, entry);
                            }
                            Some(Symbol::Prebound { name: Some(n), .. }) => {
                                let ptr_ptr = Ptr::<u32, true>::from_bits(addr);
                                into_mem.write(ptr_ptr, 0); // Clear prebinding.
                                external_relocations.push((addr, String::from(n)));
                            }
                            _ => panic!("Unexpected symbol kind {:?}", sym),
                        };
                    }
                }
                LoadCommand::EncryptionInfo { id, .. } => {
                    if id != 0 {
                        return Err(
                            "The executable is encrypted. touchHLE can't run encrypted apps!",
                        );
                    }
                }
                LoadCommand::LoadDyLib(DyLib { name, .. }) => {
                    dynamic_libraries.push(String::from(&*name));
                }
                // Old-style entry point PC command
                LoadCommand::UnixThread { state, .. } => {
                    let ThreadState::Arm {
                        __r: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                        __sp: 0,
                        __lr: 0,
                        __pc: pc,
                        __cpsr: 0,
                    } = state
                    else {
                        panic!("Unexpected initial thread state in {:?}: {:?}", name, state);
                    };
                    // There should only be a single initial thread state.
                    assert!(entry_point_pc.is_none());
                    entry_point_pc = Some(pc);
                }
                // New-style entry point PC command
                LoadCommand::EntryPoint {
                    entryoff,
                    stacksize,
                } => {
                    if stacksize != 0 {
                        log!("TODO: stack size of {:#x} bytes requested", stacksize);
                    }
                    // There should only be a single entry point.
                    // (Presumably an executable won't use both commands?)
                    assert!(entry_point_pc.is_none());
                    let entryoff: u32 = entryoff.try_into().unwrap();
                    entry_point_pc = Some(text_segment_base.unwrap() + entryoff);
                }
                // LoadCommand::DyldInfo is apparently a newer thing that 2008
                // games don't have. Ignore for now? Unsure if/when iOS got it.
                LoadCommand::DyldInfo { .. } => {
                    log!("Warning! DyldInfo is not handled.");
                }
                _ => (),
            }
        }

        let sections = all_sections
            .iter()
            .map(|section| {
                let section = &**section;

                let name = section.sectname.clone();
                let addr: u32 = section.addr.try_into().unwrap();
                let size: u32 = section.size.try_into().unwrap();
                let type_ = section.flags.sect_type();

                log_dbg!(
                    "Section: {:?} {:#x} ({:#x} bytes), type {}",
                    name,
                    addr,
                    size,
                    type_,
                );

                use SectionType as ST;
                let (type_, dyld_entry_size) = match type_ {
                    S_MOD_INIT_FUNC_POINTERS => (ST::ModInitFuncPointers, None),
                    // Symbol stub sections have a variable size depending on
                    // whether PIC is in use.
                    S_SYMBOL_STUBS => (ST::SymbolStubs, Some(section.reserved2)),
                    // 4 bytes per pointer
                    S_LAZY_SYMBOL_POINTERS => (ST::LazySymbolPointers, Some(4)),
                    S_NON_LAZY_SYMBOL_POINTERS => (ST::NonLazySymbolPointers, Some(4)),
                    _ => (ST::Normal, None),
                };
                let dyld_indirect_symbol_info = dyld_entry_size.map(|entry_size| {
                    let indirect_start = section.reserved1 as usize;
                    assert!(size % entry_size == 0);
                    let indirect_count = (size / entry_size) as usize;
                    let indirects = &mut indirect_undef_symbols[indirect_start..][..indirect_count];
                    let syms = indirects.iter_mut().map(|sym| sym.take()).collect();
                    DyldIndirectSymbolInfo {
                        entry_size,
                        indirect_undef_symbols: syms,
                    }
                });

                Section {
                    name,
                    addr,
                    size,
                    type_,
                    dyld_indirect_symbol_info,
                }
            })
            .collect();

        Ok(MachO {
            name,
            dynamic_libraries,
            sections,
            exported_symbols,
            external_relocations,
            entry_point_pc,
        })
    }

    /// Load the all the sections from a Mach-O binary (from `path`) into the
    /// guest memory (`into_mem`), and return a struct containing metadata
    /// (e.g. symbols).
    pub fn load_from_file<P: AsRef<GuestPath>>(
        path: P,
        fs: &Fs,
        into_mem: &mut Mem,
    ) -> Result<MachO, &'static str> {
        let name = path.as_ref().file_name().unwrap().to_string();
        Self::load_from_bytes(
            &fs.read(path.as_ref())
                .map_err(|_| "Could not read executable file")?,
            into_mem,
            name,
        )
    }

    /// Get a section by its name (`&str`) or type ([SectionType]).
    pub fn get_section<P: SectionPredicate>(&self, by: P) -> Option<&Section> {
        self.sections.iter().find(|section| by.test(section))
    }
}
