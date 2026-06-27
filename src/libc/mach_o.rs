/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `Mach-O` related functions.

use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::string::strcmp;
use crate::mem::{ConstPtr, GuestUSize, MutPtr, Ptr, SafeRead};
use crate::Environment;
use std::collections::HashMap;

// --- ДОБАВЛЯЕМ СТРУКТУРЫ И КОНСТАНТЫ ДЛЯ host_info ---
const HOST_BASIC_INFO: i32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct host_basic_info {
    max_cpus: i32,
    avail_cpus: i32,
    memory_size: u32,
    cpu_type: i32,
    cpu_subtype: i32,
    cpu_threadtype: i32,
    physical_cpu: i32,
    physical_cpu_max: i32,
    logical_cpu: i32,
    logical_cpu_max: i32,
    max_mem: u64,
}
unsafe impl SafeRead for host_basic_info {}

fn host_info(
    env: &mut Environment,
    _host: i32,
    flavor: i32,
    info_out: MutPtr<i32>,
    info_cnt: MutPtr<u32>,
) -> i32 {
    match flavor {
        HOST_BASIC_INFO => {
            let mut info = host_basic_info::default();

            // Эмулируем железо iPad/iPhone
            info.max_cpus = 1;
            info.avail_cpus = 1;
            info.physical_cpu = 1;
            info.logical_cpu = 1;

            // CPU_TYPE_ARM = 12, CPU_SUBTYPE_ARM_V7 = 9
            info.cpu_type = 12;
            info.cpu_subtype = 9;

            // 256MB RAM (безопасное значение для старых игр)
            let mem_bytes = 256 * 1024 * 1024;
            info.memory_size = mem_bytes as u32;
            info.max_mem = mem_bytes as u64;

            let struct_size = (std::mem::size_of::<host_basic_info>() / 4) as u32;
            let provided_cnt = env.mem.read(info_cnt);

            if provided_cnt < struct_size {
                return 4; // KERN_INVALID_ARGUMENT
            }

            env.mem.write(info_out.cast(), info);
            env.mem.write(info_cnt, struct_size);

            0 // KERN_SUCCESS
        }
        _ => {
            // Если запрашивают другой flavor, просто рапортуем об успехе без
            // паники
            0
        }
    }
}
// -----------------------------------------------------

fn get_end(env: &mut Environment) -> u32 {
    // Assume app binary is the first.
    // From https://www.manpagez.com/man/3/get_end/
    // `In a Mach-O file <...> get_end returns the first address after
    // the last segment in the executable`
    // It was confirmed on a real device with the TestApp binary.
    env.bins[0].last_segment_end
}

fn get_etext(env: &mut Environment) -> u32 {
    // Assume app binary is the first.
    let app_sections = &env.bins[0].sections;
    assert_eq!(
        app_sections
            .iter()
            .filter(|s| s.name.to_uppercase() == "__TEXT")
            .count(),
        1
    );
    let text_section = app_sections
        .iter()
        .find(|s| s.name.to_uppercase() == "__TEXT")
        .unwrap();
    text_section.next_section_addr()
}

// --- dyld API functions ---

/// `uint32_t _dyld_image_count(void)` — returns the number of images
/// (Mach-O binaries) currently loaded in the process address space.
/// See: https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/dyld.3.html
fn _dyld_image_count(env: &mut Environment) -> u32 {
    env.bins.len() as u32
}

/// `const struct mach_header* _dyld_get_image_header(uint32_t image_index)`
/// Returns the mach_header pointer for the image at `image_index`.
/// The mach_header is located at the start of the __TEXT segment.
fn _dyld_get_image_header(env: &mut Environment, image_index: u32) -> u32 {
    let idx = image_index as usize;
    if idx >= env.bins.len() {
        log!(
            "Warning: _dyld_get_image_header({}) out of range (only {} images loaded), returning 0",
            image_index,
            env.bins.len()
        );
        return 0;
    }
    env.bins[idx].text_base
}

/// `const char* _dyld_get_image_name(uint32_t image_index)`
/// Returns a C-string pointer with the path of the image at `image_index`.
/// We allocate the string in guest memory the first time it's requested.
fn _dyld_get_image_name(env: &mut Environment, image_index: u32) -> ConstPtr<u8> {
    let idx = image_index as usize;
    if idx >= env.bins.len() {
        log!(
            "Warning: _dyld_get_image_name({}) out of range, returning NULL",
            image_index
        );
        return Ptr::null();
    }
    let name = env.bins[idx].name.clone();
    let len = name.len() as u32 + 1;
    let ptr: MutPtr<u8> = env.mem.alloc(len).cast();
    let dst = env.mem.bytes_at_mut(ptr, len);
    dst[..name.len()].copy_from_slice(name.as_bytes());
    dst[name.len()] = 0;
    ptr.cast_const()
}

/// `intptr_t _dyld_get_image_vmaddr_slide(uint32_t image_index)`
/// Returns the virtual memory address slide for the image. Since touchHLE
/// loads binaries at their preferred addresses (slide=0 for the main app),
/// we return 0. Dylibs might have a slide but in practice iOS games don't
/// query this for anything critical.
fn _dyld_get_image_vmaddr_slide(env: &mut Environment, image_index: u32) -> u32 {
    let idx = image_index as usize;
    if idx >= env.bins.len() {
        return 0;
    }
    // For the main binary (index 0) the slide is always 0 in touchHLE.
    // For dylibs we don't track the slide separately, return 0.
    0
}

/// `void _dyld_register_func_for_add_image(void (*func)(const struct mach_header *mh, intptr_t vmaddr_slide))`
///
/// Per Apple's [dyld(3) manpage](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/dyld.3.html):
///
/// > The `_dyld_register_func_for_add_image` function registers the
/// > specified function to be called when a new image is added (a
/// > bundle or a dynamic shared library) to the program. When this
/// > function is first called, it is called once for each image that
/// > is currently part of the program.
///
/// touchHLE finishes loading every binary and dylib before guest code
/// starts executing, and we do not support dynamic image load / unload
/// at runtime, so the documented contract collapses to: "invoke the
/// callback exactly once for every already-loaded image". We honour
/// that here. Each call passes the image's mach_header pointer and a
/// `vmaddr_slide` of 0 (touchHLE loads binaries at their preferred
/// addresses).
fn _dyld_register_func_for_add_image(env: &mut Environment, func: GuestFunction) {
    if func.to_ptr().is_null() {
        log!("Warning: _dyld_register_func_for_add_image(NULL) — ignored");
        return;
    }
    log_dbg!(
        "_dyld_register_func_for_add_image({:?}): invoking for {} already-loaded image(s).",
        func,
        env.bins.len()
    );
    // Snapshot the per-image data first because the callback runs guest
    // code and `env.bins` may be borrowed during the call.
    let images: Vec<u32> = env.bins.iter().map(|b| b.text_base).collect();
    for mh in images {
        // vmaddr_slide = 0 for the main binary, and we don't track per-dylib
        // slides separately. This matches what `_dyld_get_image_vmaddr_slide`
        // already returns and keeps the two APIs consistent.
        let _: () = func.call_from_host(env, (mh, 0u32));
    }
}

/// `void _dyld_register_func_for_remove_image(void (*func)(const struct mach_header *mh, intptr_t vmaddr_slide))`
///
/// Per Apple's [dyld(3) manpage](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/dyld.3.html):
///
/// > Functions registered with `_dyld_register_func_for_remove_image()`
/// > are called after any terminators in an image are run and before
/// > the image is un-memory-mapped.
///
/// touchHLE never unloads images — `dlclose` is a no-op in our runtime
/// — so the callback would never fire. We accept the registration so
/// that calling code does not treat the call as a failure, but the
/// callback is intentionally never invoked.
fn _dyld_register_func_for_remove_image(_env: &mut Environment, func: GuestFunction) {
    log_dbg!(
        "_dyld_register_func_for_remove_image({:?}): accepted; image removal never fires in touchHLE",
        func
    );
}

// MARK: - NX architecture info (mach-o/arch.h)
//
// `NXArchInfo` is the C-level descriptor used by `NXGetArchInfoFromCpuType`,
// `NXGetArchInfoFromName`, `NXGetLocalArchInfo`, and `NXGetAllArchInfos`. The
// table here mirrors Apple's `/usr/share/file/magic`-style listing — every
// CPU type / subtype pair iOS apps are likely to query (ARM family, i386,
// x86_64, ARM64, PowerPC) has an entry, and unknown subtypes fall back to
// the architecture's `ALL` row.

/// `NXByteOrder` — value reported in `NXArchInfo::byteorder`.
const NX_UNKNOWN_BYTE_ORDER: u32 = 0;
const NX_LITTLE_ENDIAN: u32 = 1;
const NX_BIG_ENDIAN: u32 = 2;

// `cpu_type_t` constants are 32-bit signed (the high bit is set for 64-bit
// variants on real hardware, e.g. `CPU_ARCH_ABI64`). Values from
// `<mach/machine.h>`.
const CPU_ARCH_ABI64: i32 = 0x0100_0000_u32 as i32;
const CPU_TYPE_ANY: i32 = -1;
const CPU_TYPE_VAX: i32 = 1;
const CPU_TYPE_MC680X0: i32 = 6;
const CPU_TYPE_X86: i32 = 7;
const CPU_TYPE_I386: i32 = CPU_TYPE_X86;
const CPU_TYPE_X86_64: i32 = CPU_TYPE_X86 | CPU_ARCH_ABI64;
const CPU_TYPE_HPPA: i32 = 11;
const CPU_TYPE_ARM: i32 = 12;
const CPU_TYPE_ARM64: i32 = CPU_TYPE_ARM | CPU_ARCH_ABI64;
const CPU_TYPE_MC88000: i32 = 13;
const CPU_TYPE_SPARC: i32 = 14;
const CPU_TYPE_POWERPC: i32 = 18;
const CPU_TYPE_POWERPC64: i32 = CPU_TYPE_POWERPC | CPU_ARCH_ABI64;

// `cpu_subtype_t` values used by the entries below. We only define the
// constants we actually reference.
const CPU_SUBTYPE_MULTIPLE: i32 = -1;
const CPU_SUBTYPE_LITTLE_ENDIAN: i32 = 0;
const CPU_SUBTYPE_BIG_ENDIAN: i32 = 1;

const CPU_SUBTYPE_X86_ALL: i32 = 3;
const CPU_SUBTYPE_X86_64_ALL: i32 = 3;
const CPU_SUBTYPE_X86_64_H: i32 = 8;
const CPU_SUBTYPE_I386_486: i32 = 4;
const CPU_SUBTYPE_I486_SX: i32 = 4 + 128;
const CPU_SUBTYPE_PENT: i32 = 5;
const CPU_SUBTYPE_PENTPRO: i32 = 22;
const CPU_SUBTYPE_PENTII_M3: i32 = 54;
const CPU_SUBTYPE_PENTII_M5: i32 = 86;

const CPU_SUBTYPE_ARM_ALL: i32 = 0;
const CPU_SUBTYPE_ARM_V4T: i32 = 5;
const CPU_SUBTYPE_ARM_V6: i32 = 6;
const CPU_SUBTYPE_ARM_V5TEJ: i32 = 7;
const CPU_SUBTYPE_ARM_XSCALE: i32 = 8;
const CPU_SUBTYPE_ARM_V7: i32 = 9;
const CPU_SUBTYPE_ARM_V7F: i32 = 10;
const CPU_SUBTYPE_ARM_V7S: i32 = 11;
const CPU_SUBTYPE_ARM_V7K: i32 = 12;
const CPU_SUBTYPE_ARM_V6M: i32 = 14;
const CPU_SUBTYPE_ARM_V7M: i32 = 15;
const CPU_SUBTYPE_ARM_V7EM: i32 = 16;
const CPU_SUBTYPE_ARM_V8: i32 = 13;

const CPU_SUBTYPE_ARM64_ALL: i32 = 0;
const CPU_SUBTYPE_ARM64_V8: i32 = 1;
const CPU_SUBTYPE_ARM64E: i32 = 2;

const CPU_SUBTYPE_POWERPC_ALL: i32 = 0;
const CPU_SUBTYPE_POWERPC_601: i32 = 1;
const CPU_SUBTYPE_POWERPC_602: i32 = 2;
const CPU_SUBTYPE_POWERPC_603: i32 = 3;
const CPU_SUBTYPE_POWERPC_603E: i32 = 4;
const CPU_SUBTYPE_POWERPC_603EV: i32 = 5;
const CPU_SUBTYPE_POWERPC_604: i32 = 6;
const CPU_SUBTYPE_POWERPC_604E: i32 = 7;
const CPU_SUBTYPE_POWERPC_750: i32 = 9;
const CPU_SUBTYPE_POWERPC_7400: i32 = 10;
const CPU_SUBTYPE_POWERPC_7450: i32 = 11;
const CPU_SUBTYPE_POWERPC_970: i32 = 100;

/// In-process layout of `NXArchInfo`. All pointer fields are 32-bit guest
/// pointers and `byteorder` is an `int` (4 bytes), so the struct is exactly
/// 20 bytes wide on 32-bit iOS.
#[repr(C, packed)]
#[derive(Copy, Clone)]
struct NXArchInfoGuest {
    name: ConstPtr<u8>,
    cputype: i32,
    cpusubtype: i32,
    byteorder: u32,
    description: ConstPtr<u8>,
}
unsafe impl SafeRead for NXArchInfoGuest {}

/// Compile-time descriptor for an `NXArchInfo` entry, prior to allocation
/// in guest memory.
struct NXArchInfoEntry {
    name: &'static str,
    cputype: i32,
    cpusubtype: i32,
    byteorder: u32,
    description: &'static str,
}

/// Table of every architecture iOS apps are likely to query.
///
/// Order matches Apple's `/usr/share/man/man3/NXGetAllArchInfos.3` listing:
/// the "ALL" / "generic" entry for an architecture must come first so that
/// `NXGetArchInfoFromCpuType(cputype, CPU_SUBTYPE_ANY)` and lookups for an
/// unknown subtype fall back to the right canonical entry.
const NX_ARCH_INFOS: &[NXArchInfoEntry] = &[
    // "Any" / hppa / m88k entries from Apple's table.
    NXArchInfoEntry {
        name: "hppa",
        cputype: CPU_TYPE_HPPA,
        cpusubtype: 0,
        byteorder: NX_BIG_ENDIAN,
        description: "HP-PA",
    },
    NXArchInfoEntry {
        name: "sparc",
        cputype: CPU_TYPE_SPARC,
        cpusubtype: 0,
        byteorder: NX_BIG_ENDIAN,
        description: "SPARC",
    },
    NXArchInfoEntry {
        name: "m68k",
        cputype: CPU_TYPE_MC680X0,
        cpusubtype: 1,
        byteorder: NX_BIG_ENDIAN,
        description: "Motorola 68K",
    },
    NXArchInfoEntry {
        name: "m88k",
        cputype: CPU_TYPE_MC88000,
        cpusubtype: 0,
        byteorder: NX_BIG_ENDIAN,
        description: "Motorola 88K",
    },
    // i386 family.
    NXArchInfoEntry {
        name: "i386",
        cputype: CPU_TYPE_I386,
        cpusubtype: CPU_SUBTYPE_X86_ALL,
        byteorder: NX_LITTLE_ENDIAN,
        description: "Intel 80x86",
    },
    NXArchInfoEntry {
        name: "i486",
        cputype: CPU_TYPE_I386,
        cpusubtype: CPU_SUBTYPE_I386_486,
        byteorder: NX_LITTLE_ENDIAN,
        description: "Intel 80486",
    },
    NXArchInfoEntry {
        name: "i486SX",
        cputype: CPU_TYPE_I386,
        cpusubtype: CPU_SUBTYPE_I486_SX,
        byteorder: NX_LITTLE_ENDIAN,
        description: "Intel 80486SX",
    },
    NXArchInfoEntry {
        name: "pentium",
        cputype: CPU_TYPE_I386,
        cpusubtype: CPU_SUBTYPE_PENT,
        byteorder: NX_LITTLE_ENDIAN,
        description: "Intel Pentium",
    },
    NXArchInfoEntry {
        name: "pentpro",
        cputype: CPU_TYPE_I386,
        cpusubtype: CPU_SUBTYPE_PENTPRO,
        byteorder: NX_LITTLE_ENDIAN,
        description: "Intel Pentium Pro",
    },
    NXArchInfoEntry {
        name: "pentIIm3",
        cputype: CPU_TYPE_I386,
        cpusubtype: CPU_SUBTYPE_PENTII_M3,
        byteorder: NX_LITTLE_ENDIAN,
        description: "Intel Pentium II (Model 3)",
    },
    NXArchInfoEntry {
        name: "pentIIm5",
        cputype: CPU_TYPE_I386,
        cpusubtype: CPU_SUBTYPE_PENTII_M5,
        byteorder: NX_LITTLE_ENDIAN,
        description: "Intel Pentium II (Model 5)",
    },
    // x86_64 family.
    NXArchInfoEntry {
        name: "x86_64",
        cputype: CPU_TYPE_X86_64,
        cpusubtype: CPU_SUBTYPE_X86_64_ALL,
        byteorder: NX_LITTLE_ENDIAN,
        description: "Intel x86-64",
    },
    NXArchInfoEntry {
        name: "x86_64h",
        cputype: CPU_TYPE_X86_64,
        cpusubtype: CPU_SUBTYPE_X86_64_H,
        byteorder: NX_LITTLE_ENDIAN,
        description: "Intel x86-64h Haswell",
    },
    // PowerPC family (kept for old binaries / tools that still query it).
    NXArchInfoEntry {
        name: "ppc",
        cputype: CPU_TYPE_POWERPC,
        cpusubtype: CPU_SUBTYPE_POWERPC_ALL,
        byteorder: NX_BIG_ENDIAN,
        description: "PowerPC",
    },
    NXArchInfoEntry {
        name: "ppc601",
        cputype: CPU_TYPE_POWERPC,
        cpusubtype: CPU_SUBTYPE_POWERPC_601,
        byteorder: NX_BIG_ENDIAN,
        description: "PowerPC 601",
    },
    NXArchInfoEntry {
        name: "ppc602",
        cputype: CPU_TYPE_POWERPC,
        cpusubtype: CPU_SUBTYPE_POWERPC_602,
        byteorder: NX_BIG_ENDIAN,
        description: "PowerPC 602",
    },
    NXArchInfoEntry {
        name: "ppc603",
        cputype: CPU_TYPE_POWERPC,
        cpusubtype: CPU_SUBTYPE_POWERPC_603,
        byteorder: NX_BIG_ENDIAN,
        description: "PowerPC 603",
    },
    NXArchInfoEntry {
        name: "ppc603e",
        cputype: CPU_TYPE_POWERPC,
        cpusubtype: CPU_SUBTYPE_POWERPC_603E,
        byteorder: NX_BIG_ENDIAN,
        description: "PowerPC 603e",
    },
    NXArchInfoEntry {
        name: "ppc603ev",
        cputype: CPU_TYPE_POWERPC,
        cpusubtype: CPU_SUBTYPE_POWERPC_603EV,
        byteorder: NX_BIG_ENDIAN,
        description: "PowerPC 603ev",
    },
    NXArchInfoEntry {
        name: "ppc604",
        cputype: CPU_TYPE_POWERPC,
        cpusubtype: CPU_SUBTYPE_POWERPC_604,
        byteorder: NX_BIG_ENDIAN,
        description: "PowerPC 604",
    },
    NXArchInfoEntry {
        name: "ppc604e",
        cputype: CPU_TYPE_POWERPC,
        cpusubtype: CPU_SUBTYPE_POWERPC_604E,
        byteorder: NX_BIG_ENDIAN,
        description: "PowerPC 604e",
    },
    NXArchInfoEntry {
        name: "ppc750",
        cputype: CPU_TYPE_POWERPC,
        cpusubtype: CPU_SUBTYPE_POWERPC_750,
        byteorder: NX_BIG_ENDIAN,
        description: "PowerPC 750",
    },
    NXArchInfoEntry {
        name: "ppc7400",
        cputype: CPU_TYPE_POWERPC,
        cpusubtype: CPU_SUBTYPE_POWERPC_7400,
        byteorder: NX_BIG_ENDIAN,
        description: "PowerPC 7400",
    },
    NXArchInfoEntry {
        name: "ppc7450",
        cputype: CPU_TYPE_POWERPC,
        cpusubtype: CPU_SUBTYPE_POWERPC_7450,
        byteorder: NX_BIG_ENDIAN,
        description: "PowerPC 7450",
    },
    NXArchInfoEntry {
        name: "ppc970",
        cputype: CPU_TYPE_POWERPC,
        cpusubtype: CPU_SUBTYPE_POWERPC_970,
        byteorder: NX_BIG_ENDIAN,
        description: "PowerPC 970",
    },
    NXArchInfoEntry {
        name: "ppc64",
        cputype: CPU_TYPE_POWERPC64,
        cpusubtype: CPU_SUBTYPE_POWERPC_ALL,
        byteorder: NX_BIG_ENDIAN,
        description: "PowerPC 64-bit",
    },
    NXArchInfoEntry {
        name: "ppc970-64",
        cputype: CPU_TYPE_POWERPC64,
        cpusubtype: CPU_SUBTYPE_POWERPC_970,
        byteorder: NX_BIG_ENDIAN,
        description: "PowerPC 970 64-bit",
    },
    // ARM family — the one touchHLE actually emulates.
    NXArchInfoEntry {
        name: "arm",
        cputype: CPU_TYPE_ARM,
        cpusubtype: CPU_SUBTYPE_ARM_ALL,
        byteorder: NX_LITTLE_ENDIAN,
        description: "ARM",
    },
    NXArchInfoEntry {
        name: "armv4t",
        cputype: CPU_TYPE_ARM,
        cpusubtype: CPU_SUBTYPE_ARM_V4T,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm v4t",
    },
    NXArchInfoEntry {
        name: "armv5",
        cputype: CPU_TYPE_ARM,
        cpusubtype: CPU_SUBTYPE_ARM_V5TEJ,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm v5",
    },
    NXArchInfoEntry {
        name: "xscale",
        cputype: CPU_TYPE_ARM,
        cpusubtype: CPU_SUBTYPE_ARM_XSCALE,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm xscale",
    },
    NXArchInfoEntry {
        name: "armv6",
        cputype: CPU_TYPE_ARM,
        cpusubtype: CPU_SUBTYPE_ARM_V6,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm v6",
    },
    NXArchInfoEntry {
        name: "armv6m",
        cputype: CPU_TYPE_ARM,
        cpusubtype: CPU_SUBTYPE_ARM_V6M,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm v6m",
    },
    NXArchInfoEntry {
        name: "armv7",
        cputype: CPU_TYPE_ARM,
        cpusubtype: CPU_SUBTYPE_ARM_V7,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm v7",
    },
    NXArchInfoEntry {
        name: "armv7f",
        cputype: CPU_TYPE_ARM,
        cpusubtype: CPU_SUBTYPE_ARM_V7F,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm v7f",
    },
    NXArchInfoEntry {
        name: "armv7s",
        cputype: CPU_TYPE_ARM,
        cpusubtype: CPU_SUBTYPE_ARM_V7S,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm v7s",
    },
    NXArchInfoEntry {
        name: "armv7k",
        cputype: CPU_TYPE_ARM,
        cpusubtype: CPU_SUBTYPE_ARM_V7K,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm v7k",
    },
    NXArchInfoEntry {
        name: "armv7m",
        cputype: CPU_TYPE_ARM,
        cpusubtype: CPU_SUBTYPE_ARM_V7M,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm v7m",
    },
    NXArchInfoEntry {
        name: "armv7em",
        cputype: CPU_TYPE_ARM,
        cpusubtype: CPU_SUBTYPE_ARM_V7EM,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm v7em",
    },
    NXArchInfoEntry {
        name: "armv8",
        cputype: CPU_TYPE_ARM,
        cpusubtype: CPU_SUBTYPE_ARM_V8,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm v8",
    },
    NXArchInfoEntry {
        name: "arm64",
        cputype: CPU_TYPE_ARM64,
        cpusubtype: CPU_SUBTYPE_ARM64_ALL,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm64",
    },
    NXArchInfoEntry {
        name: "arm64v8",
        cputype: CPU_TYPE_ARM64,
        cpusubtype: CPU_SUBTYPE_ARM64_V8,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm64 v8",
    },
    NXArchInfoEntry {
        name: "arm64e",
        cputype: CPU_TYPE_ARM64,
        cpusubtype: CPU_SUBTYPE_ARM64E,
        byteorder: NX_LITTLE_ENDIAN,
        description: "arm64e",
    },
    // VAX / Little / Big / Multiple stand-ins from the Darwin sources.
    NXArchInfoEntry {
        name: "vax",
        cputype: CPU_TYPE_VAX,
        cpusubtype: 0,
        byteorder: NX_LITTLE_ENDIAN,
        description: "VAX",
    },
    NXArchInfoEntry {
        name: "little",
        cputype: CPU_TYPE_ANY,
        cpusubtype: CPU_SUBTYPE_LITTLE_ENDIAN,
        byteorder: NX_LITTLE_ENDIAN,
        description: "Little Endian",
    },
    NXArchInfoEntry {
        name: "big",
        cputype: CPU_TYPE_ANY,
        cpusubtype: CPU_SUBTYPE_BIG_ENDIAN,
        byteorder: NX_BIG_ENDIAN,
        description: "Big Endian",
    },
    NXArchInfoEntry {
        name: "any",
        cputype: CPU_TYPE_ANY,
        cpusubtype: CPU_SUBTYPE_MULTIPLE,
        byteorder: NX_UNKNOWN_BYTE_ORDER,
        description: "Architecture Independent",
    },
];

#[derive(Default)]
pub struct State {
    /// Cache of `(cputype, cpusubtype)` → guest pointer to an `NXArchInfoGuest`.
    ///
    /// Each entry is allocated lazily and lives for the duration of the
    /// process so callers can safely hold onto the returned `const`
    /// pointers (and string pointers reachable through them) indefinitely,
    /// matching Apple's contract that `NXGetArchInfoFromCpuType` returns a
    /// static, never-freed pointer.
    arch_infos: HashMap<(i32, i32), MutPtr<NXArchInfoGuest>>,
    /// Cache of C-string allocations for `name` / `description`. Keyed by
    /// the `&'static str` itself so a string shared between entries is only
    /// allocated in guest memory once.
    arch_strings: HashMap<&'static str, ConstPtr<u8>>,
    /// Guest pointer to the `NULL`-terminated array returned by
    /// `NXGetAllArchInfos`. Built on first use.
    arch_info_list: Option<ConstPtr<NXArchInfoGuest>>,
}

/// Allocates a guest copy of `s` and caches the pointer in [`State::arch_strings`].
fn intern_string(env: &mut Environment, s: &'static str) -> ConstPtr<u8> {
    if let Some(&cached) = env.libc_state.mach_o.arch_strings.get(s) {
        return cached;
    }
    let bytes = s.as_bytes();
    let len = bytes.len() as GuestUSize + 1;
    let ptr: MutPtr<u8> = env.mem.alloc(len).cast();
    let dst = env.mem.bytes_at_mut(ptr, len);
    dst[..bytes.len()].copy_from_slice(bytes);
    dst[bytes.len()] = 0;
    let const_ptr = ptr.cast_const();
    env.libc_state.mach_o.arch_strings.insert(s, const_ptr);
    const_ptr
}

/// Allocates a guest copy of the [`NXArchInfoEntry`] at `idx` (and caches it
/// in [`State::arch_infos`]). The returned pointer never changes for the
/// lifetime of the process.
fn intern_arch_info(
    env: &mut Environment,
    entry: &'static NXArchInfoEntry,
) -> MutPtr<NXArchInfoGuest> {
    let key = (entry.cputype, entry.cpusubtype);
    if let Some(&cached) = env.libc_state.mach_o.arch_infos.get(&key) {
        return cached;
    }
    let name = intern_string(env, entry.name);
    let description = intern_string(env, entry.description);
    let info = NXArchInfoGuest {
        name,
        cputype: entry.cputype,
        cpusubtype: entry.cpusubtype,
        byteorder: entry.byteorder,
        description,
    };
    let ptr: MutPtr<NXArchInfoGuest> = env.mem.alloc_and_write(info);
    env.libc_state.mach_o.arch_infos.insert(key, ptr);
    ptr
}

/// Picks the best [`NXArchInfoEntry`] for `(cputype, cpusubtype)`. A perfect
/// match wins; otherwise we fall back to the architecture's "ALL" entry, and
/// finally to `None` if the `cputype` itself is unknown.
fn find_arch_entry(cputype: i32, cpusubtype: i32) -> Option<&'static NXArchInfoEntry> {
    // Apple masks off the CPU_SUBTYPE_MASK feature bits (high byte) before
    // matching; mirror that so e.g. `CPU_SUBTYPE_ARM_V7 | CPU_SUBTYPE_LIB64`
    // still resolves to armv7.
    const CPU_SUBTYPE_MASK: i32 = 0x00ff_ffff;
    let masked_sub = cpusubtype & CPU_SUBTYPE_MASK;
    let mut all_for_cputype: Option<&'static NXArchInfoEntry> = None;
    for entry in NX_ARCH_INFOS {
        if entry.cputype != cputype {
            continue;
        }
        if entry.cpusubtype == masked_sub {
            return Some(entry);
        }
        // First "ALL" entry encountered for the architecture wins.
        if all_for_cputype.is_none()
            && (entry.cpusubtype == 0 // ARM_ALL / PPC_ALL / ARM64_ALL
                || entry.cpusubtype == CPU_SUBTYPE_X86_ALL)
        {
            all_for_cputype = Some(entry);
        }
    }
    all_for_cputype
}

/// `const NXArchInfo *NXGetArchInfoFromCpuType(cpu_type_t cputype, cpu_subtype_t cpusubtype);`
///
/// Returns a stable pointer to an `NXArchInfo` describing the given CPU.
/// `cpusubtype == CPU_SUBTYPE_MULTIPLE (-1)` requests the architecture's
/// "ALL" entry; unknown subtypes also fall back to that entry, which mirrors
/// Apple's behaviour. Unknown `cputype` returns `NULL`.
fn NXGetArchInfoFromCpuType(
    env: &mut Environment,
    cputype: i32,
    cpusubtype: i32,
) -> ConstPtr<NXArchInfoGuest> {
    let Some(entry) = find_arch_entry(cputype, cpusubtype) else {
        log!(
            "NXGetArchInfoFromCpuType: no entry for cputype={:#x} cpusubtype={:#x}, returning NULL",
            cputype,
            cpusubtype
        );
        return Ptr::null();
    };
    intern_arch_info(env, entry).cast_const()
}

/// `const NXArchInfo *NXGetArchInfoFromName(const char *name);`
fn NXGetArchInfoFromName(env: &mut Environment, name: ConstPtr<u8>) -> ConstPtr<NXArchInfoGuest> {
    if name.is_null() {
        return Ptr::null();
    }
    let mut best: Option<&'static NXArchInfoEntry> = None;
    for entry in NX_ARCH_INFOS {
        let candidate = intern_string(env, entry.name);
        // Compare in guest memory so that subtle encoding differences (NULs,
        // etc.) match `strcmp` semantics rather than Rust string equality.
        if strcmp(env, name, candidate) == 0 {
            best = Some(entry);
            break;
        }
    }
    match best {
        Some(entry) => intern_arch_info(env, entry).cast_const(),
        None => Ptr::null(),
    }
}

/// `const NXArchInfo *NXGetLocalArchInfo(void);`
///
/// touchHLE emulates an ARMv7 CPU, so we return the `armv7` entry.
fn NXGetLocalArchInfo(env: &mut Environment) -> ConstPtr<NXArchInfoGuest> {
    NXGetArchInfoFromCpuType(env, CPU_TYPE_ARM, CPU_SUBTYPE_ARM_V7)
}

/// `const NXArchInfo *NXGetAllArchInfos(void);`
///
/// Returns a pointer to a contiguous, NULL-terminated array of all known
/// architectures. The array is built on first use and never freed (matching
/// Apple's contract).
fn NXGetAllArchInfos(env: &mut Environment) -> ConstPtr<NXArchInfoGuest> {
    if let Some(cached) = env.libc_state.mach_o.arch_info_list {
        return cached;
    }
    // Allocate room for every entry plus a final all-zero terminator.
    let count = NX_ARCH_INFOS.len() as GuestUSize;
    let stride = std::mem::size_of::<NXArchInfoGuest>() as GuestUSize;
    let total = stride.checked_mul(count + 1).unwrap();
    let base: MutPtr<NXArchInfoGuest> = env.mem.alloc(total).cast();
    for (i, entry) in NX_ARCH_INFOS.iter().enumerate() {
        let name = intern_string(env, entry.name);
        let description = intern_string(env, entry.description);
        let info = NXArchInfoGuest {
            name,
            cputype: entry.cputype,
            cpusubtype: entry.cpusubtype,
            byteorder: entry.byteorder,
            description,
        };
        env.mem.write(base + (i as GuestUSize), info);
    }
    // NULL terminator: all-zero entry signals end of list, matching what
    // `cctools` ships in `/usr/lib/libmacho.dylib`.
    let terminator = NXArchInfoGuest {
        name: Ptr::null(),
        cputype: 0,
        cpusubtype: 0,
        byteorder: NX_UNKNOWN_BYTE_ORDER,
        description: Ptr::null(),
    };
    env.mem.write(base + count, terminator);
    let const_ptr = base.cast_const();
    env.libc_state.mach_o.arch_info_list = Some(const_ptr);
    const_ptr
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(get_end()),
    export_c_func!(get_etext()),
    export_c_func!(host_info(_, _, _, _)),
    export_c_func!(_dyld_image_count()),
    export_c_func!(_dyld_get_image_header(_)),
    export_c_func!(_dyld_get_image_name(_)),
    export_c_func!(_dyld_get_image_vmaddr_slide(_)),
    export_c_func!(_dyld_register_func_for_add_image(_)),
    export_c_func!(_dyld_register_func_for_remove_image(_)),
    export_c_func!(NXGetArchInfoFromCpuType(_, _)),
    export_c_func!(NXGetArchInfoFromName(_)),
    export_c_func!(NXGetLocalArchInfo()),
    export_c_func!(NXGetAllArchInfos()),
];
