//! This is separated out into its own package so that we can avoid rebuilding
//! dynarmic more often than necessary, and to improve build-time parallelism.

// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]

// See build.rs and lib.cpp
extern "C" {
    pub fn test_cpu_by_adding_numbers(a: i32, b: i32) -> i32;
}
