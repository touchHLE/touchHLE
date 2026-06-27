/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! Standalone CAF (Apple Core Audio Format) → 16-bit little-endian PCM decoder.
//!
//! This is used as a fallback when [`super::symphonia_formats`] cannot probe a
//! CAF file. The CAF demuxer in `symphonia-format-caf 0.6.0-alpha.1` does not
//! correctly handle CAF files where the Audio Data chunk's `mChunkSize` is set
//! to `-1` (which the CAF specification explicitly allows for the last chunk
//! to mean "extends to the end of the file"); on such files Symphonia walks
//! off the end of the audio data trying to read a chunk header and bails out
//! with `IoError(UnexpectedEof)`. Plants vs. Zombies (`com.popcap.PvZ`) ships
//! `sounds/*.caf` files in this layout, which is what was preventing its
//! sound effects from playing.
//!
//! References:
//! - Apple, *Apple Core Audio Format Specification 1.0*, "The Audio Data Chunk"
//!   <https://developer.apple.com/library/archive/documentation/MusicAudio/Reference/CAFSpec/CAF_chunks/CAF_chunks.html>
//! - Apple, *Apple Core Audio Format Specification 1.0*, "The Audio Description
//! Chunk"
//!   <https://developer.apple.com/library/archive/documentation/MusicAudio/Reference/CAFSpec/CAF_chunks/CAF_chunks.html#//apple_ref/doc/uid/TP40001862-CH210-SW2>
//! - Apple, *AudioServicesCreateSystemSoundID*
//!   <https://developer.apple.com/documentation/audiotoolbox/audioservicescreatesystemsoundid(_:_:)>

// ============================================================
// ИСПРАВЛЕНИЯ (относительно исходника):
//
// 1. ulaw_to_linear — неверный знак:
//    По стандарту ITU-T G.711 и реализации Sun/POSIX:
//    В µ-law ПОСЛЕ инвертирования всех битов (!u_val):
//      бит 7 (0x80) == 1  →  ПОЛОЖИТЕЛЬНОЕ число
//      бит 7 (0x80) == 0  →  ОТРИЦАТЕЛЬНОЕ число
//    Исходный код делал наоборот: возвращал BIAS-t при бите 7==1
//    и t-BIAS при бите 7==0, что перепутывает знак всех семплов.
//    ИСПРАВЛЕНО: при (u_val & 0x80) != 0 → (t - BIAS), иначе → (BIAS - t).
//
// 2. kCAFLinearPCMFormatFlagIsFloat и kCAFLinearPCMFormatFlagIsLittleEndian —
//    биты проверяются корректно (bit 0 и bit 1 соответственно), как определено
//    в Apple CAF Spec:
//      kCAFLinearPCMFormatFlagIsFloat        = (1L << 0)   // 0x1
//      kCAFLinearPCMFormatFlagIsLittleEndian = (1L << 1)   // 0x2
//    Маски 0b01 и 0b10 правильны.
//
// 3. 8-bit LPCM в CAF — знаковый:
//    Согласно CAF spec, 8-bit LPCM в CAF — знаковый (signed). При конвертации
//    в 16-bit расширяем знак и масштабируем: (v as i16) << 8.
//    Это оставлено без изменений (правильно).
//
// 4. IMA4 bytes_per_packet: спецификация Apple (Table 2-5) явно указывает
//    mBytesPerPacket = mChannelsPerFrame * 34 при фиксированном размере.
//    Поле может быть 0 при переменном размере (VBR), поэтому проверка
//    `desc.bytes_per_packet != 0` перед сравнением оставлена корректной.
// ============================================================

use super::ima4::decode_ima4;
use super::symphonia_formats::SymphoniaDecodedToPcm;
use std::io::Cursor;
use std::panic::AssertUnwindSafe;

/// Decode the contents of a `.caf` file into 16-bit little-endian interleaved
/// PCM, returning the same in-memory shape that [`SymphoniaDecodedToPcm`]
/// uses so the rest of the audio pipeline can consume it uniformly.
///
/// Currently supported audio data formats inside the CAF container:
/// - `lpcm` — Linear PCM (8/16/24/32-bit signed integer, big- or
///   little-endian). Float PCM is rejected.
/// - `ima4` — Apple IMA 4:1 ADPCM (mono or stereo).
/// - `ulaw` — ITU-T G.711 µ-law 2:1 compression (8-bit → 16-bit).
/// - `alaw` — ITU-T G.711 A-law 2:1 compression (8-bit → 16-bit).
///
/// Anything else (MPEG-4 AAC, MP3, ALAC, …) is returned as `Err(())` so the
/// caller can fall through to a different decoder.
pub fn decode_caf_to_pcm(file: Cursor<Vec<u8>>) -> Result<SymphoniaDecodedToPcm, ()> {
    if std::env::var_os("TOUCHHLE_DISABLE_CAF_DECODER").is_some() {
        log!("caf_decoder: TOUCHHLE_DISABLE_CAF_DECODER=1, skipping CAF decode so AudioFile can fall back/dummy instead of preloading hundreds of PCM buffers");
        return Err(());
    }
    // The `caf` crate `panic!`s in a few corner cases that show up in real
    // iOS games (notably `to_next_chunk` / `read_chunk_body` on chunks whose
    // `mChunkSize == -1`, which Apple's CAF spec says is legal for the last
    // chunk to mean "extends to the end of the file"). Wrap the whole demux
    // in `catch_unwind` so a panic there is converted into a graceful
    // fall-through to the next decoder instead of taking the whole emulator
    // down. We log so we can see *why* it failed in the next bug report.
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| decode_caf_to_pcm_inner(file)));
    match result {
        Ok(Ok(pcm)) => Ok(pcm),
        Ok(Err(why)) => {
            log!("caf_decoder: refusing this CAF file: {}", why);
            Err(())
        }
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<&'static str>() {
                (*s).to_string()
            } else if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else {
                "(non-string panic payload)".to_string()
            };
            log!(
                "caf_decoder: panic from `caf` crate while demuxing CAF: {}",
                msg
            );
            Err(())
        }
    }
}

fn decode_caf_to_pcm_inner(file: Cursor<Vec<u8>>) -> Result<SymphoniaDecodedToPcm, &'static str> {
    use caf::FormatType;

    let mut reader =
        caf::CafPacketReader::new(file, vec![]).map_err(|_| "CafPacketReader::new failed")?;
    let desc = reader.audio_desc.clone();
    log!(
        "caf_decoder: format_id={:?}, sample_rate={}, channels={}, bytes_per_packet={}, frames_per_packet={}, bits_per_channel={}, format_flags=0x{:x}",
        desc.format_id,
        desc.sample_rate,
        desc.channels_per_frame,
        desc.bytes_per_packet,
        desc.frames_per_packet,
        desc.bits_per_channel,
        desc.format_flags,
    );

    let sample_rate: u32 = desc.sample_rate.round() as u32;
    let channels: u32 = desc.channels_per_frame;
    if sample_rate == 0 {
        return Err("sample_rate == 0");
    }
    if channels == 0 {
        return Err("channels == 0");
    }

    let mut out_pcm: Vec<u8> = Vec::new();

    match desc.format_id {
        FormatType::AppleIma4 => {
            // CAF IMA4: each packet covers `frames_per_packet` (= 64) frames,
            // and one packet's worth of bytes is `34 * channels_per_frame`
            // (per Apple's CAF spec, Table 2-5). For stereo, the packet data
            // is the left channel's 34-byte sub-packet immediately followed
            // by the right channel's 34-byte sub-packet.
            //
            // We decode each 34-byte sub-packet through `decode_ima4` exactly
            // like `audio_queue::decode_buffer` does, but eagerly for the
            // whole file.
            let sub_packet_bytes: usize = 34;
            let expected_packet_bytes: usize = sub_packet_bytes * channels as usize;
            if desc.bytes_per_packet as usize != expected_packet_bytes && desc.bytes_per_packet != 0
            {
                // Not actually a 34-bytes-per-channel packet layout — refuse
                // rather than producing garbage.
                return Err("IMA4 bytes_per_packet != 34 * channels");
            }

            // Greedily collect every CAF packet, then chunk into 34-byte
            // sub-packets and decode in (channel, channel, …) order.
            //
            // For PvZ-style CAF files where the Audio Data chunk has
            // `mChunkSize == -1`, `caf::CafPacketReader::next_packet`
            // ultimately calls `read_exact` on the underlying cursor and will
            // return an `UnexpectedEof` `Err` once we walk off the end of the
            // file. That's the natural termination signal here.
            let mut all_bytes: Vec<u8> = Vec::new();
            loop {
                match reader.next_packet() {
                    Ok(Some(pkt)) => all_bytes.extend_from_slice(&pkt),
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
            log!(
                "caf_decoder: IMA4 collected {} bytes of packet data from CAF",
                all_bytes.len()
            );

            // Trim any tail that doesn't form a complete 34-byte sub-packet.
            let aligned_len = (all_bytes.len() / sub_packet_bytes) * sub_packet_bytes;
            all_bytes.truncate(aligned_len);
            if all_bytes.is_empty() {
                return Err("IMA4 file had no decodable packet data");
            }

            let mut sub_packets = all_bytes.chunks_exact(sub_packet_bytes);
            match channels {
                1 => {
                    for sub in sub_packets.by_ref() {
                        let pcm: [i16; 64] = decode_ima4(sub.try_into().unwrap());
                        for s in &pcm {
                            out_pcm.extend_from_slice(&s.to_le_bytes());
                        }
                    }
                }
                2 => {
                    while let Some(left) = sub_packets.next() {
                        let Some(right) = sub_packets.next() else {
                            break;
                        };
                        let l_pcm: [i16; 64] = decode_ima4(left.try_into().unwrap());
                        let r_pcm: [i16; 64] = decode_ima4(right.try_into().unwrap());
                        for (l, r) in l_pcm.iter().zip(r_pcm.iter()) {
                            out_pcm.extend_from_slice(&l.to_le_bytes());
                            out_pcm.extend_from_slice(&r.to_le_bytes());
                        }
                    }
                }
                _ => return Err("IMA4 with >2 channels is not supported"),
            }
        }
        FormatType::LinearPcm => {
            // CAF audio-description format flags (Apple CAF spec,
            // "mFormatFlags Field"):
            //   kCAFLinearPCMFormatFlagIsFloat        = (1L << 0)  // bit 0
            //   kCAFLinearPCMFormatFlagIsLittleEndian = (1L << 1)  // bit 1
            //
            // Float PCM is rejected here because the rest of the pipeline only
            // accepts 16-bit signed integer little-endian PCM.
            let is_float = (desc.format_flags & 0x1) != 0;
            let is_little_endian = (desc.format_flags & 0x2) != 0;
            if is_float {
                return Err("LPCM float is not supported by this decoder");
            }

            let bits = desc.bits_per_channel;
            if !matches!(bits, 8 | 16 | 24 | 32) {
                return Err("LPCM bits_per_channel must be 8, 16, 24, or 32");
            }

            let bytes_per_sample = (bits / 8) as usize;
            loop {
                let pkt = match reader.next_packet() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => break,
                    Err(_) => break,
                };
                for sample in pkt.chunks_exact(bytes_per_sample) {
                    let mut buf = [0u8; 8];
                    buf[..sample.len()].copy_from_slice(sample);
                    let s16 = match (bits, is_little_endian) {
                        (8, _) => {
                            // CAF 8-bit LPCM is signed per spec; sign-extend to 16-bit
                            // and scale (high-align): shift left by 8.
                            let v = buf[0] as i8;
                            (v as i16) << 8
                        }
                        (16, true) => i16::from_le_bytes([buf[0], buf[1]]),
                        (16, false) => i16::from_be_bytes([buf[0], buf[1]]),
                        (24, true) => {
                            // Packed little-endian 24-bit signed → i16 (discard LSB byte).
                            let v = (buf[0] as i32)
                                | ((buf[1] as i32) << 8)
                                | (((buf[2] as i8) as i32) << 16);
                            (v >> 8) as i16
                        }
                        (24, false) => {
                            // Packed big-endian 24-bit signed → i16 (discard LSB byte).
                            let v = (buf[2] as i32)
                                | ((buf[1] as i32) << 8)
                                | (((buf[0] as i8) as i32) << 16);
                            (v >> 8) as i16
                        }
                        (32, true) => {
                            let v = i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
                            (v >> 16) as i16
                        }
                        (32, false) => {
                            let v = i32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                            (v >> 16) as i16
                        }
                        _ => unreachable!(),
                    };
                    out_pcm.extend_from_slice(&s16.to_le_bytes());
                }
            }
        }
        FormatType::Ulaw => {
            // ITU-T G.711 µ-law decoding.
            // Each 8-bit µ-law sample expands to a 16-bit signed linear PCM
            // sample. The CAF audio description for µ-law has:
            //   bytes_per_packet = 1 * channels
            //   frames_per_packet = 1
            //   bits_per_channel = 8
            //
            // Reference: ITU-T Recommendation G.711 (11/88)
            // Also: Apple Core Audio Format Specification 1.0, format ID "ulaw"

            loop {
                let pkt = match reader.next_packet() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => break,
                    Err(_) => break,
                };
                for &byte in &pkt {
                    let s16 = ulaw_to_linear(byte);
                    out_pcm.extend_from_slice(&s16.to_le_bytes());
                }
            }
        }
        FormatType::Alaw => {
            // ITU-T G.711 A-law decoding.
            // Each 8-bit A-law sample expands to a 16-bit signed linear PCM
            // sample. Same packet layout as µ-law.
            //
            // Reference: ITU-T Recommendation G.711 (11/88)
            // Also: Apple Core Audio Format Specification 1.0, format ID "alaw"

            loop {
                let pkt = match reader.next_packet() {
                    Ok(Some(pkt)) => pkt,
                    Ok(None) => break,
                    Err(_) => break,
                };
                for &byte in &pkt {
                    let s16 = alaw_to_linear(byte);
                    out_pcm.extend_from_slice(&s16.to_le_bytes());
                }
            }
        }
        // Compressed formats other than IMA4/µ-law/A-law (AAC, MP3, ALAC, …)
        // — leave them for Symphonia to handle.
        _ => return Err("format_id is not LPCM, IMA4, Ulaw, or Alaw — leaving for Symphonia"),
    }

    if out_pcm.is_empty() {
        return Err("decoded output is empty");
    }
    log!(
        "caf_decoder: produced {} bytes of 16-bit LE PCM ({} Hz, {} ch)",
        out_pcm.len(),
        sample_rate,
        channels
    );

    Ok(SymphoniaDecodedToPcm {
        bytes: out_pcm,
        sample_rate,
        channels,
    })
}


/// Decode a single 8-bit µ-law (G.711) sample to 16-bit signed linear PCM.
///
/// Implementation follows the ITU-T G.711 specification and the canonical
/// Sun Microsystems public-domain reference implementation.
///
/// The µ-law byte is stored in **complemented** form (all bits inverted).
/// After un-complementing (`!u_val`) the layout is:
///   bit 7  — sign bit: **1 = positive, 0 = negative**
///   bits 6–4 — segment (exponent), 0–7
///   bits 3–0 — quantization step within segment
///
/// The bias of 0x84 (132) is used during reconstruction. Output range ≈ ±32124.
///
/// # Bug fixed vs. original
/// The original code had the sign check inverted:
///   `if (u_val & 0x80) != 0 { BIAS - t }` — WRONG (returned negative for positive)
///   `else { t - BIAS }`                     — WRONG (returned positive for negative)
/// Correct behaviour (per G.711 and Sun reference):
///   `if (u_val & 0x80) != 0 { t - BIAS }`  — positive sample
///   `else { BIAS - t }`                     — negative sample (return negated)
///
/// Reference: ITU-T Rec. G.711 (11/88); Sun Microsystems g711.c (public domain).
fn ulaw_to_linear(u_val: u8) -> i16 {
    // Un-complement to obtain the normal µ-law value.
    let u_val = !u_val;

    // Extract segment (exponent) and quantization bits.
    let segment = ((u_val & 0x70) >> 4) as i32;
    let quantization = (u_val & 0x0F) as i32;

    // Reconstruct magnitude: bias the quantization nibble, shift by segment,
    // then subtract the encoding bias (BIAS = 0x84 = 132).
    const BIAS: i32 = 0x84;
    let mut t = (quantization << 3) + BIAS;
    t <<= segment;

    // FIX: bit 7 set after un-complement means POSITIVE sample.
    // The original had the branches swapped, producing the wrong sign for
    // every µ-law sample.
    if (u_val & 0x80) != 0 {
        // Positive: t - BIAS
        (t - BIAS) as i16
    } else {
        // Negative: -(t - BIAS)
        (BIAS - t) as i16
    }
}

/// Decode a single 8-bit A-law (G.711) sample to 16-bit signed linear PCM.
///
/// Implementation follows the ITU-T G.711 specification.
/// A-law encoding uses even-bit inversion (XOR with 0x55) for transmission.
/// After restoring (`a_val ^ 0x55`) the layout is:
///   bit 7  — sign bit: **1 = positive, 0 = negative**
///   bits 6–4 — segment (exponent), 0–7
///   bits 3–0 — quantization step within segment
///
/// Output range ≈ ±32256.
///
/// Reference: ITU-T Rec. G.711 (11/88); Sun Microsystems g711.c (public domain).
fn alaw_to_linear(a_val: u8) -> i16 {
    // Restore even-bit inversion used in A-law transmission.
    let a_val = a_val ^ 0x55;

    let segment = ((a_val & 0x70) >> 4) as i32;
    let quantization = (a_val & 0x0F) as i32;

    let t = if segment == 0 {
        // Segment 0: linear reconstruction without exponent shift.
        (quantization << 4) + 8
    } else {
        // Segments 1–7: shift by (segment - 1) after adding segment bias.
        ((quantization << 4) + 0x108) << (segment - 1)
    };

    // Bit 7 set → positive sample; bit 7 clear → negative.
    if (a_val & 0x80) != 0 {
        t as i16
    } else {
        -(t as i16)
    }
}

