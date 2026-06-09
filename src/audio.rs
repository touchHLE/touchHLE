/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Audio file decoding and OpenAL bindings.
//!
//! The audio file decoding support is an abstraction over various libraries
//! (currently [hound] and [symphonia]), usage of which should be
//! confined to this module.
//!
//! Resources:
//! - [Apple Core Audio Format Specification 1.0](https://developer.apple.com/library/archive/documentation/MusicAudio/Reference/CAFSpec/CAF_intro/CAF_intro.html)

mod ima4;
pub mod openal;
mod symphonia_formats;

pub use ima4::decode_ima4;

use crate::fs::{Fs, GuestPath};
use std::io::Cursor;

#[derive(Debug)]
pub enum AudioFileOpenError {
    FileReadError,
    FileDecodeError,
}

#[derive(Debug)]
pub enum AudioFormat {
    LinearPcm {
        is_float: bool,
        is_little_endian: bool,
    },
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

pub struct AudioFile(AudioFileInner);
enum AudioFileInner {
    Wave(hound::WavReader<Cursor<Vec<u8>>>),
    Symphonia(symphonia_formats::SymphoniaDecodedToPcm),
}

impl AudioFile {
    pub fn open_for_reading<P: AsRef<GuestPath>>(
        path: P,
        fs: &Fs,
    ) -> Result<Self, AudioFileOpenError> {
        // TODO: it would be better not to load the whole file at once
        let Ok(bytes) = fs.read(path.as_ref()) else {
            // TODO: Handle other FS related errors?
            return Err(AudioFileOpenError::FileReadError);
        };

        if let Ok(bytes) = Self::read_from_vec(bytes) {
            Ok(bytes)
        } else {
            log!(
                "Could not decode audio file at path {:?}, likely an unimplemented file format.",
                path.as_ref()
            );
            Err(AudioFileOpenError::FileReadError)
        }
    }

    pub fn read_from_vec(bytes: Vec<u8>) -> Result<Self, AudioFileOpenError> {
        // WavReader::new() consume the reader (in this case, a Cursor)
        // passed to it. This is a bit annoying considering we don't know
        // which is appropriate for the file. This is worked around here by
        // using temporary readers for checking if the file is the supported
        // format, then recreating the reader if that works.
        if hound::WavReader::new(Cursor::new(&bytes)).is_ok() {
            let reader = hound::WavReader::new(Cursor::new(bytes)).unwrap();
            Ok(AudioFile(AudioFileInner::Wave(reader)))
        // TODO: Real MP3/MP4/Non-linear PCM container handling. Currently we
        // are immediately decoding the entire file to PCM and acting as if
        // it's a PCM file, simply because because this is easier. Full MP3
        // support would require a lot of changes in Audio Toolbox.
        } else if let Ok(pcm) = symphonia_formats::decode_symphonia_to_pcm(Cursor::new(bytes)) {
            Ok(AudioFile(AudioFileInner::Symphonia(pcm)))
        } else {
            Err(AudioFileOpenError::FileDecodeError)
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
            AudioFileInner::Symphonia(symphonia_formats::SymphoniaDecodedToPcm {
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
            panic!("{format:?} is a compressed format!");
        }
        ((bytes_per_packet / frames_per_packet) / channels_per_frame).into()
    }

    pub fn byte_count(&self) -> u64 {
        match self.0 {
            AudioFileInner::Wave(ref wave_reader) => {
                let sample_count = wave_reader.len(); // position-independent
                u64::from(sample_count) * self.bytes_per_sample()
            }
            AudioFileInner::Symphonia(symphonia_formats::SymphoniaDecodedToPcm {
                ref bytes,
                ..
            }) => bytes.len() as u64,
        }
    }

    pub fn packet_count(&self) -> u64 {
        match self.0 {
            AudioFileInner::Wave(_)
            | AudioFileInner::Symphonia(symphonia_formats::SymphoniaDecodedToPcm { .. }) => {
                // never variable-size
                self.byte_count() / u64::from(self.packet_size_fixed())
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
                assert!(offset.is_multiple_of(bytes_per_sample));
                assert!(u64::try_from(buffer.len())
                    .unwrap()
                    .is_multiple_of(bytes_per_sample));

                let sample_count = u64::try_from(buffer.len()).unwrap() / bytes_per_sample;
                let sample_count: usize = sample_count.try_into().unwrap();

                let AudioFileInner::Wave(ref mut wave_reader) = self.0 else {
                    unreachable!()
                };

                let channels: u64 = wave_reader.spec().channels.into();
                // WavReader expects number of samples which are
                // independent of the number of channels here
                wave_reader
                    .seek((offset / (bytes_per_sample * channels)).try_into().unwrap())
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
            AudioFileInner::Symphonia(symphonia_formats::SymphoniaDecodedToPcm {
                ref bytes,
                ..
            }) => {
                let bytes = bytes.get(offset as usize..).ok_or(())?;
                let bytes_to_read = buffer.len().min(bytes.len());
                let bytes = &bytes[..bytes_to_read];
                buffer[..bytes_to_read].copy_from_slice(bytes);
                Ok(bytes_to_read)
            }
        }
    }

    pub fn estimated_duration(&self) -> f64 {
        let AudioDescription {
            sample_rate,
            bytes_per_packet,
            frames_per_packet,
            ..
        } = self.audio_description();
        assert!(bytes_per_packet != 0); // TODO
        self.byte_count() as f64 * frames_per_packet as f64
            / (bytes_per_packet as f64 * sample_rate)
    }
}
