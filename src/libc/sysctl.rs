/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `sys/sysctl.h`

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::{set_errno, ENOENT, ENOMEM};
use crate::libc::net::if_::{IfInfo, INTERFACES};
use crate::mem::{guest_size_of, ConstPtr, GuestUSize, MutPtr, MutVoidPtr, PAGE_SIZE};
use crate::Environment;

// Top level constants
const CTL_KERN: i32 = 1;
const CTL_NET: i32 = 4;
const CTL_HW: i32 = 6;

// CTL_NET / PF_ROUTE
const PF_ROUTE: i32 = 17;
const AF_INET: i32 = 2;
const AF_LINK: i32 = 18;
const NET_RT_IFLIST: i32 = 3;

// net/route.h message constants (32-bit Darwin layout)
const RTM_VERSION: u8 = 5;
const RTM_NEWADDR: u8 = 12;
const RTM_IFINFO: u8 = 14;
const RTA_NETMASK: u32 = 0x4;
const RTA_IFP: u32 = 0x10;
const RTA_IFA: u32 = 0x20;

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
        6 => {
            let elems: Vec<i32> = (0..name_len).map(|i| env.mem.read(name + i)).collect();
            // {CTL_NET, PF_ROUTE, 0, AF_LINK|AF_INET, NET_RT_IFLIST, ifindex}:
            // routing-table interface dump, used by networking SDKs to find
            // the local interface addresses (getifaddrs() underneath).
            if elems[0] == CTL_NET
                && elems[1] == PF_ROUTE
                && elems[2] == 0
                && (elems[3] == AF_LINK || elems[3] == AF_INET)
                && elems[4] == NET_RT_IFLIST
            {
                return sysctl_rt_iflist(env, elems[5] as u32, oldp, oldlenp);
            }
            set_errno(env, ENOENT);
            log!("TODO: sysctl() for name {elems:?} is unimplemented, returning -1");
            -1
        }
        _ => {
            // Tolerate unknown MIB lengths instead of crashing: log the query
            // and report "not found".
            let elems: Vec<i32> = (0..name_len).map(|i| env.mem.read(name + i)).collect();
            set_errno(env, ENOENT);
            log!("TODO: sysctl() for name {elems:?} is unimplemented, returning -1");
            -1
        }
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

/// Builds a `NET_RT_IFLIST` response: a stream of `RTM_IFINFO` + `RTM_NEWADDR`
/// routing messages (32-bit Darwin layout) describing the emulated interfaces,
/// as returned by the kernel for `sysctl({CTL_NET, PF_ROUTE, 0, AF_LINK,
/// NET_RT_IFLIST, ifindex})`. `ifindex == 0` dumps all interfaces.
fn sysctl_rt_iflist(
    env: &mut Environment,
    ifindex: u32,
    oldp: MutVoidPtr,
    oldlenp: MutPtr<GuestUSize>,
) -> i32 {
    let mut blob: Vec<u8> = Vec::new();
    for info in INTERFACES {
        if ifindex != 0 && info.index != ifindex {
            continue;
        }
        push_ifinfo(&mut blob, info);
        push_newaddr(&mut blob, info);
    }
    if blob.is_empty() {
        set_errno(env, ENOENT);
        return -1;
    }
    if oldp.is_null() {
        // Size-only probe.
        env.mem.write(oldlenp, blob.len() as GuestUSize);
        return 0;
    }
    let oldlen = env.mem.read(oldlenp);
    if (oldlen as usize) < blob.len() {
        env.mem.write(oldlenp, blob.len() as GuestUSize);
        set_errno(env, ENOMEM);
        return -1;
    }
    for (i, byte) in blob.iter().enumerate() {
        env.mem.write(oldp.cast::<u8>() + i as u32, *byte);
    }
    env.mem.write(oldlenp, blob.len() as GuestUSize);
    0
}

fn push_u16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn push_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

/// `struct sockaddr_dl` (11–20 bytes), rounded up to a 4-byte boundary.
fn push_sockaddr_dl(buf: &mut Vec<u8>, info: &IfInfo) {
    let name = info.name.as_bytes();
    let sdl_len = 8 + name.len() + info.alen as usize;
    buf.push(sdl_len as u8);
    buf.push(AF_LINK as u8);
    push_u16(buf, info.index as u16);
    buf.push(info.sdl_type);
    buf.push(name.len() as u8);
    buf.push(info.alen);
    buf.push(0); // sdl_slen
    buf.extend_from_slice(name);
    buf.extend_from_slice(&info.mac[..info.alen as usize]);
    while buf.len() % 4 != 0 {
        buf.push(0);
    }
}

/// `struct sockaddr_in` (16 bytes).
fn push_sockaddr_in(buf: &mut Vec<u8>, addr: [u8; 4]) {
    buf.push(16); // sin_len
    buf.push(AF_INET as u8);
    push_u16(buf, 0); // sin_port
    buf.extend_from_slice(&addr);
    buf.extend_from_slice(&[0u8; 8]); // sin_zero
}

/// `struct if_data` (96 bytes on 32-bit Darwin).
fn push_if_data(buf: &mut Vec<u8>, info: &IfInfo) {
    buf.push(info.sdl_type); // ifi_type
    buf.push(0); // ifi_typelen
    buf.push(0); // ifi_physical
    buf.push(info.alen); // ifi_addrlen
    buf.push(if info.alen > 0 { 14 } else { 0 }); // ifi_hdrlen
    buf.push(0); // ifi_recvquota
    buf.push(0); // ifi_xmitquota
    buf.push(0); // ifi_unused1
    push_u32(buf, info.mtu); // ifi_mtu
    push_u32(buf, 0); // ifi_metric
    push_u32(buf, 10_000_000); // ifi_baudrate
    for _ in 0..13 {
        push_u32(buf, 0); // volatile statistics
    }
    push_u32(buf, 0); // ifi_lastchange.tv_sec
    push_u32(buf, 0); // ifi_lastchange.tv_usec
    push_u32(buf, 0); // ifi_unused2
    push_u32(buf, 0); // ifi_hwassist
    push_u32(buf, 0); // ifi_reserved1
    push_u32(buf, 0); // ifi_reserved2
}

/// `RTM_IFINFO`: `struct if_msghdr` (112 bytes) + `sockaddr_dl`.
fn push_ifinfo(buf: &mut Vec<u8>, info: &IfInfo) {
    let mut dl: Vec<u8> = Vec::new();
    push_sockaddr_dl(&mut dl, info);
    push_u16(buf, (112 + dl.len()) as u16); // ifm_msglen
    buf.push(RTM_VERSION);
    buf.push(RTM_IFINFO);
    push_u32(buf, RTA_IFP); // ifm_addrs
    push_u32(buf, info.flags); // ifm_flags
    push_u16(buf, info.index as u16); // ifm_index
    push_u16(buf, 0); // padding
    push_if_data(buf, info);
    buf.extend_from_slice(&dl);
}

/// `RTM_NEWADDR`: `struct ifa_msghdr` (20 bytes) + netmask + `sockaddr_dl` +
/// address (sockaddrs in ascending `RTA_*` bit order).
fn push_newaddr(buf: &mut Vec<u8>, info: &IfInfo) {
    let mut dl: Vec<u8> = Vec::new();
    push_sockaddr_dl(&mut dl, info);
    push_u16(buf, (20 + 16 + dl.len() + 16) as u16); // ifam_msglen
    buf.push(RTM_VERSION);
    buf.push(RTM_NEWADDR);
    push_u32(buf, RTA_NETMASK | RTA_IFP | RTA_IFA); // ifam_addrs
    push_u32(buf, info.flags); // ifam_flags
    push_u16(buf, info.index as u16); // ifam_index
    push_u16(buf, 0); // padding
    push_u32(buf, 0); // ifam_metric
    push_sockaddr_in(buf, info.netmask);
    buf.extend_from_slice(&dl);
    push_sockaddr_in(buf, info.ip);
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sysctl(_, _, _, _, _, _)),
    export_c_func!(sysctlbyname(_, _, _, _, _)),
];
