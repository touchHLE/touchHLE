/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `net/if.h`

use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::mem::{ConstPtr, MutPtr, Ptr};
use crate::Environment;

/// Description of one emulated network interface. The addresses are fake
/// (RFC 1918 / loopback); only the index/name/address mapping is emulated so
/// games can complete their networking setup and gracefully discover that
/// their actual online services are unreachable.
pub struct IfInfo {
    pub index: u32,
    pub name: &'static str,
    /// `IFT_*` type (6 = ethernet, 23 = ppp, 24 = loopback).
    pub sdl_type: u8,
    /// Media address (MAC) length in bytes.
    pub alen: u8,
    /// `IFF_*` flags.
    pub flags: u32,
    pub mtu: u32,
    pub mac: [u8; 6],
    pub ip: [u8; 4],
    pub netmask: [u8; 4],
}

// IFF_* (Darwin)
pub const IFF_UP: u32 = 0x1;
pub const IFF_BROADCAST: u32 = 0x2;
pub const IFF_LOOPBACK: u32 = 0x8;
pub const IFF_POINTOPOINT: u32 = 0x10;
pub const IFF_RUNNING: u32 = 0x40;
pub const IFF_MULTICAST: u32 = 0x80;

/// Emulated interface table, matching typical iOS assignments: loopback,
/// Wi-Fi (en0) and cellular (pdp_ip0).
pub const INTERFACES: &[IfInfo] = &[
    IfInfo {
        index: 1,
        name: "lo0",
        sdl_type: 24,
        alen: 0,
        flags: IFF_UP | IFF_LOOPBACK | IFF_RUNNING,
        mtu: 16384,
        mac: [0; 6],
        ip: [127, 0, 0, 1],
        netmask: [255, 0, 0, 0],
    },
    IfInfo {
        index: 4,
        name: "en0",
        sdl_type: 6,
        alen: 6,
        flags: IFF_UP | IFF_BROADCAST | IFF_RUNNING | IFF_MULTICAST,
        mtu: 1500,
        mac: [0x00, 0x11, 0x22, 0x33, 0x44, 0x55],
        ip: [192, 168, 1, 100],
        netmask: [255, 255, 255, 0],
    },
    IfInfo {
        index: 5,
        name: "pdp_ip0",
        sdl_type: 23,
        alen: 0,
        flags: IFF_UP | IFF_POINTOPOINT | IFF_RUNNING,
        mtu: 1500,
        mac: [0; 6],
        ip: [10, 0, 0, 2],
        netmask: [255, 0, 0, 0],
    },
];

/// `IF_NAMESIZE` (16 on Darwin).
const IF_NAMESIZE: u32 = 16;

/// `struct if_nameindex { unsigned int if_index; char *if_name; }` — 8 bytes
/// per entry, list terminated by a `{0, NULL}` sentinel.
#[repr(C)]
#[allow(non_camel_case_types)]
struct if_nameindex {
    if_index: u32,
    if_name: ConstPtr<u8>,
}

fn if_nameindex(env: &mut Environment) -> ConstPtr<if_nameindex> {
    // One entry per emulated interface, plus the {0, NULL} terminator.
    let count = INTERFACES.len() as u32 + 1;
    let list: MutPtr<if_nameindex> = env.mem.alloc(8 * count).cast();
    for (i, info) in INTERFACES.iter().enumerate() {
        let name_ptr = env.mem.alloc_and_write_cstr(info.name.as_bytes());
        let slot = list + i as u32;
        env.mem.write(slot.cast::<u32>(), info.index);
        env.mem.write(slot.cast::<u32>() + 1u32, name_ptr.to_bits());
    }
    // Sentinel entry {0, NULL}.
    let sentinel = list + (count - 1);
    env.mem.write(sentinel.cast::<u32>(), 0u32);
    env.mem.write(sentinel.cast::<u32>() + 1u32, 0u32);
    list.cast_const()
}

/// `unsigned int if_nametoindex(const char *ifname)` — returns 0 when the
/// interface does not exist.
fn if_nametoindex(env: &mut Environment, ifname: ConstPtr<u8>) -> u32 {
    if ifname.is_null() {
        return 0;
    }
    let name = env.mem.cstr_at(ifname);
    INTERFACES
        .iter()
        .find(|info| name == info.name.as_bytes())
        .map(|info| info.index)
        .unwrap_or(0)
}

/// `char *if_indextoname(unsigned int ifindex, char *ifname)` — NULL when the
/// index does not exist; otherwise copies the name into `ifname` (which must
/// hold at least `IF_NAMESIZE` bytes) and returns it.
fn if_indextoname(env: &mut Environment, ifindex: u32, ifname: MutPtr<u8>) -> MutPtr<u8> {
    if ifname.is_null() {
        return Ptr::null();
    }
    let entry = INTERFACES.iter().find(|info| info.index == ifindex);
    match entry {
        Some(info) => {
            let mut bytes = [0u8; IF_NAMESIZE as usize];
            bytes[..info.name.len()].copy_from_slice(info.name.as_bytes());
            for (i, b) in bytes.iter().enumerate() {
                env.mem.write(ifname + i as u32, *b);
            }
            ifname
        }
        None => Ptr::null(),
    }
}

fn if_freenameindex(_env: &mut Environment, _ptr: MutPtr<if_nameindex>) {}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(if_nameindex()),
    export_c_func!(if_nametoindex(_)),
    export_c_func!(if_indextoname(_, _)),
    export_c_func!(if_freenameindex(_)),
];
