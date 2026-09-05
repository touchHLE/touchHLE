/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Media Player framework.

mod media_entity;
mod media_item_collection;
mod media_library;
mod media_picker_controller;
mod media_playlist;
mod media_query;
mod movie_player;
mod music_player;

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/System/Library/Frameworks/MediaPlayer.framework/MediaPlayer",
    aliases: &[],
    class_exports: &[
        movie_player::CLASSES,
        music_player::CLASSES,
        media_entity::CLASSES,
        media_item_collection::CLASSES,
        media_library::CLASSES,
        media_picker_controller::CLASSES,
        media_playlist::CLASSES,
        media_query::CLASSES,
    ],
    constant_exports: &[movie_player::CONSTANTS, music_player::CONSTANTS],
    function_exports: &[],
};

#[derive(Default)]
pub struct State {
    movie_player: movie_player::State,
}

/// For use by `NSRunLoop`: check media players' status, send notifications if
/// necessary.
pub fn handle_players(env: &mut crate::Environment) {
    movie_player::handle_players(env);
}
