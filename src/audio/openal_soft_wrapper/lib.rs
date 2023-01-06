//! This is separated out into its own package so that we can avoid rebuilding
//! OpenAL Soft more often than necessary, and to improve build-time
//! parallelism. This package **only** exists to build OpenAL Soft, it exports
//! no Rust symbols at all! The bindings are found in the main crate.

// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]
