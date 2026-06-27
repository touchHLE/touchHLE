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
use crate::mem::{GuestUSize, Mem, Ptr};
use mach_object::{
    cpu_subtype_t, vm_prot_t, Bind, BindSymbolType, DyLib, LoadCommand, MachCommand, OFile, Rebase,
    Symbol, SymbolIter, ThreadState, N_ARM_THUMB_DEF, S_LAZY_SYMBOL_POINTERS,
    S_MOD_INIT_FUNC_POINTERS, S_NON_LAZY_SYMBOL_POINTERS, S_SYMBOL_STUBS,
};
use std::collections::HashMap;
use std::io::{Cursor, Seek, SeekFrom};

const VM_PROT_READ: vm_prot_t = 1;
const VM_PROT_WRITE: vm_prot_t = 2;
#[allow(dead_code)]
const VM_PROT_EXECUTE: vm_prot_t = 4;

#[derive(Debug)]
pub struct MachO {
    /// Name (for debugging purposes and sorting)
    pub name: String,
    /// Paths of dynamic libraries referenced by the binary.
    pub dynamic_libraries: Vec<String>,
    /// Metadata related to sections.
    pub sections: Vec<Section>,
    /// Defined symbols in the binary (both external and local). This is a
    /// hashmap so the dynamic linker can look things up quickly. Thumb function
    /// symbols always have the Thumb bit set.
    pub exported_symbols: HashMap<String, u32>,
    /// List of addresses and names of external relocations for the dynamic
    /// linker to resolve.
    pub external_relocations: Vec<(u32, String)>,
    /// Address/program counter value for the entry point.
    pub entry_point_pc: Option<u32>,
    /// Whether the entry point came from an `LC_MAIN` load command rather
    /// than `LC_UNIXTHREAD`/`LC_THREAD`. With `LC_MAIN`, the entry point is
    /// the C `main` function itself, which dyld calls with
    /// `(argc, argv, envp, apple)` passed in registers per the calling
    /// convention, rather than a `start` routine that reads them from the
    /// stack.
    pub entry_point_is_lc_main: bool,
    /// End address of the highest-addressed segment.
    /// This is used by get_end() to return the first address after the last
    /// segment in the executable.
    pub last_segment_end: u32,
    /// Base address of the __TEXT segment (where the mach_header lives in
    /// memory). Used by `_dyld_get_image_header`.
    pub text_base: u32,
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

impl Section {
    /// Returns address of the next section.
    pub fn next_section_addr(&self) -> u32 {
        self.addr + self.size
    }
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
        _ => {
            log!(
                "Warning: cpu_subtype_to_str: unexpected cpu subtype: {:?}; using 'armv???'.",
                ty
            );
            "armv???"
        }
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
        slide_to_address: u32,
    ) -> Result<MachO, &'static str> {
        log_dbg!("Reading {:?}", name);

        let mut cursor = Cursor::new(bytes);

        let file = OFile::parse(&mut cursor).map_err(|_| "Could not parse Mach-O file")?;

        let (header, commands) = match file {
            OFile::MachFile { header, commands } => (header, commands),
            OFile::FatFile { files, .. } => {
                let mut best_subslice: Option<&[u8]> = None;
                let mut best_type = None;
                let mut had_truncated_slice = false;
                for (arch, _) in files {
                    if arch.cputype != mach_object::CPU_TYPE_ARM {
                        continue;
                    }
                    // Per Apple's Mach-O FAT documentation, each `fat_arch`
                    // gives an absolute offset and size into the wrapping FAT
                    // file. Some malformed / partially-downloaded IPAs (e.g.
                    // Bike Race Free) have a header that describes a slice
                    // larger than the bytes we actually have on disk, which
                    // used to crash with `range end index N out of range for
                    // slice of length M`. Bounds-check here and skip the slice
                    // instead of panicking; we'll fall back to whatever other
                    // slice is present, or surface a clean error.
                    let off = arch.offset as usize;
                    let size = arch.size as usize;
                    let end = off.checked_add(size);
                    let in_bounds = matches!(end, Some(e) if e <= bytes.len());
                    if !in_bounds {
                        log!(
                            "Warning: FAT slice (cputype {:#x}, cpusubtype {:#x}) declares \
                             range {:#x}..{:#x}, past end of file ({:#x}); skipping this slice.",
                            arch.cputype,
                            arch.cpusubtype,
                            off,
                            off.saturating_add(size),
                            bytes.len(),
                        );
                        had_truncated_slice = true;
                        continue;
                    }
                    if arch.cpusubtype == mach_object::CPU_SUBTYPE_ARM_V7
                        || (arch.cpusubtype == mach_object::CPU_SUBTYPE_ARM_V6
                            && best_type != Some(mach_object::CPU_SUBTYPE_ARM_V7))
                        || best_type.is_none()
                    {
                        best_subslice = Some(&bytes[off..off + size]);
                        best_type = Some(arch.cpusubtype);
                    }
                }
                return if let Some(subslice) = best_subslice {
                    MachO::load_from_bytes(subslice, into_mem, name, slide_to_address)
                } else if had_truncated_slice {
                    Err("FAT binary is truncated: declared ARM slice extends past end of file (the .ipa may be corrupt or incomplete)")
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
        let mut segment_offsets = Vec::new();
        let mut last_segment_end: u32 = 0;

        // Info used for the result
        let mut dynamic_libraries = Vec::new();
        let mut exported_symbols = HashMap::new();
        let mut indirect_undef_symbols: Vec<Option<String>> = Vec::new();
        let mut external_relocations: Vec<(u32, String)> = Vec::new();
        let mut entry_point_pc: Option<u32> = None;
        let mut entry_point_is_lc_main = false;

        let slide = slide_to_address;

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
                        first_segment_base = Some(vmaddr + slide);
                    }
                    if first_read_write_segment_base.is_none()
                        && (initprot & VM_PROT_READ) != 0
                        && (initprot & VM_PROT_WRITE) != 0
                    {
                        first_read_write_segment_base = Some(vmaddr + slide);
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
                            text_segment_base = Some(vmaddr + slide);
                            true
                        }
                        "__DATA" => true,
                        // `__RESTRICT` is a one-section segment Apple's
                        // linker emits to mark a binary as non-injectable
                        // (it disables `DYLD_INSERT_LIBRARIES` and the
                        // task-port debugging path in dyld). It contains a
                        // `__restrict` C-string section; the segment is
                        // documented in Apple's open-source dyld
                        // (`dyld/src/ImageLoaderMachO.cpp`) and xnu's
                        // `bsd/kern/mach_loader.c`. We don't enforce that
                        // policy in HLE, so treat the segment as a regular
                        // read-only data segment without spamming a warning.
                        "__RESTRICT" => true,
                        // `__S3E_DATA` is a data segment used by the Marmalade SDK
                        // (formerly Airplay SDK), which some older iOS games link with.
                        "__S3E_DATA" => true,
                        _ => {
                            log!("Warning: Unexpected segment name: {}", segname);
                            true
                        }
                    };

                    // Apple's xnu kernel (`bsd/kern/mach_loader.c`,
                    // `parse_machfile()` → `load_segment()`) explicitly
                    // skips `LC_SEGMENT` commands whose `vmsize == 0` —
                    // they reserve no address space and have no data.
                    // Some shipping iOS binaries (e.g. games containing a
                    // dummy `__RESTRICT` segment from older linkers) emit
                    // zero-vmsize segments; reserving 0 bytes used to
                    // panic our allocator (`NonZeroU32::new(0).unwrap()`
                    // in `Chunk::new`), so honour the kernel's contract
                    // and silently ignore them here.
                    if load_me && vmsize == 0 {
                        log_dbg!(
                            "Skipping zero-vmsize segment {} at {:#x} (matches xnu mach_loader.c behaviour)",
                            segname,
                            vmaddr + slide
                        );
                        all_sections.extend_from_slice(&sections);
                        segment_offsets.push(vmaddr);
                        last_segment_end = last_segment_end.max(vmaddr + slide);
                        continue;
                    }

                    if load_me {
                        log_dbg!(
                            "reserve {} addr {:#x} size {}",
                            segname,
                            vmaddr + slide,
                            vmsize
                        );
                        into_mem.reserve(vmaddr + slide, vmsize);

                        // If filesize is less than vmsize, the rest of the
                        // segment should be filled with zeroes. We are assuming
                        // the memory is already zeroed!
                        if filesize > 0 {
                            assert!(filesize <= vmsize);
                            let fileoff: usize = fileoff
                                .try_into()
                                .map_err(|_| "Segment file offset does not fit host usize")?;
                            let filesize_usize = filesize as usize;
                            let file_end = fileoff
                                .checked_add(filesize_usize)
                                .ok_or("Segment file range overflows host usize")?;
                            if file_end > bytes.len() {
                                log!(
                                    "Warning: segment {} declares file range \
                                     {:#x}..{:#x}, past end of Mach-O file \
                                     ({:#x}); loading available bytes and \
                                     leaving the rest zero-filled",
                                    segname,
                                    fileoff,
                                    file_end,
                                    bytes.len(),
                                );
                            }
                            if fileoff < bytes.len() {
                                let available_end = file_end.min(bytes.len());
                                let src = &bytes[fileoff..available_end];
                                let copy_len = src.len() as GuestUSize;
                                let dst =
                                    into_mem.bytes_at_mut(Ptr::from_bits(vmaddr + slide), copy_len);
                                dst.copy_from_slice(src);
                            }
                        }
                    }

                    all_sections.extend_from_slice(&sections);
                    segment_offsets.push(vmaddr);
                    last_segment_end = last_segment_end.max(vmaddr + vmsize + slide);
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
                                exported_symbols.insert(name.to_string(), slide + entry);
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
                            // Если имя есть (Some), оно превратится в
                            // Some(String).
                            // Если имени нет (None), вернется None, и мы
                            // избежим паники.
                            Some(Symbol::Undefined { name, .. }) => name.map(String::from),
                            Some(Symbol::Prebound { name, .. }) => name.map(String::from),
                            Some(Symbol::Defined { name, .. }) => name.map(String::from),
                            // Debug-символы по-прежнему игнорируем
                            Some(Symbol::Debug { .. }) => None,
                            None => None,
                            other => {
                                log!(
                                    "Warning: indirect symbol table contains an unexpected symbol kind {:?}; treating as anonymous.",
                                    other
                                );
                                None
                            }
                        })
                    }

                    let extrels = &bytes[extreloff as usize..][..nextrel as usize * 8];
                    for entry in extrels.chunks(8) {
                        let entry_arr: [u8; 8] = match entry.try_into() {
                            Ok(a) => a,
                            Err(_) => {
                                log!(
                                    "Warning: external relocation table entry is not 8 bytes; truncated dynamic symbol table."
                                );
                                break;
                            }
                        };
                        let reloc = Reloc::parse(is_bigend, entry_arr);
                        let Reloc::External {
                            addr,
                            sym_idx,
                            is_pc_relative: false,
                            size: 4,
                            type_: 0, // generic
                        } = reloc
                        else {
                            log!(
                                "Warning: skipping unsupported external relocation entry: {:?}",
                                reloc
                            );
                            continue;
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
                        assert_eq!(slide, 0); // TODO
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
                                let addend: u32 = into_mem.read(addr);
                                let entry = entry as u32;
                                let entry = if desc & N_ARM_THUMB_DEF != 0 {
                                    entry | GuestFunction::THUMB_BIT
                                } else {
                                    entry
                                };
                                into_mem.write(addr, entry.wrapping_add(addend));
                            }
                            Some(Symbol::Prebound { name: Some(n), .. }) => {
                                let ptr_ptr = Ptr::<u32, true>::from_bits(addr);
                                into_mem.write(ptr_ptr, 0); // Clear prebinding.
                                external_relocations.push((addr, String::from(n)));
                            }
                            other => {
                                log!(
                                    "Warning: external relocation references an unexpected symbol kind {:?}; skipping fixup at {:#x}.",
                                    other,
                                    addr
                                );
                            }
                        };
                    }
                }
                LoadCommand::EncryptionInfo { id, .. } if id != 0 => {
                    return Err("The executable is encrypted. touchHLE can't run encrypted apps!");
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
                        log!(
                            "Warning: unexpected initial thread state in {:?}: {:?}; falling back to extracted PC if present.",
                            name,
                            state
                        );
                        // Try to read the PC anyway; otherwise leave entry
                        // unset which will surface as a clear error later.
                        if let ThreadState::Arm { __pc, .. } = state {
                            if entry_point_pc.is_none() {
                                entry_point_pc = Some(__pc);
                            }
                        }
                        continue;
                    };
                    if entry_point_pc.is_some() {
                        log!(
                            "Warning: multiple initial thread states in {:?}; ignoring extra entries.",
                            name
                        );
                        continue;
                    }
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
                    entry_point_is_lc_main = true;
                }
                // Used in iOS 3.1+ apps. Also contains info about
                // weak and lazy binds, but we already handle those.
                LoadCommand::DyldInfo {
                    rebase_off,
                    rebase_size,
                    bind_off,
                    bind_size,
                    ..
                } => {
                    fn checked_dyld_info_slice<'a>(
                        bytes: &'a [u8],
                        off: u64,
                        size: u64,
                        kind: &str,
                        name: &str,
                    ) -> &'a [u8] {
                        // Convert to host usize with bounds checking — a truncated
                        // Mach-O (e.g. a corrupt IPA) can declare a dyld_info range
                        // that runs past EOF, and the raw slice panics with
                        // "range end index N out of range for slice of length M".
                        // Apple's dyld bails out on the binary; we degrade
                        // gracefully and load the rest.
                        let Ok(off_usize) = usize::try_from(off) else {
                            log!(
                                "Warning: {} dyld_info range starts at {:#x} past host usize \
                                 in {:?}; skipping.",
                                kind, off, name
                            );
                            return &[];
                        };
                        let Ok(size_usize) = usize::try_from(size) else {
                            log!(
                                "Warning: {} dyld_info range size {:#x} past host usize \
                                 in {:?}; skipping.",
                                kind, size, name
                            );
                            return &[];
                        };
                        let Some(end) = off_usize.checked_add(size_usize) else {
                            log!(
                                "Warning: {} dyld_info range {:#x}..{:#x}+{:#x} overflows \
                                 in {:?}; skipping.",
                                kind, off_usize, off_usize, size_usize, name
                            );
                            return &[];
                        };
                        if end > bytes.len() {
                            log!(
                                "Warning: {} dyld_info range {:#x}..{:#x} past end of \
                                 Mach-O file ({:#x}) in {:?}; skipping.",
                                kind, off_usize, end, bytes.len(), name
                            );
                            return &[];
                        }
                        &bytes[off_usize..end]
                    }

                    let rebase_opcodes = Rebase::parse(
                        checked_dyld_info_slice(bytes, rebase_off as u64, rebase_size as u64, "rebase", &name),
                        size_of::<GuestUSize>(),
                    );

                    for symb in rebase_opcodes {
                        match symb.symbol_type {
                            BindSymbolType::Pointer => {
                                let addr = segment_offsets[symb.segment_index]
                                    + symb.symbol_offset as u32
                                    + slide;
                                let original_location = Ptr::from_bits(addr);
                                let old: u32 = into_mem.read(original_location);
                                log_dbg!(
                                    "Pointer rebase at {:#x} from {:#x} to {:#x}",
                                    addr,
                                    old,
                                    old + slide
                                );
                                into_mem.write(original_location, old + slide);
                            }
                            other => {
                                log!(
                                    "Warning: unhandled DyldInfo rebase symbol type: {:?}; skipping.",
                                    other
                                );
                            }
                        }
                    }

                    let bind_opcodes = Bind::parse(
                        checked_dyld_info_slice(bytes, bind_off as u64, bind_size as u64, "bind", &name),
                        size_of::<GuestUSize>(),
                    );
                    for symb in bind_opcodes {
                        match symb.symbol_type {
                            BindSymbolType::Pointer => {
                                let addr = segment_offsets[symb.segment_index]
                                    + symb.symbol_offset as u32
                                    + slide;
                                log_dbg!("Pointer bind: {:#x} -> {}", addr, symb.name);
                                external_relocations.push((addr, symb.name));
                            }
                            other => {
                                log!(
                                    "Warning: unhandled DyldInfo bind symbol type: {:?}; skipping.",
                                    other
                                );
                            }
                        }
                    }
                }
                _ => (),
            }
        }

        let sections = all_sections
            .iter()
            .map(|section| {
                let section = &**section;

                let name = section.sectname.clone();
                let addr: u32 = TryInto::<u32>::try_into(section.addr).unwrap() + slide;
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
                    assert!(size.is_multiple_of(entry_size));
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
            entry_point_is_lc_main,
            last_segment_end,
            text_base: text_segment_base.unwrap_or(first_segment_base.unwrap_or(0)),
        })
    }

    /// Load the all the sections from a Mach-O binary (from `path`) into the
    /// guest memory (`into_mem`), and return a struct containing metadata
    /// (e.g. symbols).
    pub fn load_from_file<P: AsRef<GuestPath>>(
        path: P,
        fs: &Fs,
        into_mem: &mut Mem,
        slide_to_address: u32,
    ) -> Result<MachO, &'static str> {
        let name = path.as_ref().file_name().unwrap().to_string();
        Self::load_from_bytes(
            &fs.read(path.as_ref())
                .map_err(|_| "Could not read executable file")?,
            into_mem,
            name,
            slide_to_address,
        )
    }

    /// Get a section by its name (`&str`) or type ([SectionType]).
    pub fn get_section<P: SectionPredicate>(&self, by: P) -> Option<&Section> {
        self.sections.iter().find(|section| by.test(section))
    }
}
