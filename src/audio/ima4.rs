/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Decoder for the Apple IMA4 ADPCM format (FourCC: `ima4`)
//!
//! Resources on IMA ADPCM in general:
//! - MultimediaWiki's [IMA ADPCM](https://wiki.multimedia.cx/index.php?title=IMA_ADPCM) page
//! - The IMA's _Recommended Practices for Enhancing Digital Audio Compatibility
//!   in Multimedia Systems_, which includes a reference decoding algorithm in C
//!   on pages 31 and 32.
//!   - [OCR'd PDF](http://www.cs.columbia.edu/~hgs/audio/dvi/IMA_ADPCM.pdf)
//!   - [Untouched scans](http://www.cs.columbia.edu/~hgs/audio/dvi/)
//!
//! Resources on Apple IMA4:
//! - MultimediaWiki's [Apple QuickTime IMA ADPCM](https://wiki.multimedia.cx/index.php?title=Apple_QuickTime_IMA_ADPCM) page
//! - Apple's [Technical Note TN1081: Understanding the Differences Between Apple and Windows IMA-ADPCM Compressed Sound Files](https://web.archive.org/web/20080705145411/http://developer.apple.com/technotes/tn/tn1081.html) (also available [here](https://developer.apple.com/library/archive/technotes/tn/tn1081.html))
//!
//! The implementation here generally follows the naming from the IMA reference
//! algorithm.

const INDEX_TABLE: &[i8] = &[-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

const STEP_SIZE_TABLE: &[u16] = &[
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

/// Decode a 34-byte IMA4 ADPCM packet to 16-bit signed integer PCM.
///
/// The packet is always a single channel. For stereo, the packets alternate
/// between left and right, such that the first packet is for the left channel
/// and every other packet is for the right channel.
pub fn decode_ima4(in_packet: &[u8; 34]) -> [i16; 64] {
    let mut out_packet = [0i16; 64];

    let header = u16::from_be_bytes(in_packet[0..2].try_into().unwrap());
    let mut index = ((header & 0x7f) as usize).min(STEP_SIZE_TABLE.len() - 1);
    let mut predicted_sample = ((header >> 7) << 7) as i16;
    let mut step_size = STEP_SIZE_TABLE[index];

    for (byte_idx, &byte) in in_packet[2..].iter().enumerate() {
        for nibble_idx in 0..2 {
            let nibble = (byte >> (nibble_idx * 4)) & 0xf;

            predicted_sample = {
                let mut difference = 0;
                if nibble & 4 != 0 {
                    difference += step_size;
                }
                if nibble & 2 != 0 {
                    difference += step_size >> 1;
                }
                if nibble & 1 != 0 {
                    difference += step_size >> 2;
                }
                difference += step_size >> 3;

                if nibble & 8 != 0 {
                    predicted_sample.saturating_sub_unsigned(difference)
                } else {
                    predicted_sample.saturating_add_unsigned(difference)
                }
            };

            out_packet[byte_idx * 2 + nibble_idx] = predicted_sample;
            index = index
                .saturating_add_signed(INDEX_TABLE[nibble as usize].into())
                .min(STEP_SIZE_TABLE.len() - 1);
            step_size = STEP_SIZE_TABLE[index];
        }
    }

    out_packet
}
