/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! This is separated out into its own package so that we can avoid rebuilding
//! dr_mp3 more often than necessary, and to improve build-time parallelism.

// Allow the crate to have a non-snake-case name (touchHLE).
// This also allows items in the crate to have non-snake-case names.
#![allow(non_snake_case)]

// See build.rs and lib.c
extern "C" {
    fn touchHLE_decode_mp3_to_pcm(
        data: *const u8,
        data_size: usize,
        channels: *mut u32,
        sample_rate: *mut u32,
        frame_count: *mut u64,
    ) -> *mut i16;
    fn touchHLE_free_decoded_mp3_pcm(samples: *mut i16);
}

/// PCM data decoded from an MP3 file.
pub struct Mp3DecodedToPcm {
    /// 16-bit little-endian PCM samples, grouped in frames (one sample per
    /// channel in each frame).
    pub bytes: Vec<u8>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u32,
}

#[allow(clippy::result_unit_err)]
pub fn decode_mp3_to_pcm(data: &[u8]) -> Result<Mp3DecodedToPcm, ()> {
    let mut channels = 0;
    let mut sample_rate = 0;
    let mut frame_count = 0;
    let samples_ptr = unsafe {
        touchHLE_decode_mp3_to_pcm(
            data.as_ptr(),
            data.len(),
            &mut channels,
            &mut sample_rate,
            &mut frame_count,
        )
    };
    if samples_ptr.is_null() {
        return Err(());
    }

    let bytes = unsafe {
        std::slice::from_raw_parts(
            samples_ptr as *const _,
            std::mem::size_of::<i16>() * (frame_count as usize) * (channels as usize),
        )
    }
    .to_vec();
    unsafe { touchHLE_free_decoded_mp3_pcm(samples_ptr) };

    Ok(Mp3DecodedToPcm {
        bytes,
        sample_rate,
        channels,
    })
}
