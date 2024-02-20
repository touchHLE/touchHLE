/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Core Audio Types framework. (Yes, it's not part of Core Audio?)

use crate::mem::SafeRead;

// The audio frameworks love FourCC's, and we currently don't need these
// anywhere else, so this is as good a place to put this as any.
/// Get the 32-bit integer corresponding to a FourCC. Where you'd write
/// `'e.g.'` in C, write `fourcc(b"e.g.")` in Rust.
pub const fn fourcc(fourcc: &[u8; 4]) -> u32 {
    u32::from_be_bytes(*fourcc)
}
/// Display a FourCC appropriately for debugging.
pub fn debug_fourcc(fourcc: u32) -> String {
    if let Ok(utf8) = std::str::from_utf8(&fourcc.to_be_bytes()) {
        format!("'{}'", utf8)
    } else {
        format!("{:#x}", fourcc)
    }
}

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct AudioStreamBasicDescription {
    // Hz
    pub sample_rate: f64,
    pub format_id: AudioFormatID,
    pub format_flags: AudioFormatFlags,
    pub bytes_per_packet: u32,
    pub frames_per_packet: u32,
    pub bytes_per_frame: u32,
    pub channels_per_frame: u32,
    pub bits_per_channel: u32,
    pub _reserved: u32,
}
unsafe impl SafeRead for AudioStreamBasicDescription {}
impl std::fmt::Debug for AudioStreamBasicDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let &AudioStreamBasicDescription {
            sample_rate,
            format_id,
            format_flags,
            bytes_per_packet,
            frames_per_packet,
            bytes_per_frame,
            channels_per_frame,
            bits_per_channel,
            _reserved,
        } = self;
        f.debug_struct("AudioStreamBasicDescription")
            .field("sample_rate", &sample_rate)
            .field("format_id", &debug_fourcc(format_id))
            .field("format_flags", &{
                let mut flags = Vec::new();
                if (format_flags & kAudioFormatFlagIsFloat) != 0 {
                    flags.push("kAudioFormatFlagIsFloat");
                }
                if (format_flags & kAudioFormatFlagIsBigEndian) != 0 {
                    flags.push("kAudioFormatFlagIsBigEndian");
                }
                if (format_flags & kAudioFormatFlagIsSignedInteger) != 0 {
                    flags.push("kAudioFormatFlagIsSignedInteger");
                }
                if (format_flags & kAudioFormatFlagIsPacked) != 0 {
                    flags.push("kAudioFormatFlagIsPacked");
                }
                flags
            })
            .field("bytes_per_packet", &bytes_per_packet)
            .field("frames_per_packet", &frames_per_packet)
            .field("bytes_per_frame", &bytes_per_frame)
            .field("channels_per_frame", &channels_per_frame)
            .field("bits_per_channel", &bits_per_channel)
            .finish()
    }
}

/// Usually a FourCC.
pub type AudioFormatID = u32;
pub const kAudioFormatLinearPCM: AudioFormatID = fourcc(b"lpcm");
pub const kAudioFormatAppleIMA4: AudioFormatID = fourcc(b"ima4");

pub type AudioFormatFlags = u32;
pub const kAudioFormatFlagIsFloat: AudioFormatFlags = 1 << 0;
pub const kAudioFormatFlagIsBigEndian: AudioFormatFlags = 1 << 1;
pub const kAudioFormatFlagIsSignedInteger: AudioFormatFlags = 1 << 2;
pub const kAudioFormatFlagIsPacked: AudioFormatFlags = 1 << 3;
pub const kAudioFormatFlagIsAlignedHigh: AudioFormatFlags = 1 << 4;
