//! Reading of Mach-O files, the executable and library format on iPhone OS.
//! Currently only handles executables.
//!
//! Implemented using the mach_object crate. All usage of that crate should be
//! confined to this module. The goal is to read the Mach-O binary exactly once,
//! storing any information we'll need later.
//!
//! Useful resources:
//! - Apple's [Overview of the Mach-O Executable Format](https://developer.apple.com/library/archive/documentation/Performance/Conceptual/CodeFootprint/Articles/MachOOverview.html) explains what "segments" and "sections" are, and provides short descriptions of the purposes of some common sections.
//! - Alex Drummond's [Inside a Hello World executable on OS X](https://adrummond.net/posts/macho) is about macOS circa 2017 rather than iPhone OS circa 2008, so not all of what it says applies, but the sections up to and including "9. The indirect symbol table" are helpful.
//! - The LLVM functions [`RuntimeDyldMachO::populateIndirectSymbolPointersSection`](https://github.com/llvm/llvm-project/blob/2e999b7dd1934a44d38c3a753460f1e5a217e9a5/llvm/lib/ExecutionEngine/RuntimeDyld/RuntimeDyldMachO.cpp#L179-L220) and [`MachOObjectFile::getIndirectSymbolTableEntry`](https://github.com/llvm/llvm-project/blob/3c09ed006ab35dd8faac03311b14f0857b01949c/llvm/lib/Object/MachOObjectFile.cpp#L4803-L4808) are references for how to read the indirect symbol table.

use crate::memory::{Memory, Ptr};
use mach_object::{DyLib, LoadCommand, MachCommand, OFile, Symbol, SymbolIter};
use std::io::{Cursor, Seek, SeekFrom};

#[derive(Debug)]
pub struct MachO {
    /// Address of the entry-point procedure (aka `start`).
    pub entry_point_addr: Option<u32>,
    /// Paths of dynamic libraries referenced by the binary.
    pub dynamic_libraries: Vec<String>,
    /// Metadata related to sections.
    pub sections: Vec<Section>,
}

#[derive(Debug)]
pub struct Section {
    /// Section name.
    pub name: String,
    /// Section address in memory.
    pub addr: u32,
    /// Section size in bytes.
    pub size: u32,
    /// Information specific to special dynamic linker sections, if this is one.
    pub dyld_indirect_symbol_info: Option<DyldIndirectSymbolInfo>,
}

/// Information relevant to certain special sections which contain a series of
/// pointers or stub functions for indirectly referencing dynamically-linked
/// symbols.
#[derive(Debug)]
pub struct DyldIndirectSymbolInfo {
    /// The size in bytes of an entry (pointer or stub function) in the section.
    pub entry_size: u32,
    /// A list of symbol names corresponding to the entries.
    pub indirect_undef_symbols: Vec<Option<String>>,
}

impl MachO {
    /// Load the all the sections from a Mach-O binary (provided as `bytes`)
    /// into the guest memory (`into_mem`), and return a struct containing
    /// metadata (e.g. symbols).
    pub fn load_from_bytes(bytes: &[u8], into_mem: &mut Memory) -> Result<MachO, &'static str> {
        let mut cursor = Cursor::new(bytes);

        let file = OFile::parse(&mut cursor).map_err(|_| "Could not parse Mach-O file")?;

        let (header, commands) = match file {
            OFile::MachFile { header, commands } => (header, commands),
            OFile::FatFile { .. } => {
                unimplemented!("Fat binary support is not implemented yet");
            }
            OFile::ArFile { .. } | OFile::SymDef { .. } => {
                return Err("Unexpected Mach-O file kind: not an executable");
            }
        };

        if header.cputype != mach_object::CPU_TYPE_ARM {
            return Err("Executable is not for an ARM CPU!");
        }
        let is_bigend = header.is_bigend();
        if is_bigend {
            return Err("Executable is not little-endian!");
        }
        let is_64bit = header.is_64bit();
        if is_64bit {
            return Err("Executable is not 32-bit!");
        }
        // TODO: Check cpusubtype (should be some flavour of ARMv6/ARMv7)

        // Info used while parsing file
        let mut all_sections = Vec::new();
        let mut sym_tab_info: Option<(u32, u32, u32, u32)> = None;

        // Info used for the result
        let mut entry_point_addr: Option<u32> = None;
        let mut dynamic_libraries = Vec::new();
        let mut indirect_undef_symbols: Vec<Option<String>> = Vec::new();

        for MachCommand(command, _size) in commands {
            match command {
                LoadCommand::Segment {
                    vmaddr,
                    vmsize,
                    fileoff,
                    filesize,
                    sections,
                    ..
                } => {
                    assert!(filesize <= vmsize);
                    // Copy the bytes from the file into memory. Note that
                    // filesize may be less than vmsize, in which case the rest
                    // of the segment should be filled with zeroes. This code
                    // is assuming the memory is already zeroed.
                    {
                        let src = &bytes[fileoff..][..filesize];
                        let dst = into_mem.bytes_at_mut(
                            Ptr::from_bits(vmaddr.try_into().unwrap()),
                            filesize.try_into().unwrap(),
                        );
                        dst.copy_from_slice(src);
                    };
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
                                name: Some("start"),
                                entry,
                                ..
                            } = symbol
                            {
                                entry_point_addr = Some(entry.try_into().unwrap());
                            }
                        }
                    }
                }
                LoadCommand::DySymTab {
                    indirectsymoff,
                    nindirectsyms,
                    ..
                } => {
                    let (symoff, nsyms, stroff, strsize) = sym_tab_info.unwrap();
                    let table = &bytes[indirectsymoff as usize..][..nindirectsyms as usize * 4];
                    for i in 0..nindirectsyms {
                        let idx = &table[i as usize * 4..][..4];
                        assert!(!is_bigend);
                        let idx = u32::from_le_bytes(idx.try_into().unwrap());

                        indirect_undef_symbols.push(if idx >= nsyms {
                            None
                        } else {
                            let symoff = (symoff + idx * 12) as u64;
                            cursor.seek(SeekFrom::Start(symoff)).unwrap();

                            let mut cursor = cursor.clone();
                            // This is not how you're supposed to use SymbolIter
                            // but the parse_symbol() method on it requires the
                            // bytestring crate, so...
                            let mut iter = SymbolIter::new(
                                &mut cursor,
                                Vec::new(),
                                1,
                                stroff,
                                strsize,
                                is_bigend,
                                is_64bit,
                            );
                            match iter.next() {
                                Some(Symbol::Undefined { name: Some(n), .. }) => {
                                    Some(String::from(n))
                                }
                                _ => None,
                            }
                        })
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
                // LoadCommand::DyldInfo is apparently a newer thing that 2008
                // games don't have. Ignore for now? Unsure if/when iOS got it.
                LoadCommand::DyldInfo { .. } => {
                    eprintln!("Warning! DyldInfo is not handled.");
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

                let dyld_indirect_symbol_info = match &*name {
                    "__symbol_stub4" => Some(12),
                    "__nl_symbol_ptr" | "__la_symbol_ptr" => Some(4),
                    _ => None,
                }
                .map(|entry_size| {
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
                    dyld_indirect_symbol_info,
                }
            })
            .collect();

        Ok(MachO {
            entry_point_addr,
            dynamic_libraries,
            sections,
        })
    }

    /// Load the all the sections from a Mach-O binary (from `path`) into the
    /// guest memory (`into_mem`), and return a struct containing metadata
    /// (e.g. symbols).
    pub fn load_from_file<P: AsRef<std::path::Path>>(
        path: P,
        into_mem: &mut Memory,
    ) -> Result<MachO, &'static str> {
        Self::load_from_bytes(
            &std::fs::read(path).map_err(|_| "Could not read executable file")?,
            into_mem,
        )
    }

    pub fn get_section(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.name == name)
    }
}
