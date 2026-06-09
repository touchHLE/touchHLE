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
use symphonia::core::audio::AudioSpec;
use symphonia::core::codecs::audio::well_known::{
    CODEC_ID_AAC, CODEC_ID_ADPCM_IMA_QT, CODEC_ID_ADPCM_IMA_WAV, CODEC_ID_ALAC, CODEC_ID_MP3,
    CODEC_ID_PCM_S16LE,
};
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
    let mut probed = symphonia::default::get_probe()
        .probe(
            &Default::default(),
            mss,
            Default::default(),
            Default::default(),
        )
        .map_err(|_| ())?;

    let track = probed
        .tracks()
        .iter()
        .find(|t| {
            if let Some(codec_params) = &t.codec_params {
                if let Some(audio_codec_params) = codec_params.audio() {
                    audio_codec_params.codec == CODEC_ID_AAC
                        || audio_codec_params.codec == CODEC_ID_ADPCM_IMA_WAV
                        || audio_codec_params.codec == CODEC_ID_ADPCM_IMA_QT
                        || audio_codec_params.codec == CODEC_ID_ALAC
                        || audio_codec_params.codec == CODEC_ID_MP3
                        || audio_codec_params.codec == CODEC_ID_PCM_S16LE
                } else {
                    false
                }
            } else {
                false
            }
        })
        .ok_or(())?;
    let track_id = track.id;

    // Not sure why this would fail, maybe an unusual AAC track.
    let audio_codec_params = track.codec_params.as_ref().unwrap().audio().unwrap();
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(audio_codec_params, &Default::default())
        .map_err(|_| ())?;

    let mut out_pcm = Vec::<u8>::new();
    let mut audio_spec: Option<AudioSpec> = None;
    {
        let mut tmp_raw_s16_buf: Option<Vec<u8>> = None;
        loop {
            let packet = match probed.next_packet() {
                Ok(packet) => match packet {
                    Some(packet) => packet,
                    // "If Ok(None) is returned, the media has ended and
                    // no more packets will be produced until the reader
                    // is seeked to a new position."
                    None => break,
                },
                // Assume I/O errors can only mean end-of-file, because the
                // entire file is in-memory.
                Err(symphonia::core::errors::Error::IoError(_)) => break,
                Err(_) => return Err(()),
            };

            if packet.track_id != track_id {
                continue;
            }
            let Ok(decoded_packet) = decoder.decode(&packet) else {
                break;
            };

            // For some reason, the "audio spec" (number of channels etc)
            // is reported per-packet? This is weird because it must be the same
            // for all of them.
            let audio_spec = audio_spec.get_or_insert_with(|| decoded_packet.spec().clone());
            assert_eq!(audio_spec, decoded_packet.spec());

            // Note that this assumes every packet's buffer's capacity is the
            // same, which is a dubious assumption, but Symphonia's own example
            // code does it, so maybe it's fine?
            let tmp_raw_s16_buf = tmp_raw_s16_buf
                .get_or_insert_with(|| Vec::with_capacity(decoded_packet.capacity()));
            tmp_raw_s16_buf.clear();
            decoded_packet.copy_bytes_to_vec_interleaved_as::<i16>(tmp_raw_s16_buf);

            out_pcm.extend_from_slice(tmp_raw_s16_buf);
        }
    }
    let audio_spec = audio_spec.ok_or(())?;
    Ok(SymphoniaDecodedToPcm {
        bytes: out_pcm,
        sample_rate: audio_spec.rate(),
        channels: audio_spec.channels().count().try_into().unwrap(),
    })
}
