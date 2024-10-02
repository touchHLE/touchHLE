/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Media Player framework.

pub mod media_library;
pub mod media_query;
pub mod movie_player;
pub mod music_player;

#[derive(Default)]
pub struct State {
    movie_player: movie_player::State,
}

/// For use by `NSRunLoop`: check media players' status, send notifications if
/// necessary.
pub fn handle_players(env: &mut crate::Environment) {
    movie_player::handle_players(env);
}
