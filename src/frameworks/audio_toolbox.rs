/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Audio Toolbox framework.

pub mod audio_file;
pub mod audio_queue;
pub mod audio_services;
pub mod audio_session;

#[derive(Default)]
pub struct State {
    audio_file: audio_file::State,
    audio_queue: audio_queue::State,
}
