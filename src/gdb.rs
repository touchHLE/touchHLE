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

use crate::cpu::CpuError;
use crate::environment::{Environment, ThreadState};
use crate::mem::{GuestUSize, Ptr};
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// GDB target description XML.
/// Copied from binutils-gdb @ gdb/features/arm/(arm-core.xml & arm-vfpv3.xml)
const TARGET_XML: &str = r#"
<target version="1.0">
<!-- Copyright (C) 2007-2025 Free Software Foundation, Inc.

     Copying and distribution of this file, with or without modification,
     are permitted in any medium without royalty provided the copyright
     notice and this notice are preserved.  -->
<architecture>armv7</architecture>
<osabi>Darwin</osabi>
<feature name="org.gnu.gdb.arm.core">
  <reg name="r0" bitsize="32"/>
  <reg name="r1" bitsize="32"/>
  <reg name="r2" bitsize="32"/>
  <reg name="r3" bitsize="32"/>
  <reg name="r4" bitsize="32"/>
  <reg name="r5" bitsize="32"/>
  <reg name="r6" bitsize="32"/>
  <reg name="r7" bitsize="32"/>
  <reg name="r8" bitsize="32"/>
  <reg name="r9" bitsize="32"/>
  <reg name="r10" bitsize="32"/>
  <reg name="r11" bitsize="32"/>
  <reg name="r12" bitsize="32"/>
  <reg name="sp" bitsize="32" type="data_ptr"/>
  <reg name="lr" bitsize="32"/>
  <reg name="pc" bitsize="32" type="code_ptr"/>

  <!-- The CPSR is register 25, rather than register 16, because
       the FPA registers historically were placed between the PC
       and the CPSR in the "g" packet.  -->
  <reg name="cpsr" bitsize="32" regnum="25"/>
</feature>
<feature name="org.gnu.gdb.arm.vfp">
  <reg name="d0" bitsize="64" type="ieee_double"/>
  <reg name="d1" bitsize="64" type="ieee_double"/>
  <reg name="d2" bitsize="64" type="ieee_double"/>
  <reg name="d3" bitsize="64" type="ieee_double"/>
  <reg name="d4" bitsize="64" type="ieee_double"/>
  <reg name="d5" bitsize="64" type="ieee_double"/>
  <reg name="d6" bitsize="64" type="ieee_double"/>
  <reg name="d7" bitsize="64" type="ieee_double"/>
  <reg name="d8" bitsize="64" type="ieee_double"/>
  <reg name="d9" bitsize="64" type="ieee_double"/>
  <reg name="d10" bitsize="64" type="ieee_double"/>
  <reg name="d11" bitsize="64" type="ieee_double"/>
  <reg name="d12" bitsize="64" type="ieee_double"/>
  <reg name="d13" bitsize="64" type="ieee_double"/>
  <reg name="d14" bitsize="64" type="ieee_double"/>
  <reg name="d15" bitsize="64" type="ieee_double"/>
  <reg name="d16" bitsize="64" type="ieee_double"/>
  <reg name="d17" bitsize="64" type="ieee_double"/>
  <reg name="d18" bitsize="64" type="ieee_double"/>
  <reg name="d19" bitsize="64" type="ieee_double"/>
  <reg name="d20" bitsize="64" type="ieee_double"/>
  <reg name="d21" bitsize="64" type="ieee_double"/>
  <reg name="d22" bitsize="64" type="ieee_double"/>
  <reg name="d23" bitsize="64" type="ieee_double"/>
  <reg name="d24" bitsize="64" type="ieee_double"/>
  <reg name="d25" bitsize="64" type="ieee_double"/>
  <reg name="d26" bitsize="64" type="ieee_double"/>
  <reg name="d27" bitsize="64" type="ieee_double"/>
  <reg name="d28" bitsize="64" type="ieee_double"/>
  <reg name="d29" bitsize="64" type="ieee_double"/>
  <reg name="d30" bitsize="64" type="ieee_double"/>
  <reg name="d31" bitsize="64" type="ieee_double"/>

  <reg name="fpscr" bitsize="32" type="int" group="float"/>
</feature>
</target>
"#;

/// GDB Remote Serial Protocol handler, implementing a server.
pub struct GdbServer {
    reader: BufReader<TcpStream>,
    thread_for_run: isize,
    thread_for_other: isize,
    reader_is_nonblocking: bool,
    first_char_ne_break: bool,
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

        connection.set_nonblocking(false).unwrap();

        GdbServer {
            reader: BufReader::with_capacity(4096, connection),
            thread_for_run: 0,
            thread_for_other: 0,
            first_halt: true,
            first_char_ne_break: false,
            reader_is_nonblocking: true,
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

        self.first_char_ne_break = false;

        // Packets begin with '$', followed by the main content, followed by
        // '#', followed by a two-digit checksum in hexadecimal.
        // Except when some optional extensions are enabled, the content is
        // always ASCII.

        if buffer[0] == b'+' {
            // This is just an acknowledgment
            self.reader.consume(1);
            log_dbg!("Got ACK");
            return None;
        } else if buffer[0] == 0x03 {
            // Ctrl-C, can ignore since we're already in the debugger
            self.reader.consume(1);
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
        write!(self.reader.get_mut(), "${body}#{checksum:02x}").unwrap();
        log_dbg!("Sent packet: {:?}", body);
    }

    /// Communciates with the debugger, returning only once it requests
    /// execution should continue. Returns [true] if the CPU should step and
    /// then resume debugging, or [false] if it should resume normal execution.
    pub fn wait_for_debugger(&mut self, stop_reason: Option<CpuError>, env: &mut Environment) {
        echo!("Waiting for debugger to continue.");

        if self.reader_is_nonblocking {
            self.reader.get_mut().set_nonblocking(false).unwrap();
            self.reader_is_nonblocking = false;
        }

        fn regs_for_command<'b>(
            server: &mut GdbServer,
            env: &'b mut Environment,
        ) -> &'b mut [u32; 16] {
            let tid = server.thread_for_other;
            // TID zero should mean any thread, but gdb expects it to
            // mean the current thread.
            let tid: usize = if tid == 0 {
                env.current_thread
            } else {
                (tid - 1).try_into().unwrap()
            };
            if tid == env.current_thread {
                env.cpu.regs_mut()
            } else {
                &mut env.threads[tid].guest_context.as_mut().unwrap().regs
            }
        }

        fn extregs_for_command<'b>(
            server: &mut GdbServer,
            env: &'b mut Environment,
        ) -> &'b mut [u64; 32] {
            let tid = server.thread_for_other;
            // TID zero should mean any thread, but gdb expects it to
            // mean the current thread.
            let tid: usize = if tid == 0 {
                env.current_thread
            } else {
                (tid - 1).try_into().unwrap()
            };
            if tid == env.current_thread {
                env.cpu.extregs_mut()
            } else {
                &mut env.threads[tid].guest_context.as_mut().unwrap().extregs
            }
        }

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
                    self.send_packet(format!("T05thread:{:x};", env.current_thread + 1).as_str());
                    // SIGTRAP
                }
            }
            // GDB uses an undefined instruction for software breakpoints in
            // normal Arm code, and the BKPT instruction in Thumb code.
            // It apparently expects SIGTRAP instead of SIGILL even in the
            // former case.
            Some(CpuError::UndefinedInstruction) | Some(CpuError::Breakpoint) => {
                self.send_packet(format!("T05thread:{:x};", env.current_thread + 1).as_str());
                // SIGTRAP
            }
            Some(CpuError::MemoryError) => {
                self.send_packet(format!("T0bthread:{:x};", env.current_thread + 1).as_str());
                // SIGSEGV
            }
            Some(CpuError::Interrupt) => {
                self.send_packet(format!("T02thread:{:x};", env.current_thread + 1).as_str());
                // SIGINT
            }
        }

        'packet_loop: loop {
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
                    let regs = regs_for_command(self, env);
                    let mut packet = String::with_capacity(16 * 4 * 2);
                    for reg in regs {
                        // Rust always prints in big-endian, but GDB expects
                        // little-endian.
                        let reg = u32::from_be_bytes(reg.to_le_bytes());
                        write!(packet, "{reg:08x}").unwrap();
                    }
                    self.send_packet(&packet);
                }
                // Write general registers
                b'G' => {
                    let data = &p[1..];
                    let regs = regs_for_command(self, env);
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
                    let tid = self.thread_for_other;
                    let tid: usize = if tid == 0 {
                        env.current_thread
                    } else {
                        (tid - 1).try_into().unwrap()
                    };
                    let reg = if num < 16 {
                        let regs = regs_for_command(self, env);
                        Some(regs[num])
                    } else if num == 25 {
                        if tid == env.current_thread {
                            Some(env.cpu.cpsr())
                        } else {
                            Some(env.threads[tid].guest_context.as_ref().unwrap().cpsr)
                        }
                    } else if num > 25 && num < 58 {
                        // The vfp registers are 64 bit.
                        let regs = extregs_for_command(self, env);
                        let reg = u64::from_be_bytes(regs[num - 26].to_le_bytes());
                        self.send_packet(&format!("{reg:016x}"));
                        continue;
                    } else if num == 58 {
                        if tid == env.current_thread {
                            Some(env.cpu.fpscr())
                        } else {
                            Some(env.threads[tid].guest_context.as_ref().unwrap().fpscr)
                        }
                    } else {
                        None
                    };
                    if let Some(reg) = reg {
                        // Rust always prints in big-endian, but GDB expects
                        // little-endian.
                        let reg = u32::from_be_bytes(reg.to_le_bytes());
                        self.send_packet(&format!("{reg:08x}"));
                    } else {
                        // Error 0
                        self.send_packet("E00");
                    }
                }
                // Write single register by number
                b'P' => {
                    let tid = self.thread_for_other;
                    let tid: usize = if tid == 0 {
                        env.current_thread
                    } else {
                        (tid - 1).try_into().unwrap()
                    };
                    let (num, word) = p[1..].split_once('=').unwrap();
                    let num = usize::from_str_radix(num, 16).unwrap();
                    if num > 25 && num < 58 {
                        let word = u64::from_str_radix(word, 16).unwrap();
                        let regs = extregs_for_command(self, env);
                        regs[num - 26] = word;
                        self.send_packet("E00");
                        continue;
                    }
                    let word = u32::from_str_radix(word, 16).unwrap();
                    // Rust decodes in big-endian, but GDB supplies
                    // little-endian.
                    let word = u32::from_le_bytes(word.to_be_bytes());
                    if num < 16 {
                        let regs = regs_for_command(self, env);
                        regs[num] = word;
                        self.send_packet("OK");
                    } else if num == 25 {
                        if tid == env.current_thread {
                            env.cpu.set_cpsr(word);
                        } else {
                            env.threads[tid].guest_context.as_mut().unwrap().cpsr = word;
                        }
                        self.send_packet("OK");
                    } else if num == 58 {
                        if tid == env.current_thread {
                            env.cpu.set_fpscr(word);
                        } else {
                            env.threads[tid].guest_context.as_mut().unwrap().fpscr = word;
                        }
                        self.send_packet("OK");
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
                    match env.mem.get_bytes_fallible(Ptr::from_bits(addr), length) {
                        Some(data) => {
                            for byte in data {
                                write!(packet, "{byte:02x}").unwrap();
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

                    match env.mem.get_bytes_fallible_mut(Ptr::from_bits(addr), length) {
                        Some(dest) => {
                            for i in 0..(length as usize) {
                                let byte = &data[i * 2..][..2];
                                let byte = u8::from_str_radix(byte, 16).unwrap();
                                dest[i] = byte;
                            }
                            // Important for e.g. software breakpoints.
                            env.cpu.invalidate_cache_range(addr, length);
                            self.send_packet("OK");
                        }
                        None => {
                            // Error 0
                            self.send_packet("E00");
                        }
                    }
                }
                // Continue or "Continue with signal".
                // Presumably "with" means "ignoring"?
                b'c' | b'C' => {
                    // Signal is just ignored for now (TODO?)
                    if p.as_bytes()[0] == b'c' {
                        let addr = &p[1..];
                        if !addr.is_empty() {
                            todo!("TODO: Resume at {}", addr);
                        }
                    } else if let Some((_signal, addr)) = p[1..].split_once(';') {
                        todo!("TODO: Resume at {}", addr);
                    }
                    for thread in env.threads.iter_mut() {
                        if !thread.is_alive() {
                            continue;
                        } else {
                            // TODO: Is this actually the correct behaviour?
                            // (This is effectively the same as
                            // set scheduler-locking step)
                            // It's probably not a big deal, since any
                            // reasonably new version of gdb will use vCont.
                            thread.state = ThreadState::Running;
                        }
                    }

                    break;
                }
                // Step or "Step with signal".
                b's' | b'S' => {
                    // Signal is just ignored for now (TODO?)
                    if p.as_bytes()[0] == b's' {
                        let addr = &p[1..];
                        if !addr.is_empty() {
                            todo!("TODO: Resume at {}", addr);
                        }
                    } else if let Some((_signal, addr)) = p[1..].split_once(';') {
                        todo!("TODO: Resume at {}", addr);
                    }
                    assert!(self.thread_for_run > 0);
                    let target_tid = self.thread_for_run.try_into().unwrap();
                    for (tid, thread) in env.threads.iter_mut().enumerate() {
                        if !thread.is_alive() {
                            continue;
                        } else if tid == target_tid {
                            thread.state = ThreadState::Stepping;
                        } else {
                            // TODO: Is this actually the correct behaviour?
                            // It's probably not a big deal, since any
                            // reasonably new version of gdb will use vCont.
                            thread.state = ThreadState::Paused;
                        }
                    }

                    break;
                }
                // New style run/continue command.
                b'v' => {
                    if p == "vCont?" {
                        self.send_packet("vCont;c;s;C;S")
                    } else if p.starts_with("vCont") {
                        let Some((_, commands)) = p.split_once(';') else {
                            // Bad vcont packet
                            self.send_packet("E00");
                            continue;
                        };
                        let mut states = vec![None; env.threads.len()];
                        let mut default_state = ThreadState::Paused;
                        let mut has_stepped = false;
                        for subcommand in commands.split(';') {
                            let state = match subcommand.as_bytes()[0] {
                                // Signal is just ignored for now (TODO?)
                                b'c' | b'C' => ThreadState::Running,
                                b's' | b'S' => {
                                    // It doesn't _really_ make sense for more
                                    // than one thread to be stepping at any
                                    // given time, although maybe gdb allows it?
                                    assert!(!has_stepped);
                                    has_stepped = true;
                                    ThreadState::Stepping
                                }
                                _ => {
                                    // Bad packet
                                    self.send_packet("E00");
                                    continue 'packet_loop;
                                }
                            };
                            match subcommand.split_once(':') {
                                Some((_, tid_str)) => {
                                    let tid: isize = tid_str.parse().unwrap();
                                    if tid == -1 {
                                        default_state = state;
                                    } else {
                                        let tid: usize = tid.try_into().unwrap();
                                        let tid = tid.saturating_sub(1);
                                        states[tid] = Some(state);
                                    }
                                }
                                None => default_state = state,
                            }
                        }
                        for (thread, curr_state) in env.threads.iter_mut().zip(states) {
                            if !thread.is_alive() {
                                continue;
                            }
                            match curr_state {
                                Some(next_state) => thread.state = next_state,
                                None => thread.state = default_state,
                            }
                        }
                        break;
                    } else {
                        log_dbg!("Unhandled packet.");
                        self.send_packet("");
                    }
                }
                // Checks if thread is still alive
                b'T' => {
                    let tid: usize = p.split_at(1).1.parse().unwrap();
                    if env.threads[tid - 1].is_alive() {
                        self.send_packet("OK");
                    } else {
                        self.send_packet("");
                    }
                }
                // Specifies thread commands should run on.
                b'H' => {
                    let command = p.as_bytes()[1];
                    let tid: isize = p.split_at(2).1.parse().unwrap();
                    if command == b'c' {
                        self.thread_for_run = tid;
                    } else if command == b'g' {
                        self.thread_for_other = tid;
                    } else {
                        // Unsupported command type
                        self.send_packet("E00");
                    }
                    self.send_packet("OK");
                }
                // Kill
                b'k' => {
                    panic!("Debugger requested kill.");
                }
                b'q' => {
                    if p == "qC" {
                        let tid = env.current_thread + 1;
                        self.send_packet(format!("QC{tid:x}").as_str());
                    } else if p == "qfThreadInfo" {
                        // First command to get threads, just send whole
                        // list over now.
                        let mut live_threads = Vec::new();
                        for (tid, thread) in env.threads.iter().enumerate() {
                            if thread.is_alive() {
                                live_threads.push(tid + 1);
                            }
                        }

                        let mut packet = "m".to_string();
                        for tid in live_threads[..live_threads.len() - 1].iter() {
                            write!(packet, "{tid:x},").unwrap();
                        }
                        write!(packet, "{:x}", live_threads[live_threads.len() - 1]).unwrap();
                        self.send_packet(packet.as_str());
                    } else if p == "qsThreadInfo" {
                        // Second command to get threads, end the list.
                        self.send_packet("l");
                    } else if p.starts_with("qThreadExtraInfo") {
                        let (_, tid_str) = p.split_once(',').unwrap();
                        let tid: usize = tid_str.parse().unwrap();
                        let tid = tid.saturating_sub(1);
                        let thread_block = format!("{}", env.threads[tid].blocked_by);
                        let mut thread_block_hex = String::new();
                        thread_block
                            .bytes()
                            .for_each(|b| write!(thread_block_hex, "{b:02x}").unwrap());
                        self.send_packet(thread_block_hex.as_str());
                    } else if p == "qAttached" {
                        // Query whether we're attaching to an existing or new
                        // process (always sends new process)
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
                        self.send_packet("");
                    }
                }
                _ => {
                    log_dbg!("Unhandled packet.");
                    // Tell GDB we don't understand this packet.
                    // In some cases this causes convenient fallbacks:
                    // Since we don't support 'Z', GDB will implement
                    // software breakpoints for us with trap instructions.
                    self.send_packet("");
                }
            }
        }
    }

    /// Returns true if a break event was sent.
    pub fn break_was_sent(&mut self) -> bool {
        if self.first_char_ne_break {
            return false;
        }

        if !self.reader_is_nonblocking {
            self.reader.get_mut().set_nonblocking(true).unwrap();
            self.reader_is_nonblocking = true;
        }

        let buf = match self.reader.fill_buf() {
            Ok(buf) => buf,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => self.reader.buffer(),
            _ => unimplemented!(),
        };
        let Some(c) = buf.first() else {
            return false;
        };

        // Not a break signal, return now.
        if *c != 0x03 {
            self.first_char_ne_break = true;
            false
        } else {
            self.reader.consume(1);
            true
        }
    }
}
