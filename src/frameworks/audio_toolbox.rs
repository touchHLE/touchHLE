//! The Audio Toolbox framework.

pub mod audio_file;

#[derive(Default)]
pub struct State {
    audio_file: audio_file::State,
}
