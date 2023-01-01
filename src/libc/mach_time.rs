//! `mach_time.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{MutPtr, SafeRead};
use crate::Environment;

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

pub const FUNCTIONS: FunctionExports = &[export_c_func!(mach_timebase_info(_))];
