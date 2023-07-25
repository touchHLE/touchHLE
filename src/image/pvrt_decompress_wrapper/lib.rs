/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! This is separated out into its own package so that we can avoid rebuilding
//! PVRTDecompress more often than necessary, and to improve build-time
//! parallelism.

// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]

use std::ffi::c_void;

// See build.rs, lib.cpp and ../../../vendor/PVRTDecompress/PVRTDecompress.h
extern "C" {
    pub fn touchHLE_decompress_pvrtc(
        pvrtc_data: *const c_void,
        is_2bit: bool,
        width: u32,
        height: u32,
        rgba_data: *mut u8,
    ) -> u32;
}
