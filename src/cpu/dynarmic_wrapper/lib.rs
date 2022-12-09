//! This is separated out into its own package so that we can avoid rebuilding
//! dynarmic more often than necessary, and to improve build-time parallelism.

// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]

/// Opaque type from C
#[allow(non_camel_case_types)]
pub type touchHLE_DynarmicWrapper = std::ffi::c_void;
/// Opaque type from Rust (this is the `Memory` type from the main crate, but
/// `c_void` is used here to avoid depending on it directly)
#[allow(non_camel_case_types)]
pub type touchHLE_Memory = std::ffi::c_void;

// Import functions from lib.cpp, see build.rs. Note that lib.cpp depends on
// some functions being exported from Rust, but those are in the main crate.
extern "C" {
    pub fn touchHLE_DynarmicWrapper_new() -> *mut touchHLE_DynarmicWrapper;
    pub fn touchHLE_DynarmicWrapper_delete(cpu: *mut touchHLE_DynarmicWrapper);
    pub fn touchHLE_DynarmicWrapper_regs_const(cpu: *const touchHLE_DynarmicWrapper) -> *const u32;
    pub fn touchHLE_DynarmicWrapper_regs_mut(cpu: *mut touchHLE_DynarmicWrapper) -> *mut u32;
    pub fn touchHLE_DynarmicWrapper_run(
        cpu: *mut touchHLE_DynarmicWrapper,
        mem: *mut touchHLE_Memory,
        ticks: *mut u64,
    ) -> i32;
}
