/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `arpa/inet.h` (Internet address manipulation routines)

use crate::libc::netdb::socklen_t;
use crate::libc::sys::socket::AF_INET;
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, SafeRead};
use crate::{export_c_func, Environment};

use crate::dyld::FunctionExports;
use std::net::Ipv4Addr;

#[allow(non_camel_case_types)]
type in_addr_t = u32;

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
#[allow(non_camel_case_types)]
struct in_addr {
    s_addr: in_addr_t,
}
unsafe impl SafeRead for in_addr {}

fn inet_addr(env: &mut Environment, str: ConstPtr<u8>) -> in_addr_t {
    let inet_addr_str = env.mem.cstr_at_utf8(str).unwrap();
    let address: Ipv4Addr = inet_addr_str.parse().unwrap();
    let res = u32::from_le_bytes(address.octets());
    log_dbg!("inet_addr({:?}) => {}", inet_addr_str, res);
    res
}

fn inet_ntop(
    env: &mut Environment,
    af: i32,
    src: ConstVoidPtr,
    dst: MutPtr<u8>,
    size: socklen_t,
) -> ConstPtr<u8> {
    assert_eq!(af, AF_INET);
    let addr_ptr: ConstPtr<in_addr> = src.cast();
    let addr = env.mem.read(addr_ptr);
    let ipv4_addr = Ipv4Addr::from_bits(u32::from_be(addr.s_addr));
    log_dbg!("inet_ntop: addr = {:?}", ipv4_addr);
    let binding = ipv4_addr.to_string();
    let addr_bytes = binding.as_bytes();
    let len: GuestUSize = addr_bytes.len().try_into().unwrap();
    assert!(len < size);
    env.mem.bytes_at_mut(dst, len).copy_from_slice(addr_bytes);
    env.mem.write(dst + len, b'\0');
    dst.cast_const()
}

fn inet_pton(env: &mut Environment, af: i32, src: ConstPtr<u8>, dst: MutVoidPtr) -> i32 {
    assert_eq!(af, AF_INET);
    let str = env.mem.cstr_at_utf8(src.cast()).unwrap();
    log_dbg!("inet_pton '{}'", str);
    let address: Ipv4Addr = str.parse().unwrap();
    let addr = in_addr {
        s_addr: u32::from_le_bytes(address.octets()),
    };
    let addr_ptr: MutPtr<in_addr> = dst.cast();
    env.mem.write(addr_ptr, addr);
    1 // address was valid, success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(inet_addr(_)),
    export_c_func!(inet_ntop(_, _, _, _)),
    export_c_func!(inet_pton(_, _, _)),
];
