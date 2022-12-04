//! Reading of Mach-O files, the executable and library format on iPhone OS.
//! Currently only handles executables.
//!
//! Implemented using the mach_object crate. All usage of that crate should be
//! confined to this module.

use mach_object::{DyLib, LoadCommand, MachCommand, OFile};
use std::io::Cursor;

pub struct MachO {}

impl MachO {
    pub fn from_bytes(bytes: &[u8]) -> Result<MachO, &'static str> {
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
        // TODO: Check cpusubtype (should be some flavour of ARMv6/ARMv7)

        for MachCommand(command, _size) in commands {
            match command {
                LoadCommand::Segment {
                    segname,
                    vmaddr,
                    vmsize,
                    sections,
                    ..
                } => {
                    println!(
                        "Segment: {:?} ({:#x}–{:#x})",
                        segname,
                        vmaddr,
                        vmaddr + vmsize
                    );
                    for section in sections {
                        println!("- Section: {:?}", section.sectname);
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
                _ => (),
            }
        }

        // TODO: actually read stuff

        Ok(MachO {})
    }

    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<MachO, &'static str> {
        Self::from_bytes(&std::fs::read(path).map_err(|_| "Could not read executable file")?)
    }
}
