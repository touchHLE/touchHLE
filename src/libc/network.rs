/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::mem::{ConstPtr, ConstVoidPtr, MutPtr, MutVoidPtr};

#[allow(non_camel_case_types)]
struct ifaddrs {}

fn getifaddrs(_env: &mut Environment, _ifap: MutPtr<MutPtr<ifaddrs>>) -> i32 {
    -1
}

fn if_nameindex(_env: &mut Environment, _ifname: ConstPtr<u8>) -> i32 {
    0
}

#[allow(clippy::too_many_arguments)]
fn DNSServiceBrowse(
    _env: &mut Environment,
    _sdRef: MutVoidPtr,
    _flags: u32,
    _interfaceIndex: u32,
    _regtype: ConstPtr<u8>,
    _domain: ConstPtr<u8>,
    _callBack: ConstVoidPtr,
    _context: MutVoidPtr,
) -> i32 {
    -1
}

fn DNSServiceRefSockFD(_env: &mut Environment, _sdRef: MutVoidPtr) -> i32 {
    -1
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(getifaddrs(_)),
    export_c_func!(if_nameindex(_)),
    export_c_func!(DNSServiceBrowse(_, _, _, _, _, _, _)),
    export_c_func!(DNSServiceRefSockFD(_)),
];
