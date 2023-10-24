/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

pub mod av_audio_session;

#[derive(Default)]
pub struct State {
    av_audio_session: av_audio_session::State,
}