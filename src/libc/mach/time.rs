/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `mach_time.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{MutPtr, SafeRead};
use crate::Environment;
use std::time::Instant;

#[repr(C, packed)]
struct struct_mach_timebase_info {
    numerator: u32,
    denominator: u32,
}
unsafe impl SafeRead for struct_mach_timebase_info {}

#[allow(non_camel_case_types)]
type kern_return_t = i32;
const KERN_SUCCESS: kern_return_t = 0;

fn mach_timebase_info(
    env: &mut Environment,
    info: MutPtr<struct_mach_timebase_info>,
) -> kern_return_t {
    env.mem.write(
        info,
        struct_mach_timebase_info {
            numerator: 1,
            denominator: 1,
        },
    );
    KERN_SUCCESS
}

/// The result of this function, multiplied by the constant from
/// [mach_timebase_info], should be the absolute time in nanoseconds.
/// The absolute time is a monotonic clock with an arbitrary starting point.
fn mach_absolute_time(env: &mut Environment) -> u64 {
    let now = Instant::now();
    now.duration_since(env.startup_time)
        .as_nanos()
        .try_into()
        .unwrap()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(mach_timebase_info(_)),
    export_c_func!(mach_absolute_time()),
];
