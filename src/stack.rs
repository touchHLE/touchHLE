/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Utilities related to the stack.

use crate::cpu::Cpu;
use crate::mem::{GuestUSize, Mem, MutPtr, Ptr};

/// Set up the stack for the main thread, ready to execute the entry-point
/// of the application (aka `start`).
///
/// The entry-point of an iPhone OS application expects certain data to have
/// been placed on the stack, which it will then pass as arguments to the C
/// `main` function. On iPhone OS, `main` can have up to four arguments:
///
/// ```c
/// int main(int argc, char *argv[], char *envp[], char *apple[]);
/// ```
///
/// There are two ways an executable's entry point receives these:
///
/// * Old-style `LC_UNIXTHREAD`/`LC_THREAD` binaries (iPhone OS 2.x–3.0) jump
///   to a `start` routine that reads `argc`/`argv`/… off the stack and then
///   calls `main` itself.
/// * `LC_MAIN` binaries (iPhone OS 3.1 and later, including the 32-bit App
///   Store builds touchHLE also targets) have their entry point be the C
///   `main` function directly; dyld calls it with `argc`, `argv`, `envp` and
///   `apple` already loaded into `r0`–`r3` per the AAPCS calling convention.
///
/// We always lay the data out on the stack (harmless for `LC_MAIN`), and when
/// `pass_args_in_registers` is set we *also* populate `r0`–`r3` so that an
/// `LC_MAIN` `main` sees its arguments. Without this, `main` would read
/// whatever garbage was left in those registers as `argc`/`argv`, which
/// typically leads to walking an invalid `argv` and crashing very early in
/// startup (e.g. BioShock aborts inside its allocator while copying the
/// command line).
pub fn prep_stack_for_start(
    mem: &mut Mem,
    cpu: &mut Cpu,
    argv: &[&str],
    envp: &[&str],
    apple: &[&str],
    pass_args_in_registers: bool,
) {
    let argc: i32 = argv.len().try_into().unwrap();

    // We are arbitrarily putting the main thread's stack at the top of the
    // address space (see also: mem::Mem::MAIN_THREAD_STACK_LOW_END).
    // Since the stack grows downwards, its first byte would be 0xffffffff.
    let stack_base: usize = 1 << 32;

    // Rust vectors grow upwards but we need to grow this one downwards, so
    // let's push the strings onto it reversed.
    let mut reversed_data = Vec::<u8>::new();
    let mut string_ptrs = Vec::<u32>::new();

    fn push_string(
        string: &str,
        reversed_data: &mut Vec<u8>,
        string_ptrs: &mut Vec<u32>,
        stack_base: usize,
    ) {
        reversed_data.reserve(string.len() + 1);
        reversed_data.push(b'\0'); // null terminator
        for &c in string.as_bytes().iter().rev() {
            reversed_data.push(c);
        }

        let ptr: u32 = (stack_base - reversed_data.len()).try_into().unwrap();
        string_ptrs.push(ptr);
    }

    string_ptrs.push(argc as u32);
    for arg in argv {
        push_string(arg, &mut reversed_data, &mut string_ptrs, stack_base);
    }

    for arg in envp {
        push_string(arg, &mut reversed_data, &mut string_ptrs, stack_base);
    }
    string_ptrs.push(0); // terminator

    for arg in apple {
        push_string(arg, &mut reversed_data, &mut string_ptrs, stack_base);
    }
    string_ptrs.push(0); // terminator

    // Pad to ensure stack is 4-byte aligned
    let misaligned_by = reversed_data.len() % 4;
    let pad_by = if misaligned_by != 0 {
        4 - misaligned_by
    } else {
        0
    };
    reversed_data.resize(reversed_data.len() + pad_by, 0);

    for ptr in string_ptrs.iter().rev() {
        reversed_data.extend_from_slice(ptr.to_be_bytes().as_slice());
    }

    let stack_ptr: MutPtr<u8> =
        Ptr::from_bits((stack_base - reversed_data.len()).try_into().unwrap());
    let stack_height: GuestUSize = reversed_data.len().try_into().unwrap();

    assert!(stack_height < Mem::MAIN_THREAD_STACK_SIZE);

    let stack_region = mem.bytes_at_mut(stack_ptr, stack_height);

    for i in 0..stack_height {
        stack_region[i as usize] = reversed_data[(stack_height - i - 1) as usize];
    }

    //println!(
    //  "{}",
    //  std::str::from_utf8(
    //      &stack_region
    //      .iter()
    //      .flat_map(|&c| std::ascii::escape_default(c))
    //      .collect::<Vec<u8>>())
    //  .unwrap()
    //);
    //println!(
    //    "{:?}",
    //    mem.cstr_at_utf8(MutPtr::from_bits(mem.read((stack_ptr + 4).cast())))
    //);

    assert!(stack_height.is_multiple_of(4)); // ensure padding worked properly

    let sp = stack_ptr.to_bits();
    cpu.regs_mut()[Cpu::SP] = sp;

    if pass_args_in_registers {
        // The on-stack layout, from `sp` upwards, is exactly `string_ptrs` in
        // order: `[argc][argv[0]…argv[argc-1]][NULL][envp…][NULL][apple…][NULL]`.
        // dyld passes `main` these as `main(argc, argv, envp, apple)`, where
        // `argv`/`envp`/`apple` are pointers to the respective sub-arrays.
        let argv_ptr = sp + 4; // skip argc
        let envp_ptr = argv_ptr + 4 * (argv.len() as u32 + 1); // skip argv + NULL
        let apple_ptr = envp_ptr + 4 * (envp.len() as u32 + 1); // skip envp + NULL
        let regs = cpu.regs_mut();
        regs[0] = argc as u32;
        regs[1] = argv_ptr;
        regs[2] = envp_ptr;
        regs[3] = apple_ptr;
    }
}
