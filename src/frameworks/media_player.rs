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

use crate::objc::{id, nil};

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
    pending_media_picker_presenter: Option<id>,
}

pub fn defer_media_picker_completion(env: &mut crate::Environment, presenter: id) {
    env.framework_state
        .media_player
        .pending_media_picker_presenter = Some(presenter);
}

fn complete_deferred_media_picker(env: &mut crate::Environment) {
    let presenter = env
        .framework_state
        .media_player
        .pending_media_picker_presenter
        .take();

    let Some(presenter) = presenter else {
        return;
    };

    if env
        .objc
        .object_has_method_named(&env.mem, presenter, "mediaPicker:didPickMediaItems:")
    {
        let _: () = crate::msg![env; presenter mediaPicker: nil didPickMediaItems:nil];
    } else if env
        .objc
        .object_has_method_named(&env.mem, presenter, "mediaPickerDidCancel:")
    {
        let _: () = crate::msg![env; presenter mediaPickerDidCancel:nil];
    }
}

/// For use by `NSRunLoop`: check media players' status, send notifications if
/// necessary.
pub fn handle_players(env: &mut crate::Environment) {
    complete_deferred_media_picker(env);
    movie_player::handle_players(env);
}
