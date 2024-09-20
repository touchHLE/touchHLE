/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Our implementations of various frameworks.
//!
//! Each child module should be named after the framework it implements.
//! It can potentially have multiple child modules itself if it's a particularly
//! complex framework.
//!
//! See also `dyld/function_lists.rs` and `objc/classes/class_lists.rs`.
//!
//! Most modules in here are not going to link to documentation that should be
//! trivial to find by searching for the class or function name. For example,
//! the documentation of `NSArray` won't link to the main developer.apple.com
//! page documenting that class, but if there's something interesting in the
//! Documentation Archive relating to arrays, that might be linked.

#![allow(non_upper_case_globals)] // Lots of Apple constants begin with "k"
#![allow(clippy::too_many_arguments)] // It's not our fault!

pub mod audio_toolbox;
pub mod av_audio;
pub mod carbon_core;
pub mod core_animation;
pub mod core_audio_types;
pub mod core_foundation;
pub mod core_graphics;
pub mod dnssd;
pub mod foundation;
pub mod game_kit;
pub mod media_player;
pub mod openal;
pub mod opengles;
pub mod store_kit;
pub mod system_configuration;
pub mod uikit;

/// Container for state of various child modules
#[derive(Default)]
pub struct State {
    audio_toolbox: audio_toolbox::State,
    core_animation: core_animation::State,
    foundation: foundation::State,
    media_player: media_player::State,
    openal: openal::State,
    opengles: opengles::State,
    uikit: uikit::State,
}
