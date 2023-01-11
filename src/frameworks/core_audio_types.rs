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

#[derive(Debug, Copy, Clone)]
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

/// Usually a FourCC.
pub type AudioFormatID = u32;
pub const kAudioFormatLinearPCM: AudioFormatID = fourcc(b"lpcm");

pub type AudioFormatFlags = u32;
pub const kAudioFormatFlagIsSignedInteger: AudioFormatFlags = 1 << 2;
