/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `<execinfo.h>` (BSD backtrace API, shipped by libSystem.B.dylib).
//!
//! Apple's headers declare:
//! ```c
//! int   backtrace(void** array, int size);
//! char**backtrace_symbols(void* const* array, int size);
//! void  backtrace_symbols_fd(void* const* array, int size, int fd);
//! ```
//!
//! Reference:
//! <https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/backtrace.3.html>
//!
//! On iOS, frame pointer is conventionally `r7` for both ARM and Thumb
//! state (per the Apple "iPhone OS ABI Reference"). Each call frame
//! stored by the prologue follows the same layout:
//!
//! ```text
//!     [r7 +  0] = saved r7  (previous frame pointer)
//!     [r7 +  4] = saved lr  (return address)
//! ```
//!
//! We walk that linked list and copy the return addresses into the
//! caller's buffer, exactly like `libsystem_c.dylib`.

use crate::cpu::Cpu;
use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::mem::{ConstPtr, MutPtr, Ptr};
use crate::Environment;

/// `int backtrace(void** array, int size)`.
fn backtrace(env: &mut Environment, array: MutPtr<u32>, size: i32) -> i32 {
    if size <= 0 || array.is_null() {
        return 0;
    }
    let size = size as u32;

    let mut count: u32 = 0;
    let pc = env.cpu.regs()[Cpu::PC];
    let lr = env.cpu.regs()[Cpu::LR];
    let mut fp = env.cpu.regs()[7];

    // Frame 0: the current PC, matching the man-page wording "the
    // calling function". `lr` is the next return address.
    if count < size {
        env.mem.write(array + count, pc);
        count += 1;
    }
    if count < size && lr != 0 {
        env.mem.write(array + count, lr);
        count += 1;
    }

    // Walk the frame-pointer chain. A frame is valid as long as `fp`
    // is non-NULL, 4-byte aligned, and strictly greater than the
    // previous frame (stack grows downward, so successive `fp` values
    // are larger).
    let mut prev_fp: u32 = 0;
    while count < size {
        if fp == 0 || fp & 3 != 0 || fp <= prev_fp {
            break;
        }
        let saved_fp_ptr: ConstPtr<u32> = Ptr::from_bits(fp);
        let saved_lr_ptr: ConstPtr<u32> = Ptr::from_bits(fp + 4);
        let saved_fp = env.mem.read(saved_fp_ptr);
        let saved_lr = env.mem.read(saved_lr_ptr);
        if saved_lr == 0 {
            break;
        }
        env.mem.write(array + count, saved_lr);
        count += 1;
        prev_fp = fp;
        fp = saved_fp;
    }

    count as i32
}

/// `char **backtrace_symbols(void *const *array, int size)`.
///
/// Apple's `libsystem_c.dylib` returns a heap-allocated `char**` whose
/// caller is expected to `free()` (Apple's docs explicitly say so). We
/// honour that contract: a single allocation big enough for the array
/// of `char*` plus the symbol strings themselves. Each entry is
/// formatted as `"N   ??? 0x%08x"`, matching what Apple emits when no
/// dSYM is loaded.
fn backtrace_symbols(env: &mut Environment, array: ConstPtr<u32>, size: i32) -> MutPtr<u32> {
    if size <= 0 || array.is_null() {
        return MutPtr::null();
    }
    let size = size as u32;

    // Build the symbol strings up front so we know how much memory to
    // allocate.
    let mut strings: Vec<String> = Vec::with_capacity(size as usize);
    let mut total_string_bytes: u32 = 0;
    for i in 0..size {
        let addr = env.mem.read(array + i);
        let s = format!("{:<3}  ???  0x{:08x}", i, addr);
        // +1 for the trailing NUL byte.
        total_string_bytes += s.len() as u32 + 1;
        strings.push(s);
    }

    let table_bytes = size * 4;
    let total = table_bytes + total_string_bytes;
    let buffer: MutPtr<u8> = env.mem.alloc(total).cast();

    // Layout: [char*; size] [string bytes ...]
    let mut string_cursor: MutPtr<u8> = buffer + table_bytes;
    for (i, s) in strings.iter().enumerate() {
        let entry_ptr: MutPtr<u32> = (buffer + (i as u32) * 4).cast();
        env.mem.write(entry_ptr, string_cursor.to_bits());
        let len = s.len() as u32;
        env.mem
            .bytes_at_mut(string_cursor, len)
            .copy_from_slice(s.as_bytes());
        env.mem.write(string_cursor + len, 0u8);
        string_cursor += len + 1;
    }
    buffer.cast()
}

/// `void backtrace_symbols_fd(void *const *array, int size, int fd)`.
fn backtrace_symbols_fd(env: &mut Environment, array: ConstPtr<u32>, size: i32, fd: i32) {
    if size <= 0 || array.is_null() {
        return;
    }
    for i in 0..size as u32 {
        let addr = env.mem.read(array + i);
        let line = format!("{:<3}  ???  0x{:08x}\n", i, addr);
        // Hand off to write(2). Failing writes are silently dropped,
        // matching libsystem_c's behaviour.
        let bytes: MutPtr<u8> = env.mem.alloc(line.len() as u32).cast();
        env.mem.bytes_at_mut(bytes, line.len() as u32)
            .copy_from_slice(line.as_bytes());
        let _ = crate::libc::posix_io::write(env, fd, bytes.cast().cast_const(), line.len() as u32);
        env.mem.free(bytes.cast());
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(backtrace(_, _)),
    export_c_func!(backtrace_symbols(_, _)),
    export_c_func!(backtrace_symbols_fd(_, _, _)),
];
