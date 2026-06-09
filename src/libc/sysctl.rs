/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `sys/sysctl.h`

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::{set_errno, ENOENT};
use crate::mem::{guest_size_of, ConstPtr, GuestUSize, MutPtr, MutVoidPtr, PAGE_SIZE};
use crate::Environment;

// Top level constants
const CTL_KERN: i32 = 1;
const CTL_HW: i32 = 6;

// CTL_KERN
const KERN_OSTYPE: i32 = 1;
const KERN_OSRELEASE: i32 = 2;
const KERN_OSREV: i32 = 3;
const KERN_VERSION: i32 = 4;
const KERN_HOSTNAME: i32 = 10;
const KERN_PROC: i32 = 14;
const KERN_OSVERSION: i32 = 65;

// KERN_PROC
const KERN_PROC_ALL: i32 = 0;

// CTL_HW
const HW_MACHINE: i32 = 1;
const HW_MODEL: i32 = 2;
const HW_NCPU: i32 = 3;
const HW_PHYSMEM: i32 = 5;
const HW_USERMEM: i32 = 6;
const HW_PAGESIZE: i32 = 7;
const HW_BUS_FREQ: i32 = 14;
const HW_CPU_FREQ: i32 = 15;
const HW_MEMSIZE: i32 = 24;

/// There is probably more idiomatic way to express "a variable sized array of
/// integers, up to 12 max", but this enum would do for now.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum SysCtlNamePath {
    Undefined,
    Length2(i32, i32),
    Length4(i32, i32, i32, i32),
}

// Clippy complains about the type.
// Below values corresponds to the original iPhone.
// Reference https://www.mail-archive.com/misc@openbsd.org/msg80988.html
static SYSCTL_VALUES: [(SysCtlNamePath, &str, SysInfoType); 18] = [
    // Generic CPU, I/O
    (SysCtlNamePath::Length2(CTL_HW, HW_MACHINE), "hw.machine", SysInfoType::String(b"iPhone1,1")),
    (SysCtlNamePath::Length2(CTL_HW, HW_MODEL), "hw.model", SysInfoType::String(b"M68AP")),
    (SysCtlNamePath::Length2(CTL_HW, HW_NCPU), "hw.ncpu", SysInfoType::Int32(1)),
    (SysCtlNamePath::Undefined, "hw.cputype", SysInfoType::Int32(12)),
    (SysCtlNamePath::Undefined, "hw.cpusubtype", SysInfoType::Int32(6)),
    (SysCtlNamePath::Length2(CTL_HW, HW_CPU_FREQ), "hw.cpufrequency", SysInfoType::Int64(412000000)),
    (SysCtlNamePath::Length2(CTL_HW, HW_BUS_FREQ), "hw.busfrequency", SysInfoType::Int64(103000000)),
    (SysCtlNamePath::Length2(CTL_HW, HW_PHYSMEM), "hw.physmem", SysInfoType::Int32(121634816)),
    (SysCtlNamePath::Length2(CTL_HW, HW_USERMEM), "hw.usermem", SysInfoType::Int32(93564928)),
    (SysCtlNamePath::Length2(CTL_HW, HW_MEMSIZE), "hw.memsize", SysInfoType::Int32(121634816)),
    (SysCtlNamePath::Length2(CTL_HW, HW_PAGESIZE), "hw.pagesize", SysInfoType::Int64(PAGE_SIZE as i64)),
    // High kernel limits
    (SysCtlNamePath::Length2(CTL_KERN, KERN_OSTYPE), "kern.ostype", SysInfoType::String(b"Darwin")),
    (SysCtlNamePath::Length2(CTL_KERN, KERN_OSRELEASE), "kern.osrelease", SysInfoType::String(b"10.0.0d3")),
    (SysCtlNamePath::Length2(CTL_KERN, KERN_OSREV), "kern.osrevision", SysInfoType::String(b"199506")),
    (SysCtlNamePath::Length2(CTL_KERN, KERN_HOSTNAME), "kern.hostname", SysInfoType::String(b"touchHLE")), // this is arbitrary
    (SysCtlNamePath::Length2(CTL_KERN, KERN_VERSION), "kern.version", SysInfoType::String(b"Darwin Kernel Version 10.0.0d3: Wed May 13 22:11:58 PDT 2009; root:xnu-1357.2.89~4/RELEASE_ARM_S5L8900X")),
    (SysCtlNamePath::Length2(CTL_KERN, KERN_OSVERSION), "kern.osversion", SysInfoType::String(b"7A341")),
    // Last 0 here is an unnamed placeholder
    (SysCtlNamePath::Length4(CTL_KERN, KERN_PROC, KERN_PROC_ALL, 0), "kern.proc.all", SysInfoType::Struct),
];

static STRING_MAP: LazyLock<HashMap<&str, SysInfoType>> = LazyLock::new(|| {
    // Can't use from_iter because the closure erases the lifetime
    let mut hashmap = HashMap::new();
    for (_, str, value) in SYSCTL_VALUES.iter() {
        hashmap.insert(*str, value.clone());
    }
    hashmap
});

#[allow(clippy::type_complexity)]
static INT_MAP: LazyLock<HashMap<SysCtlNamePath, (&str, SysInfoType)>> = LazyLock::new(|| {
    // Can't use from_iter because the closure erases the lifetime
    let mut hashmap = HashMap::new();
    for (ints, str, value) in SYSCTL_VALUES.iter() {
        if *ints == SysCtlNamePath::Undefined {
            // skip entries which do not have integer name paths
            continue;
        }
        hashmap.insert(*ints, (*str, value.clone()));
    }
    hashmap
});

#[derive(Clone)]
enum SysInfoType {
    String(&'static [u8]),
    Int32(i32),
    Int64(i64),
    Struct,
}

fn sysctl(
    env: &mut Environment,
    name: MutPtr<i32>,
    name_len: u32,
    oldp: MutVoidPtr,
    oldlenp: MutPtr<GuestUSize>,
    newp: MutVoidPtr,
    newlen: GuestUSize,
) -> i32 {
    set_errno(env, 0);

    log_dbg!(
        "sysctl({:?}, {:#x}, {:?}, {:?}, {:?}, {:x})",
        name,
        name_len,
        oldp,
        oldlenp,
        newp,
        newlen
    );
    match name_len {
        2 => {
            let (name0, name1) = (env.mem.read(name), env.mem.read(name + 1));
            sysctl_generic(
                env,
                |_| {
                    let Some(val) = INT_MAP.get(&SysCtlNamePath::Length2(name0, name1)).cloned()
                    else {
                        unimplemented!("Unknown sysctl parameter ({name0}, {name1})!")
                    };
                    val
                },
                oldp,
                oldlenp,
                newp,
                newlen,
            )
        }
        4 => {
            let (name0, name1, name2, name3) = (
                env.mem.read(name),
                env.mem.read(name + 1),
                env.mem.read(name + 2),
                env.mem.read(name + 3),
            );
            if SysCtlNamePath::Length4(name0, name1, name2, name3)
                == SysCtlNamePath::Length4(CTL_KERN, KERN_PROC, KERN_PROC_ALL, 0)
            {
                // In some Unity games, mono initialization set-ups perf
                // counters, which set-ups a shared memory area, which gets
                // a list of all processes via sysctl() if shm_open() fails.
                // (See mono's [mono-perfcounters.c](https://github.com/mono/mono/blob/62121afbb28f0b62f100ec9a942d10c5e0f4814f/mono/metadata/mono-perfcounters.c#L392)
                // and [mono-mmap.c](https://github.com/mono/mono/blob/0f53e9e151d92944cacab3e24ac359410c606df6/mono/utils/mono-mmap.c#L555))
                // Stubbing that sysctl() call doesn't seem to have any impact
                // on the games' functionality.
                //
                // ...but wait, why would perf counters need a shared memory
                // area in the first place? This is left as an exercise for
                // the reader.
                set_errno(env, ENOENT);
                log!("TODO: sysctl() for 'kern.proc.all', returning -1");
                return -1;
            }
            sysctl_generic(
                env,
                |_| {
                    let Some(val) = INT_MAP
                        .get(&SysCtlNamePath::Length4(name0, name1, name2, name3))
                        .cloned()
                    else {
                        unimplemented!(
                            "Unknown sysctl parameter ({name0}, {name1}, {name2}, {name3})!"
                        )
                    };
                    val
                },
                oldp,
                oldlenp,
                newp,
                newlen,
            )
        }
        _ => unimplemented!("sysctl() for name length {name_len} is unimplemented!"),
    }
}

fn sysctlbyname(
    env: &mut Environment,
    name: ConstPtr<u8>,
    oldp: MutVoidPtr,
    oldlenp: MutPtr<GuestUSize>,
    newp: MutVoidPtr,
    newlen: GuestUSize,
) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    let name_str = env.mem.cstr_at_utf8(name).unwrap();
    log_dbg!(
        "sysctlbyname({:?}, {:?}, {:?}, {:?}, {:x})",
        name_str,
        oldp,
        oldlenp,
        newp,
        newlen
    );
    sysctl_generic(
        env,
        |env| {
            let name_str = env.mem.cstr_at_utf8(name).unwrap();
            let Some((name_str, val)) = STRING_MAP.get_key_value(name_str) else {
                unimplemented!("Unknown sysctlbyname parameter {name_str}!")
            };
            (name_str, val.clone())
        },
        oldp,
        oldlenp,
        newp,
        newlen,
    )
}

fn sysctl_generic<F>(
    env: &mut Environment,
    // Returns the name and value of the property (or exits)
    name_lookup: F,
    oldp: MutVoidPtr,
    oldlenp: MutPtr<GuestUSize>,
    newp: MutVoidPtr,
    newlen: GuestUSize,
) -> i32
where
    F: FnOnce(&mut Environment) -> (&'static str, SysInfoType),
{
    assert!(newp.is_null());
    assert_eq!(newlen, 0);

    let (name_str, val) = name_lookup(env);
    let len: GuestUSize = match val {
        SysInfoType::String(str) => str.len() as GuestUSize + 1,
        SysInfoType::Int32(_) => guest_size_of::<i32>(),
        SysInfoType::Int64(_) => guest_size_of::<i64>(),
        _ => unimplemented!(),
    };
    if oldp.is_null() {
        env.mem.write(oldlenp, len);
        return 0;
    }
    assert!(!oldp.is_null() && !oldlenp.is_null());
    let oldlen = env.mem.read(oldlenp);
    if oldlen < len {
        // TODO: set errno
        // TODO: write partial data
        log!("sysctl(byname) for '{name_str}': the buffer of size {oldlen} is too low to fit the value of size {len}, returning -1");
        return -1;
    }
    match val {
        SysInfoType::String(str) => {
            let sysctl_str = env.mem.alloc_and_write_cstr(str);
            env.mem.memmove(oldp, sysctl_str.cast().cast_const(), len);
            env.mem.free(sysctl_str.cast());
        }
        SysInfoType::Int32(num) => {
            env.mem.write(oldp.cast(), num);
        }
        SysInfoType::Int64(num) => {
            env.mem.write(oldp.cast(), num);
        }
        _ => unimplemented!(),
    }
    env.mem.write(oldlenp, len);
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sysctl(_, _, _, _, _, _)),
    export_c_func!(sysctlbyname(_, _, _, _, _)),
];
