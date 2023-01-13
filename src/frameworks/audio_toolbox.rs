//! The Audio Toolbox framework.

pub mod audio_file;
pub mod audio_queue;

#[derive(Default)]
pub struct State {
    audio_file: audio_file::State,
    audio_queue: audio_queue::State,
}
