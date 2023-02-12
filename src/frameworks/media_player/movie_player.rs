/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMoviePlayerController` etc.

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::foundation::{ns_string, ns_url, NSInteger};
use crate::objc::{id, msg, msg_class, objc_classes, release, retain, ClassExports};
use crate::Environment;

#[derive(Default)]
pub struct State {
    active_player: Option<id>,
    finish_notification_pending: bool,
}

type MPMovieScalingMode = NSInteger;

// Value might not be correct, but as this is a linked symbol constant, it
// shouldn't matter.
pub const MPMoviePlayerPlaybackDidFinishNotification: &str =
    "MPMoviePlayerPlaybackDidFinishNotification";

/// `NSNotificationName` values.
pub const CONSTANTS: ConstantExports = &[(
    "_MPMoviePlayerPlaybackDidFinishNotification",
    HostConstant::NSString(MPMoviePlayerPlaybackDidFinishNotification),
)];

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MPMoviePlayerController: NSObject

// TODO: actual playback

- (id)initWithContentURL:(id)url { // NSURL*
    log!(
        "TODO: [(MPMoviePlayerController*){:?} initWithContentURL:{:?} ({:?})]",
        this,
        url,
        ns_url::to_rust_path(env, url),
    );
    this
}

- (())setScalingMode:(MPMovieScalingMode)_mode {
    // TODO
}

// MPMediaPlayback implementation
- (())play {
    log!("TODO: [(MPMoviePlayerController*){:?} play]", this);
    assert!(env.framework_state.media_player.movie_player.active_player.is_none());
    // Movie player is retained by the runtime until it is stopped
    retain(env, this);
    env.framework_state.media_player.movie_player.active_player = Some(this);
    // Act as if playback immediately completed.
    // This is delayed until the next NSRunLoop iteration because it seems that
    // apps (at least Crash Bandicoot Nitro Kart 3D) won't behave correctly if
    // the notification gets posted before this method returns.
    env.framework_state.media_player.movie_player.finish_notification_pending = true;
}

- (())stop {
    log!("TODO: [(MPMoviePlayerController*){:?} stop]", this);
    assert!(this == env.framework_state.media_player.movie_player.active_player.take().unwrap());
    release(env, this);
}

@end

};

/// For use by `NSRunLoop` via [super::handle_players]: check movie players'
/// status, send notifications if necessary.
pub(super) fn handle_players(env: &mut Environment) {
    if !env
        .framework_state
        .media_player
        .movie_player
        .finish_notification_pending
    {
        return;
    }
    env.framework_state
        .media_player
        .movie_player
        .finish_notification_pending = false;

    let player = env
        .framework_state
        .media_player
        .movie_player
        .active_player
        .unwrap();
    let name = ns_string::get_static_str(env, MPMoviePlayerPlaybackDidFinishNotification);
    let center: id = msg_class![env; NSNotificationCenter defaultCenter];
    // TODO: should there be some user info attached?
    let _: () = msg![env; center postNotificationName:name object:player];
    // TODO: do we need to send some other notifications too?
}
