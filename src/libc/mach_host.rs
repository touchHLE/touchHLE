/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `mach_host.h` and some other related functions

#![allow(non_camel_case_types)]

use crate::dyld::FunctionExports;
use crate::libc::mach_thread_info::{
    kern_return_t, mach_msg_type_number_t, mach_port_t, natural_t, KERN_SUCCESS,
};
use crate::mem::{guest_size_of, MutPtr, SafeRead};
use crate::{export_c_func, Environment};

type host_t = mach_port_t;
type host_name_port_t = host_t;
type host_flavor_t = natural_t;
type host_info_t = MutPtr<natural_t>;
type vm_size_t = natural_t;

// The value doesn't matter that much, only the fact that it's unique
// per host so we could assert against it in our code.
const MACH_HOST_SELF: host_name_port_t = 0x100c442e;

pub const PAGE_SIZE: vm_size_t = 4096;

const HOST_VM_INFO: host_flavor_t = 2;

#[repr(C, packed)]
struct vm_statistics {
    free_count: natural_t,
    active_count: natural_t,
    inactive_count: natural_t,
    wire_count: natural_t,
    zero_fill_count: natural_t,
    reactivations: natural_t,
    pageins: natural_t,
    pageouts: natural_t,
    faults: natural_t,
    cow_faults: natural_t,
    lookups: natural_t,
    hits: natural_t,
    purgeable_count: natural_t,
    purges: natural_t,
    speculative_count: natural_t,
}
unsafe impl SafeRead for vm_statistics {}

fn mach_host_self(_env: &mut Environment) -> host_name_port_t {
    MACH_HOST_SELF
}

fn host_page_size(
    env: &mut Environment,
    host: host_t,
    out_page_size: MutPtr<vm_size_t>,
) -> kern_return_t {
    assert_eq!(host, MACH_HOST_SELF);
    env.mem.write(out_page_size, PAGE_SIZE);
    KERN_SUCCESS
}

fn host_statistics(
    env: &mut Environment,
    host: host_t,
    flavor: host_flavor_t,
    host_info_out: host_info_t,
    host_info_out_count: MutPtr<mach_msg_type_number_t>,
) -> kern_return_t {
    assert_eq!(host, MACH_HOST_SELF);
    assert_eq!(flavor, HOST_VM_INFO);
    let out_size_available = env.mem.read(host_info_out_count);
    let out_size_expected = guest_size_of::<vm_statistics>() / guest_size_of::<natural_t>();
    assert_eq!(out_size_expected, out_size_available);
    // Below values corresponds to a run of an iOS Simulator.
    // As touchHLE doesn't have a paging system (yet? never?),
    // those numbers are (mostly) meaningless.
    // In reality, this function is commonly used by apps to get
    // the amount of current free memory available.
    // This output roughly corresponds to 1.1 Gb of free vm memory out of 2 Gb.
    // TODO: approximate size of current memory allocations and return them?
    env.mem.write(
        host_info_out.cast(),
        vm_statistics {
            free_count: 287306,
            active_count: 159853,
            inactive_count: 23544,
            wire_count: 47539,
            zero_fill_count: 33647739,
            reactivations: 129,
            pageins: 183535,
            pageouts: 0,
            faults: 51089020,
            cow_faults: 3169189,
            lookups: 896665,
            hits: 361073,
            purgeable_count: 14412,
            purges: 0,
            speculative_count: 53842,
        },
    );
    KERN_SUCCESS
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(mach_host_self()),
    export_c_func!(host_page_size(_, _)),
    export_c_func!(host_statistics(_, _, _, _)),
];
