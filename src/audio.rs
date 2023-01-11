//! Audio file decoding and OpenAL bindings.
//!
//! The audio file decoding support is an abstraction over various libraries,
//! direct usage of which should be confined to this module:
//! - Wave files use the `hound` crate.

pub mod openal;

use crate::fs::{Fs, GuestPath};
use std::io::Cursor;

pub enum SampleFormat {
    LinearPcmS16,
}
pub struct AudioFormat {
    pub channels: u16,
    /// Hz
    pub sample_rate: u32,
    pub sample_format: SampleFormat,
}

pub struct AudioFile(AudioFileInner);
enum AudioFileInner {
    Wave(hound::WavReader<Cursor<Vec<u8>>>),
}

impl AudioFile {
    pub fn open_for_reading<P: AsRef<GuestPath>>(path: P, fs: &Fs) -> Result<Self, ()> {
        // TODO: it would be better not to load the whole file at once
        let bytes = fs.read(path.as_ref())?;

        if let Ok(wave_reader) = hound::WavReader::new(Cursor::new(bytes)) {
            Ok(AudioFile(AudioFileInner::Wave(wave_reader)))
        } else {
            // We may eventually want to return an error here, this is just more
            // useful currently.
            panic!(
                "Could not decode audio file at path {:?}, likely an unimplemented file format.",
                path.as_ref()
            );
        }
    }

    pub fn audio_format(&self) -> AudioFormat {
        let AudioFileInner::Wave(ref wave_reader) = self.0;

        let hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample,
            sample_format,
        } = wave_reader.spec();
        // Hound supports unsigned 8-bit, signed 16-bit, signed 24-bit and
        // floating-point 32-bit linear PCM. We should expose all of these
        // eventually, but we should only expose formats we've tested.
        assert!(bits_per_sample == 16);
        assert!(sample_format == hound::SampleFormat::Int);

        AudioFormat {
            channels,
            sample_rate,
            sample_format: SampleFormat::LinearPcmS16,
        }
    }

    fn bytes_per_sample(&self) -> u64 {
        let AudioFormat {
            sample_format: SampleFormat::LinearPcmS16,
            ..
        } = self.audio_format();
        2
    }

    pub fn byte_count(&self) -> u64 {
        let AudioFileInner::Wave(ref wave_reader) = self.0;
        let sample_count = wave_reader.len(); // position-independent
        u64::from(sample_count) * self.bytes_per_sample()
    }

    /// Read `buffer.len()` bytes of audio data from byte offset `offset`.
    /// Returns the number of bytes read.
    pub fn read_bytes(&mut self, offset: u64, buffer: &mut [u8]) -> Result<usize, ()> {
        let bytes_per_sample = self.bytes_per_sample();
        assert!(offset % bytes_per_sample == 0);
        assert!(buffer.len() as u64 % bytes_per_sample == 0);

        let sample_count = buffer.len() as u64 / bytes_per_sample;
        let sample_count: usize = sample_count.try_into().unwrap();

        let AudioFileInner::Wave(ref mut wave_reader) = self.0;

        wave_reader
            .seek((offset / bytes_per_sample).try_into().unwrap())
            .map_err(|_| ())?;

        assert!(bytes_per_sample == 2);
        let mut byte_offset = 0;
        for sample in wave_reader.samples().take(sample_count) {
            let sample: i16 = sample.map_err(|_| ())?;
            buffer[byte_offset..][..2].copy_from_slice(&sample.to_le_bytes());
            byte_offset += 2;
        }
        Ok(byte_offset)
    }
}
