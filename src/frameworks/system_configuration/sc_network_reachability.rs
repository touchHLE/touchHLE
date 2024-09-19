/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! SCNetworkReachability

use crate::dyld::FunctionExports;
use crate::frameworks::core_foundation::cf_allocator::{kCFAllocatorDefault, CFAllocatorRef};
use crate::mem::{ConstPtr, MutPtr, Ptr, SafeRead};
use crate::{export_c_func, Environment};

#[repr(C, packed)]
struct SCNetworkReachability {}
unsafe impl SafeRead for SCNetworkReachability {}

type SCNetworkReachabilityRef = MutPtr<SCNetworkReachability>;

fn SCNetworkReachabilityCreateWithName(
    env: &mut Environment,
    allocator: CFAllocatorRef,
    name: ConstPtr<u8>,
) -> SCNetworkReachabilityRef {
    assert_eq!(allocator, kCFAllocatorDefault); // unimplemented
    log!(
        "TODO: SCNetworkReachabilityCreateWithName({:?}, {:?} {:?}) -> NULL",
        allocator,
        name,
        env.mem.cstr_at_utf8(name)
    );
    Ptr::null()
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(SCNetworkReachabilityCreateWithName(_, _))];
