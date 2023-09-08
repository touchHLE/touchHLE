/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! DNS Service Discovery C

use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::mem::{ConstPtr, ConstVoidPtr, MutVoidPtr};
use crate::Environment;

type DNSServiceErrorType = i32;
const kDNSServiceErr_Unsupported: DNSServiceErrorType = -65544;

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
) -> DNSServiceErrorType {
    // TODO: implement
    kDNSServiceErr_Unsupported
}

fn DNSServiceRefSockFD(_env: &mut Environment, _sdRef: MutVoidPtr) -> DNSServiceErrorType {
    // TODO: implement
    kDNSServiceErr_Unsupported
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(DNSServiceBrowse(_, _, _, _, _, _, _)),
    export_c_func!(DNSServiceRefSockFD(_)),
];
