/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Audio file decoding and OpenAL bindings.
//!
//! The audio file decoding support is an abstraction over various libraries
//! (currently [caf], [hound], and dr_mp3), usage of which should be confined to
//! this module.
//!
//! Resources:
//! - [Apple Core Audio Format Specification 1.0](https://developer.apple.com/library/archive/documentation/MusicAudio/Reference/CAFSpec/CAF_intro/CAF_intro.html)

mod ima4;

pub use ima4::decode_ima4;
use openal::alc_types::ALCcontext;
use touchHLE_dr_mp3_wrapper as dr_mp3;
pub use touchHLE_openal_soft_wrapper as openal;

use crate::audio;
use crate::frameworks::core_audio_types::{
    kAudioFormatAppleIMA4, kAudioFormatFlagIsBigEndian, kAudioFormatFlagIsFloat,
    kAudioFormatFlagIsPacked, kAudioFormatFlagIsSignedInteger, kAudioFormatLinearPCM,
    AudioStreamBasicDescription,
};
use crate::fs::{Fs, GuestPath};
use std::io::Cursor;

#[derive(Debug)]
pub enum AudioFormat {
    LinearPcm {
        is_float: bool,
        is_little_endian: bool,
    },
    AppleIma4,
}
/// Fields have the same meanings as in the Core Audio Format's
/// Audio Description chunk, which is in turn similar to Core Audio Types'
/// `AudioStreamBasicDescription`.
#[derive(Debug)]
pub struct AudioDescription {
    /// Hz
    pub sample_rate: f64,
    pub format: AudioFormat,
    pub bytes_per_packet: u32,
    pub frames_per_packet: u32,
    pub channels_per_frame: u32,
    pub bits_per_channel: u32,
}

impl AudioDescription {
    pub fn into_basic_description(self) -> AudioStreamBasicDescription {
        let audio::AudioDescription {
            sample_rate,
            format,
            bytes_per_packet,
            frames_per_packet,
            channels_per_frame,
            bits_per_channel,
        } = self;

        match format {
            audio::AudioFormat::LinearPcm {
                is_float,
                is_little_endian,
            } => {
                let is_packed = (bits_per_channel * channels_per_frame * frames_per_packet)
                    == (bytes_per_packet * 8);
                let format_flags = (u32::from(is_float) * kAudioFormatFlagIsFloat)
                    | (u32::from((!is_float) && matches!(bits_per_channel, 16 | 24))
                        * kAudioFormatFlagIsSignedInteger)
                    | (u32::from(is_packed) * kAudioFormatFlagIsPacked)
                    | (u32::from(!is_little_endian) * kAudioFormatFlagIsBigEndian);
                AudioStreamBasicDescription {
                    sample_rate,
                    format_id: kAudioFormatLinearPCM,
                    format_flags,
                    bytes_per_packet,
                    frames_per_packet,
                    bytes_per_frame: bytes_per_packet / frames_per_packet,
                    channels_per_frame,
                    bits_per_channel,
                    _reserved: 0,
                }
            }
            audio::AudioFormat::AppleIma4 => {
                AudioStreamBasicDescription {
                    sample_rate,
                    format_id: kAudioFormatAppleIMA4,
                    format_flags: 0,
                    bytes_per_packet,
                    frames_per_packet,
                    bytes_per_frame: 0, // compressed
                    channels_per_frame,
                    bits_per_channel,
                    _reserved: 0,
                }
            }
        }
    }
}

pub struct AudioFile(AudioFileInner);
enum AudioFileInner {
    Wave(hound::WavReader<Cursor<Vec<u8>>>),
    Caf(caf::CafPacketReader<Cursor<Vec<u8>>>),
    Mp3(dr_mp3::Mp3DecodedToPcm),
}

impl AudioFile {
    pub fn open_for_reading<P: AsRef<GuestPath>>(path: P, fs: &Fs) -> Result<Self, ()> {
        // TODO: it would be better not to load the whole file at once
        let bytes = fs.read(path.as_ref())?;

        // Both WavReader::new() and CafPacketReader::new() consume the reader
        // (in this case, a Cursor) passed to them. This is a bit annoying
        // considering we don't know which is appropriate for the file without
        // trying both. This is worked around here by using temporary readers
        // for checking if the file is the supported format, then recreating the
        // reader if that works.

        if hound::WavReader::new(Cursor::new(&bytes)).is_ok() {
            let reader = hound::WavReader::new(Cursor::new(bytes)).unwrap();
            Ok(AudioFile(AudioFileInner::Wave(reader)))
        } else if caf::CafPacketReader::new(Cursor::new(&bytes), vec![]).is_ok() {
            let reader = caf::CafPacketReader::new(Cursor::new(bytes), vec![]).unwrap();
            Ok(AudioFile(AudioFileInner::Caf(reader)))
        // TODO: Real MP3 container handling. Currently we are immediately
        // decoding the entire file to PCM and acting as if it's a PCM file,
        // simply because because this is easier. Full MP3 support would require
        // a lot of changes in Audio Toolbox.
        } else if let Ok(pcm) = dr_mp3::decode_mp3_to_pcm(&bytes) {
            Ok(AudioFile(AudioFileInner::Mp3(pcm)))
        } else {
            log!(
                "Could not decode audio file at path {:?}, likely an unimplemented file format.",
                path.as_ref()
            );
            Err(())
        }
    }

    pub fn audio_description(&self) -> AudioDescription {
        match self.0 {
            AudioFileInner::Wave(ref wave_reader) => {
                let hound::WavSpec {
                    channels,
                    sample_rate,
                    bits_per_sample,
                    sample_format,
                } = wave_reader.spec();
                // Hound supports unsigned 8-bit, signed 16-bit, signed 24-bit
                // and floating-point 32-bit linear PCM. We should expose all of
                // these eventually, but we should only expose formats we've
                // tested.
                assert!(matches!(bits_per_sample, 8 | 16));
                assert!(sample_format == hound::SampleFormat::Int);

                AudioDescription {
                    sample_rate: sample_rate.into(),
                    format: AudioFormat::LinearPcm {
                        is_float: false,
                        is_little_endian: true,
                    },
                    bytes_per_packet: u32::from(channels * bits_per_sample / 8),
                    frames_per_packet: 1,
                    channels_per_frame: channels.into(),
                    bits_per_channel: bits_per_sample as u32,
                }
            }
            AudioFileInner::Caf(ref caf_reader) => {
                let caf::chunks::AudioDescription {
                    sample_rate,
                    ref format_id,
                    format_flags,
                    bytes_per_packet,
                    frames_per_packet,
                    channels_per_frame,
                    bits_per_channel,
                } = caf_reader.audio_desc;

                AudioDescription {
                    sample_rate,
                    format: match format_id {
                        caf::FormatType::LinearPcm => {
                            assert!((format_flags & !3) == 0);
                            let is_float = (format_flags & 1) == 1;
                            let is_little_endian = (format_flags & 2) == 2;
                            AudioFormat::LinearPcm {
                                is_float,
                                is_little_endian,
                            }
                        }
                        caf::FormatType::AppleIma4 => {
                            assert!(format_flags == 0);
                            AudioFormat::AppleIma4
                        }
                        //
                        // We should expose all of the formats eventually, but
                        // the others haven't been tested yet.
                        _ => panic!("{:?} not supported yet", format_id),
                    },
                    bytes_per_packet,
                    frames_per_packet,
                    channels_per_frame,
                    bits_per_channel,
                }
            }
            AudioFileInner::Mp3(dr_mp3::Mp3DecodedToPcm {
                sample_rate,
                channels,
                ..
            }) => AudioDescription {
                sample_rate: f64::from(sample_rate),
                format: AudioFormat::LinearPcm {
                    is_float: false,
                    is_little_endian: true,
                },
                bytes_per_packet: channels * 2,
                frames_per_packet: 1,
                channels_per_frame: channels,
                bits_per_channel: 16,
            },
        }
    }

    fn bytes_per_sample(&self) -> u64 {
        let AudioDescription {
            format,
            bytes_per_packet,
            frames_per_packet,
            channels_per_frame,
            ..
        } = self.audio_description();
        if !matches!(format, AudioFormat::LinearPcm { .. }) {
            panic!("{:?} is a compressed format!", format);
        }
        ((bytes_per_packet / frames_per_packet) / channels_per_frame).into()
    }

    pub fn byte_count(&self) -> u64 {
        match self.0 {
            AudioFileInner::Wave(ref wave_reader) => {
                let sample_count = wave_reader.len(); // position-independent
                u64::from(sample_count) * self.bytes_per_sample()
            }
            AudioFileInner::Caf(_) => {
                // variable size not implemented
                u64::from(self.packet_size_fixed()) * self.packet_count()
            }
            AudioFileInner::Mp3(dr_mp3::Mp3DecodedToPcm { ref bytes, .. }) => bytes.len() as u64,
        }
    }

    pub fn packet_count(&self) -> u64 {
        match self.0 {
            AudioFileInner::Wave(_) | AudioFileInner::Mp3(dr_mp3::Mp3DecodedToPcm { .. }) => {
                // never variable-size
                self.byte_count() / u64::from(self.packet_size_fixed())
            }
            AudioFileInner::Caf(ref caf_reader) => {
                caf_reader.get_packet_count().unwrap().try_into().unwrap()
            }
        }
    }

    /// Returns the packet size if this audio format has a constant packet size,
    /// panics if not.
    pub fn packet_size_fixed(&self) -> u32 {
        let AudioDescription {
            bytes_per_packet, ..
        } = self.audio_description();
        assert!(bytes_per_packet != 0);
        bytes_per_packet
    }

    pub fn packet_size_upper_bound(&self) -> u32 {
        self.packet_size_fixed() // variable size not implemented
    }

    /// Read `buffer.len()` bytes of audio data from byte offset `offset`.
    /// Returns the number of bytes read.
    pub fn read_bytes(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, ()> {
        match self.0 {
            AudioFileInner::Wave(_) => {
                let bytes_per_sample = self.bytes_per_sample();
                assert!(offset % bytes_per_sample == 0);
                assert!(u64::try_from(buffer.len()).unwrap() % bytes_per_sample == 0);

                let sample_count = u64::try_from(buffer.len()).unwrap() / bytes_per_sample;
                let sample_count: usize = sample_count.try_into().unwrap();

                let AudioFileInner::Wave(ref mut wave_reader) = self.0 else {
                    unreachable!()
                };

                wave_reader
                    .seek((offset / bytes_per_sample).try_into().unwrap())
                    .map_err(|_| ())?;

                let mut byte_offset = 0;
                for sample in wave_reader.samples().take(sample_count) {
                    let sample: i16 = sample.map_err(|_| ())?;
                    match bytes_per_sample {
                        // From the OpenAL docs: 8-bit PCM data is expressed as
                        // an unsigned value over the range 0 to 255, 128 being
                        // an audio output level of zero. Loaded wav samples
                        // must be converted to that from signed with 0 as
                        // output level 0.
                        1 => buffer[byte_offset] = (sample + 128) as u8,
                        2 => buffer[byte_offset..][..2].copy_from_slice(&sample.to_le_bytes()),
                        _ => todo!(),
                    }
                    byte_offset += bytes_per_sample as usize;
                }
                Ok(byte_offset)
            }
            AudioFileInner::Caf(_) => {
                // variable size not implemented
                let packet_size = self.packet_size_fixed();
                assert!(offset % u64::from(packet_size) == 0);
                assert!(u64::try_from(buffer.len()).unwrap() % u64::from(packet_size) == 0);

                let packet_count = u64::try_from(buffer.len()).unwrap() / u64::from(packet_size);

                let AudioFileInner::Caf(ref mut caf_reader) = self.0 else {
                    unreachable!()
                };

                caf_reader
                    .seek_to_packet(usize::try_from(offset / u64::from(packet_size)).unwrap())
                    .map_err(|_| ())?;

                let packet_size = usize::try_from(packet_size).unwrap();

                let mut i = 0;
                let mut byte_offset = 0;
                while i < packet_count && caf_reader.next_packet_size().is_some() {
                    caf_reader
                        .read_packet_into(&mut buffer[byte_offset..][..packet_size])
                        .map_err(|_| ())?;
                    byte_offset += packet_size;
                    i += 1;
                }
                Ok(byte_offset)
            }
            AudioFileInner::Mp3(dr_mp3::Mp3DecodedToPcm { ref bytes, .. }) => {
                let bytes = bytes.get(offset as usize..).ok_or(())?;
                let bytes_to_read = buffer.len().min(bytes.len());
                let bytes = &bytes[..bytes_to_read];
                buffer[..bytes_to_read].copy_from_slice(bytes);
                Ok(bytes_to_read)
            }
        }
    }
}

#[must_use]
pub struct ContextManager(*mut ALCcontext);
impl ContextManager {
    pub fn make_active(new_context: *mut ALCcontext) -> ContextManager {
        let old_context = unsafe { openal::alcGetCurrentContext() };
        assert!(unsafe { openal::alcMakeContextCurrent(new_context) } == openal::ALC_TRUE);
        ContextManager(old_context)
    }
}
impl Drop for ContextManager {
    fn drop(&mut self) {
        assert!(unsafe { openal::alcMakeContextCurrent(self.0) } == openal::ALC_TRUE)
    }
}
