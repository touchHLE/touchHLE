/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! This is separated out into its own package so that we can avoid rebuilding
//! stb_image more often than necessary, and to improve build-time parallelism.

// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]

use std::ffi::{c_char, c_int, c_uchar, c_void};

// See build.rs, lib.c and ../../../vendor/stb/stb_image.h
extern "C" {
    pub fn stbi_convert_iphone_png_to_rgb(flag_true_if_should_convert: c_int);
    pub fn stbi_set_unpremultiply_on_load(flag_true_if_should_unpremultiply: c_int);
    pub fn stbi_load_from_memory(
        buffer: *const c_uchar,
        len: c_int,
        x: *mut c_int,
        y: *mut c_int,
        channels_in_file: *mut c_int,
        desired_channels: c_int,
    ) -> *mut c_uchar;
    pub fn stbi_image_free(retval_from_stbi_load: *mut c_void);
    pub fn stbi_failure_reason() -> *const c_char;
}
