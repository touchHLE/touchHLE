/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! This is separated out into its own package so that we can avoid rebuilding
//! dynarmic more often than necessary, and to improve build-time parallelism.

// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]

/// Opaque type from C
#[allow(non_camel_case_types)]
pub type touchHLE_DynarmicWrapper = std::ffi::c_void;
/// Opaque type from Rust (this is the `Mem` type from the main crate, but
/// `c_void` is used here to avoid depending on it directly)
#[allow(non_camel_case_types)]
pub type touchHLE_Mem = std::ffi::c_void;
/// Opaque C++ type
#[allow(non_camel_case_types)]
pub type Dynarmic_A32_Context = std::ffi::c_void;

type VAddr = u32;

// Import functions from lib.cpp, see build.rs. Note that lib.cpp depends on
// some functions being exported from Rust, but those are in the main crate.
extern "C" {
    pub fn touchHLE_DynarmicWrapper_new(
        dynamic_memory_access_ptr: *mut std::ffi::c_void,
        null_page_count: usize,
    ) -> *mut touchHLE_DynarmicWrapper;
    pub fn touchHLE_DynarmicWrapper_delete(cpu: *mut touchHLE_DynarmicWrapper);
    pub fn touchHLE_DynarmicWrapper_regs_const(cpu: *const touchHLE_DynarmicWrapper) -> *const u32;
    pub fn touchHLE_DynarmicWrapper_regs_mut(cpu: *mut touchHLE_DynarmicWrapper) -> *mut u32;
    pub fn touchHLE_DynarmicWrapper_cpsr(cpu: *const touchHLE_DynarmicWrapper) -> u32;
    pub fn touchHLE_DynarmicWrapper_set_cpsr(cpu: *mut touchHLE_DynarmicWrapper, cpsr: u32);
    pub fn touchHLE_DynarmicWrapper_swap_context(
        cpu: *mut touchHLE_DynarmicWrapper,
        context: *mut Dynarmic_A32_Context,
    );
    pub fn touchHLE_DynarmicWrapper_invalidate_cache_range(
        cpu: *mut touchHLE_DynarmicWrapper,
        start: VAddr,
        size: u32,
    );
    pub fn touchHLE_DynarmicWrapper_run_or_step(
        cpu: *mut touchHLE_DynarmicWrapper,
        mem: *mut touchHLE_Mem,
        ticks: Option<&mut u64>,
    ) -> i32;

    pub fn touchHLE_DynarmicWrapper_Context_new() -> *mut Dynarmic_A32_Context;
    pub fn touchHLE_DynarmicWrapper_Context_delete(context: *mut Dynarmic_A32_Context);
}
