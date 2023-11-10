/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Implementation of the GDB Remote Serial Protocol. This implements a server;
//! the client would be something like GDB or LLDB.
//!
//! Useful resources:
//! - [Debugging with GDB, Appendix E: GDB Remote Serial Protocol](https://sourceware.org/gdb/onlinedocs/gdb/Remote-Protocol.html)
//! - The GDB source code:
//!   - `include/gdb/signals.def` for the meanings of signal numbers
//!   - `gdb/arch/arm.h` for ARMv6 register numbers

use crate::cpu::{Cpu, CpuError};
use crate::mem::{GuestUSize, Mem, Ptr};
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// GDB target description XML.
const TARGET_XML: &str = r#"
<target version="1.0">
    <architecture>armv6</architecture>
    <osabi>Darwin</osabi>
</target>
"#;

/// GDB Remote Serial Protocol handler, implementing a server.
pub struct GdbServer {
    reader: BufReader<TcpStream>,
    first_halt: bool,
}

impl GdbServer {
    /// Create the handler from a TCP connection.
    pub fn new(mut connection: TcpStream) -> GdbServer {
        connection
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        connection
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();

        let mut hello_byte = [0u8; 1];
        connection
            .read_exact(&mut hello_byte)
            .expect("Could not read greeting");
        assert!(hello_byte[0] == b'+');

        connection.write_all(b"+").expect("Could not send greeting");

        GdbServer {
            reader: BufReader::with_capacity(4096, connection),
            first_halt: true,
        }
    }

    fn read_packet(&mut self) -> Option<String> {
        let buffer = match self.reader.fill_buf() {
            Ok(buffer) => buffer,
            Err(e) => match e.kind() {
                ErrorKind::BrokenPipe | ErrorKind::ConnectionReset => {
                    panic!("Lost connection to debugger: {}", e.kind());
                }
                _ => return None,
            },
        };

        if buffer.is_empty() {
            return None;
        }

        // Packets begin with '$', followed by the main content, followed by
        // '#', followed by a two-digit checksum in hexadecimal.
        // Except when some optional extensions are enabled, the content is
        // always ASCII.

        if buffer[0] == b'+' {
            // This is just an acknowledgment
            self.reader.consume(1);
            log_dbg!("Got ACK");
            return None;
        }

        // This is a normal packet
        assert_eq!(buffer[0], b'$');

        let Some(body_end) = buffer.iter().position(|&c| c == b'#') else {
            // Assumption: packet will never be longer than the maximum buffer
            // size, so if the buffer's full and we don't find a terminator, the
            // data must be invalid or we've parsed it wrong.
            assert!(buffer.len() != self.reader.capacity());
            log_dbg!("No packet end yet");
            return None;
        };

        let body = &buffer[1..body_end];

        let checksum1 = buffer.get((body_end + 1)..(body_end + 3))?;
        log_dbg!("Have full packet");

        let checksum1 = std::str::from_utf8(checksum1).unwrap();
        let checksum1 = u8::from_str_radix(checksum1, 16).unwrap();
        let checksum2 = body.iter().fold(0u8, |a, &b| a.wrapping_add(b));
        assert_eq!(checksum1, checksum2);

        let body = String::from_utf8(body.to_vec()).unwrap();
        self.reader.consume(body_end + 3);

        log_dbg!("Got packet: {:?}", body);

        // Send acknowledgment
        self.reader
            .get_mut()
            .write_all(b"+")
            .expect("Couldn't send ACK");

        Some(body)
    }

    fn send_packet(&mut self, body: &str) {
        let checksum = body.bytes().fold(0u8, |a, b| a.wrapping_add(b));
        write!(self.reader.get_mut(), "${}#{:02x}", body, checksum).unwrap();
        log_dbg!("Sent packet: {:?}", body);
    }

    /// Communciates with the debugger, returning only once it requests
    /// execution should continue. Returns [true] if the CPU should step and
    /// then resume debugging, or [false] if it should resume normal execution.
    #[must_use]
    pub fn wait_for_debugger(
        &mut self,
        stop_reason: Option<CpuError>,
        cpu: &mut Cpu,
        mem: &mut Mem,
    ) -> bool {
        echo!("Waiting for debugger to continue.");

        // Send reply to continue/step packet that gdb sent earlier, so it knows
        // why execution was stopped.
        match stop_reason {
            None => {
                if self.first_halt {
                    // The debugger has just connected, it hasn't sent anything
                    // yet.
                    self.first_halt = false;
                } else {
                    // The debugger previously requested stepping and no errors
                    // occurred.
                    self.send_packet("S05"); // SIGTRAP
                }
            }
            // GDB uses an undefined instruction for software breakpoints in
            // normal Arm code, and the BKPT instruction in Thumb code.
            // It apparently expects SIGTRAP instead of SIGILL even in the
            // former case.
            Some(CpuError::UndefinedInstruction) | Some(CpuError::Breakpoint) => {
                self.send_packet("S05"); // SIGTRAP
            }
            Some(CpuError::MemoryError) => {
                self.send_packet("S0b"); // SIGSEGV
            }
        }

        let do_step = loop {
            let Some(p) = self.read_packet() else {
                continue;
            };

            if p.is_empty() {
                continue;
            };

            match p.as_bytes()[0] {
                // Query for target halt reason when first connecting
                b'?' => {
                    assert!(stop_reason.is_none());
                    self.send_packet("S00"); // no signal
                }
                // Read general registers
                b'g' => {
                    let mut packet = String::with_capacity(16 * 4 * 2);
                    for reg in cpu.regs() {
                        // Rust always prints in big-endian, but GDB expects
                        // little-endian.
                        let reg = u32::from_be_bytes(reg.to_le_bytes());
                        write!(packet, "{:08x}", reg).unwrap();
                    }
                    self.send_packet(&packet);
                }
                // Write general registers
                b'G' => {
                    let data = &p[1..];
                    let regs = cpu.regs_mut();
                    assert!(data.len() == regs.len() * 4 * 2);
                    for (i, reg) in regs.iter_mut().enumerate() {
                        let word = &data[i * 4 * 2..][..4 * 2];
                        let word = u32::from_str_radix(word, 16).unwrap();
                        // Rust decodes in big-endian, but GDB supplies
                        // little-endian.
                        let word = u32::from_le_bytes(word.to_be_bytes());
                        *reg = word;
                    }
                    self.send_packet("OK");
                }
                // Read single register by number
                b'p' => {
                    let num = usize::from_str_radix(&p[1..], 16).unwrap();
                    let reg = if num < 16 {
                        Some(cpu.regs()[num])
                    } else if num == 25 {
                        Some(cpu.cpsr())
                    // TODO: FPSCR, VFP registers
                    } else {
                        None
                    };
                    if let Some(reg) = reg {
                        // Rust always prints in big-endian, but GDB expects
                        // little-endian.
                        let reg = u32::from_be_bytes(reg.to_le_bytes());
                        self.send_packet(&format!("{:08x}", reg));
                    } else {
                        // Error 0
                        self.send_packet("E00");
                    }
                }
                // Write single register by number
                b'P' => {
                    let (num, word) = p[1..].split_once('=').unwrap();
                    let num = usize::from_str_radix(num, 16).unwrap();
                    let word = u32::from_str_radix(word, 16).unwrap();
                    // Rust decodes in big-endian, but GDB supplies
                    // little-endian.
                    let word = u32::from_le_bytes(word.to_be_bytes());
                    if num < 16 {
                        cpu.regs_mut()[num] = word;
                        self.send_packet("OK");
                    } else if num == 25 {
                        cpu.set_cpsr(word);
                        self.send_packet("OK");
                    // TODO: FPSCR, VFP registers
                    } else {
                        // Error 0
                        self.send_packet("E00");
                    }
                }
                // Read memory
                b'm' => {
                    let (addr, length) = p[1..].split_once(',').unwrap();
                    let addr = GuestUSize::from_str_radix(addr, 16).unwrap();
                    let length = GuestUSize::from_str_radix(length, 16).unwrap();
                    let mut packet = String::with_capacity(length as usize * 2);
                    match mem.get_bytes_fallible(Ptr::from_bits(addr), length) {
                        Some(data) => {
                            for byte in data {
                                write!(packet, "{:02x}", byte).unwrap();
                            }
                        }
                        None => {
                            // Error 0
                            write!(packet, "E00").unwrap()
                        }
                    }
                    self.send_packet(&packet);
                }
                // Write memory
                b'M' => {
                    let (header, data) = p[1..].split_once(':').unwrap();
                    let (addr, length) = header.split_once(',').unwrap();
                    let addr = GuestUSize::from_str_radix(addr, 16).unwrap();
                    let length = GuestUSize::from_str_radix(length, 16).unwrap();
                    assert!(data.len() == length as usize * 2);

                    match mem.get_bytes_fallible_mut(Ptr::from_bits(addr), length) {
                        Some(dest) => {
                            for i in 0..(length as usize) {
                                let byte = &data[i * 2..][..2];
                                let byte = u8::from_str_radix(byte, 16).unwrap();
                                dest[i] = byte;
                            }
                            // Important for e.g. software breakpoints.
                            cpu.invalidate_cache_range(addr, length);
                            self.send_packet("OK");
                        }
                        None => {
                            // Error 0
                            self.send_packet("E00");
                        }
                    }
                }
                // Continue or Step
                b'c' | b's' => {
                    let addr = &p[1..];
                    if !addr.is_empty() {
                        todo!("TODO: Resume at {}", addr);
                    }
                    break p.as_bytes()[0] == b's';
                }
                // "Continue with signal" or "Step with signal".
                // Presumably "with" means "ignoring"?
                b'C' | b'S' => {
                    // Signal is just ignored for now (TODO?)
                    if let Some((_signal, addr)) = p[1..].split_once(';') {
                        todo!("TODO: Resume at {}", addr);
                    }
                    break p.as_bytes()[0] == b'S';
                }
                // Kill
                b'k' => {
                    panic!("Debugger requested kill.");
                }
                _ => {
                    // Query whether we're attaching to an existing or new
                    // process
                    if p == "qAttached" {
                        // New process
                        self.send_packet("0");
                    // Query for supported features
                    } else if p == "qSupported" || p.starts_with("qSupported:") {
                        // Tell GDB we can send it an XML target description.
                        self.send_packet("qXfer:features:read+");
                    // Read XML target description
                    } else if let Some(params) = p.strip_prefix("qXfer:features:read:") {
                        let (annex, params) = params.split_once(':').unwrap();
                        let (offset, length) = params.split_once(',').unwrap();
                        let offset = usize::from_str_radix(offset, 16).unwrap();
                        let length = usize::from_str_radix(length, 16).unwrap();
                        let bytes = TARGET_XML.as_bytes();
                        if annex == "target.xml" && offset <= bytes.len() {
                            let bytes = &bytes[offset..];
                            let length_read = length.min(bytes.len());
                            let mut packet = String::with_capacity(1 + length_read);
                            if length_read < length {
                                // Read data, more remains
                                packet.push('l');
                            } else {
                                // Read data, none left
                                packet.push('m');
                            }
                            // This packet uses the modern style of binary
                            // data where most bytes are unescaped.
                            // We happen to know none of the bytes in the XML
                            // need escaping, and that they're all ASCII.
                            packet.push_str(std::str::from_utf8(&bytes[..length_read]).unwrap());
                            self.send_packet(&packet);
                        } else {
                            // Unsupported annex or invalid offset
                            self.send_packet("E00");
                        }
                    } else {
                        log_dbg!("Unhandled packet.");
                        // Tell GDB we don't understand this packet.
                        // In some cases this causes convenient fallbacks:
                        // Since we don't support 'Z', GDB will implement
                        // software breakpoints for us with trap instructions.
                        self.send_packet("");
                    }
                }
            }
        };

        if do_step {
            echo!("Debugger requested step, resuming execution for one instruction only.");
        } else {
            echo!("Debugger requested continue, resuming execution.");
        }
        do_step
    }
}
