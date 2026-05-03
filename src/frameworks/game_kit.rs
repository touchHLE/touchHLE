/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! GameKit framework.
//!
//! Some features of this framework are only in iOS 4.1+, but some games (like
//! "Cut the Rope") may use it to check for game center availability with
//! a `respondsToSelector:` call to some objects of this framework.
//! Thus, we need to provide some stubs in order to not crash on that call.

mod gk_leaderboard;
mod gk_local_player;
mod gk_score;

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/System/Library/Frameworks/GameKit.framework/GameKit",
    aliases: &[],
    class_exports: &[
        gk_leaderboard::CLASSES,
        gk_local_player::CLASSES,
        gk_score::CLASSES,
    ],
    constant_exports: &[gk_local_player::CONSTANTS],
    function_exports: &[],
};
