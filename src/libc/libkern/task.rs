/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `task`

use crate::dyld::FunctionExports;
use crate::libc::mach::core_types::{
    integer_t, kern_return_t, mach_msg_type_number_t, natural_t, policy_t, time_value_t, vm_size_t,
    KERN_SUCCESS,
};
use crate::libc::mach::init::MACH_TASK_SELF;
use crate::mem::{guest_size_of, MutPtr, SafeRead};
use crate::{export_c_func, Environment};

#[repr(C, packed)]
struct OpaqueTask {
    _unused: u32,
}
unsafe impl SafeRead for OpaqueTask {}

#[repr(C, packed)]
struct task_basic_info {
    suspend_count: integer_t,
    virtual_size: vm_size_t,
    resident_size: vm_size_t,
    user_time: time_value_t,
    system_time: time_value_t,
    policy: policy_t,
}
unsafe impl SafeRead for task_basic_info {}

#[allow(non_camel_case_types)]
type task_name_t = MutPtr<OpaqueTask>;
#[allow(non_camel_case_types)]
type task_flavor_t = natural_t;
#[allow(non_camel_case_types)]
type task_info_t = MutPtr<integer_t>;

fn task_info(
    env: &mut Environment,
    target_task: task_name_t,
    flavor: task_flavor_t,
    task_info_out: task_info_t,
    task_info_out_cnt: MutPtr<mach_msg_type_number_t>,
) -> kern_return_t {
    log!(
        "TODO: task_info({:?}, {:?}, {:?}, {:?})",
        target_task,
        flavor,
        task_info_out,
        task_info_out_cnt
    );
    assert_eq!(target_task, task_name_t::from_bits(MACH_TASK_SELF));
    assert_eq!(flavor, 4); // TASK_BASIC_INFO
    let out_size_available = env.mem.read(task_info_out_cnt);
    let out_size_expected = guest_size_of::<task_basic_info>() / guest_size_of::<integer_t>();
    assert_eq!(out_size_expected, out_size_available);
    // Values taken from an iPod Touch 4 running iOS 6.1
    env.mem.write(
        task_info_out.cast(),
        task_basic_info {
            suspend_count: 7565,
            virtual_size: 50419,
            resident_size: 616650,
            user_time: time_value_t {
                seconds: 0xC313,
                microseconds: 0x35D2,
            },
            system_time: time_value_t {
                seconds: 0x316C7,
                microseconds: 0x343E,
            },
            policy: 4348,
        },
    );
    KERN_SUCCESS
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(task_info(_, _, _, _))];
