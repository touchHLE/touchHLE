/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Quick-and-dirty decoding of miscellaneous formats (MP3, AAC) to linear PCM.
//!
//! This should be the only module in touchHLE that makes use of [symphonia].
//! For AAC, Only the LC profile and MPEG-4 container format are supported (see
//! feature list in Cargo.toml).

use std::io::Cursor;
use symphonia::core::audio::{RawSampleBuffer, SignalSpec};
use symphonia::core::codecs::{CODEC_TYPE_AAC, CODEC_TYPE_MP3};
use symphonia::core::io::MediaSourceStream;

/// PCM data decoded from an miscellaneous format file.
pub struct SymphoniaDecodedToPcm {
    /// 16-bit little-endian PCM samples, grouped in frames (one sample per
    /// channel in each frame).
    pub bytes: Vec<u8>,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Channel count.
    pub channels: u32,
}

pub fn decode_symphonia_to_pcm(file: Cursor<Vec<u8>>) -> Result<SymphoniaDecodedToPcm, ()> {
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // If this failed, the container format is not supported.
    let probed = symphonia::default::get_probe()
        .format(
            &Default::default(),
            mss,
            &Default::default(),
            &Default::default(),
        )
        .map_err(|_| ())?;

    // If this failed, no audio track with a relevant format was found.
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec == CODEC_TYPE_AAC || t.codec_params.codec == CODEC_TYPE_MP3)
        .ok_or(())?;
    let track_id = track.id;

    // Not sure why this would fail, maybe an unusual AAC track.
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &Default::default())
        .map_err(|_| ())?;

    let mut out_pcm = Vec::<u8>::new();
    let mut signal_spec: Option<SignalSpec> = None;
    {
        let mut tmp_raw_s16_buf: Option<RawSampleBuffer<i16>> = None;
        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                // Assume I/O errors can only mean end-of-file, because the
                // entire file is in-memory.
                Err(symphonia::core::errors::Error::IoError(_)) => break,
                Err(_) => return Err(()),
            };

            if packet.track_id() != track_id {
                continue;
            }
            let Ok(decoded_packet) = decoder.decode(&packet) else {
                break;
            };

            // For some reason, the "signal spec" (number of channels etc)
            // is reported per-packet? This is weird because it must be the same
            // for all of them.
            let signal_spec = *signal_spec.get_or_insert_with(|| *decoded_packet.spec());
            assert_eq!(signal_spec, *decoded_packet.spec());

            // Note that this assumes every packet's buffer's capacity is the
            // same, which is a dubious assumption, but Symphonia's own example
            // code does it, so maybe it's fine?
            let tmp_raw_s16_buf = tmp_raw_s16_buf.get_or_insert_with(|| {
                RawSampleBuffer::new(decoded_packet.capacity() as _, signal_spec)
            });
            tmp_raw_s16_buf.clear();
            tmp_raw_s16_buf.copy_interleaved_ref(decoded_packet);

            out_pcm.extend_from_slice(tmp_raw_s16_buf.as_bytes());
        }
    }
    let signal_spec = signal_spec.ok_or(())?;
    Ok(SymphoniaDecodedToPcm {
        bytes: out_pcm,
        sample_rate: signal_spec.rate,
        channels: signal_spec.channels.count().try_into().unwrap(),
    })
}
