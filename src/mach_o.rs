//! Reading of Mach-O files, the executable and library format on iPhone OS.
//! Currently only handles executables.
//!
//! Implemented using the mach_object crate. All usage of that crate should be
//! confined to this module. The goal is to read the Mach-O binary exactly once,
//! storing any information we'll need later.

use crate::memory::{Memory, Ptr};
use mach_object::{DyLib, LoadCommand, MachCommand, OFile, Symbol, SymbolIter};
use std::io::{Cursor, Seek, SeekFrom};

pub struct MachO {
    pub entry_point_addr: Option<u32>,
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
        if header.is_bigend() {
            return Err("Executable is not little-endian!");
        }
        if header.is_64bit() {
            return Err("Executable is not 32-bit!");
        }
        // TODO: Check cpusubtype (should be some flavour of ARMv6/ARMv7)

        let mut all_sections = Vec::new();

        let mut entry_point_addr: Option<u32> = None;

        for MachCommand(command, _size) in commands {
            match command {
                LoadCommand::Segment {
                    segname,
                    vmaddr,
                    vmsize,
                    fileoff,
                    filesize,
                    sections,
                    ..
                } => {
                    println!(
                        "Segment: {:?} ({:#x}–{:#x}), {:#x} bytes from file",
                        segname,
                        vmaddr,
                        vmaddr + vmsize,
                        filesize
                    );
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
                    for section in &sections {
                        println!("- Section: {:?}", section.sectname);
                    }
                    all_sections.extend_from_slice(&sections);
                }
                LoadCommand::SymTab {
                    symoff,
                    nsyms,
                    stroff,
                    strsize,
                } => {
                    println!("Symbol table:");
                    if cursor.seek(SeekFrom::Start(symoff.into())).is_ok() {
                        let mut cursor = cursor.clone();
                        let symbols = SymbolIter::new(
                            &mut cursor,
                            all_sections.clone(),
                            nsyms,
                            stroff,
                            strsize,
                            /* big endian: */ false,
                            /* 64-bit: */ false,
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
                LoadCommand::EncryptionInfo { id, .. } => {
                    if id != 0 {
                        return Err(
                            "The executable is encrypted. touchHLE can't run encrypted apps!",
                        );
                    }
                }
                LoadCommand::LoadDyLib(DyLib { name, .. }) => {
                    println!("Dynamic library: {:?}", name);
                }
                // LoadCommand::DyldInfo is apparently a newer thing that 2008
                // games don't have. Ignore for now? Unsure if/when iOS got it.
                LoadCommand::DyldInfo { .. } => {
                    eprintln!("Warning! DyldInfo is not handled.");
                }
                _ => (),
            }
        }

        Ok(MachO { entry_point_addr })
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
}
