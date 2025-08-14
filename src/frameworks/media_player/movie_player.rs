/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMoviePlayerController` etc.

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_graphics::CGRect;
use crate::frameworks::foundation::{ns_string, ns_url, NSInteger};
use crate::frameworks::uikit::ui_device::UIDeviceOrientation;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, todo_objc_setter, ClassExports,
    HostObject, NSZonePtr,
};
use crate::Environment;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct State {
    active_player: Option<id>,
    /// Various apps (e.g. Crash Bandicoot Nitro Kart 3D and Spore Origins)
    /// create or start a player and await some kind of notification, but can't
    /// handle it if that notification happens immediately. This queue lets us
    /// delay such notifications until the app next returns to the run loop,
    /// which seems to be late enough.
    pending_notifications: VecDeque<(&'static str, id, Instant)>,
}
impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.framework_state.media_player.movie_player
    }
}

type MPMovieScalingMode = NSInteger;
type MPMovieControlStyle = NSInteger;

type MPMoviePlaybackState = NSInteger;
const MPMoviePlaybackStateStopped: MPMoviePlaybackState = 0;

// Values might not be correct, but as these are linked symbol constants, it
// shouldn't matter.
pub const MPMoviePlayerPlaybackDidFinishNotification: &str =
    "MPMoviePlayerPlaybackDidFinishNotification";
/// Apparently an undocumented, private API. Spore Origins uses it.
pub const MPMoviePlayerContentPreloadDidFinishNotification: &str =
    "MPMoviePlayerContentPreloadDidFinishNotification";
pub const MPMoviePlayerScalingModeDidChangeNotification: &str =
    "MPMoviePlayerScalingModeDidChangeNotification";
// TODO: More notifications?
const MPMoviePlayerPlaybackDidFinishReasonUserInfoKey: &str =
    "MPMoviePlayerPlaybackDidFinishReasonUserInfoKey";

/// `NSNotificationName` values and other constants.
pub const CONSTANTS: ConstantExports = &[
    (
        "_MPMoviePlayerPlaybackDidFinishNotification",
        HostConstant::NSString(MPMoviePlayerPlaybackDidFinishNotification),
    ),
    (
        "_MPMoviePlayerContentPreloadDidFinishNotification",
        HostConstant::NSString(MPMoviePlayerContentPreloadDidFinishNotification),
    ),
    (
        "_MPMoviePlayerScalingModeDidChangeNotification",
        HostConstant::NSString(MPMoviePlayerScalingModeDidChangeNotification),
    ),
    (
        "_MPMoviePlayerPlaybackDidFinishReasonUserInfoKey",
        HostConstant::NSString(MPMoviePlayerPlaybackDidFinishReasonUserInfoKey),
    ),
];

struct MPMoviePlayerControllerHostObject {
    // NSURL *
    content_url: id,
    // NSWindow *
    fake_window: id,
    prev_window: id,
    playing: bool,
}
impl HostObject for MPMoviePlayerControllerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MPMoviePlayerController: NSObject

// TODO: actual playback

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(MPMoviePlayerControllerHostObject {
        content_url: nil,
        fake_window: nil,
        prev_window: nil,
        playing: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithContentURL:(id)url { // NSURL*
    log!(
        "TODO: [(MPMoviePlayerController*){:?} initWithContentURL:{:?} ({:?})]",
        this,
        url,
        ns_url::to_rust_path(env, url),
    );

    retain(env, url);
    env.objc.borrow_mut::<MPMoviePlayerControllerHostObject>(this).content_url = url;

    // Act as if loading immediately completed (Spore Origins waits for this).
    State::get(env).pending_notifications.push_back(
        (MPMoviePlayerContentPreloadDidFinishNotification, this, Instant::now())
    );

    this
}

- (())dealloc {
    let url = env.objc.borrow::<MPMoviePlayerControllerHostObject>(this).content_url;
    release(env, url);

    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)contentURL {
    env.objc.borrow::<MPMoviePlayerControllerHostObject>(this).content_url
}

- (id)backgroundColor {
    msg_class![env; UIColor blackColor] // TODO
}
- (())setBackgroundColor:(id)color { // UIColor*
    todo_objc_setter!(this, color);
}

- (())setScalingMode:(MPMovieScalingMode)mode {
    todo_objc_setter!(this, mode);
}
- (())setUseApplicationAudioSession:(bool)use_session {
    todo_objc_setter!(this, use_session);
}
- (())setControlStyle:(MPMovieControlStyle)style {
    todo_objc_setter!(this, style);
}
- (())setFullscreen:(bool)fullsreen {
    todo_objc_setter!(this, fullsreen);
}

- (id)view {
    nil // TODO
}

- (MPMoviePlaybackState)playbackState {
    MPMoviePlaybackStateStopped // TODO
}

// Apparently an undocumented, private API, but Spore Origins uses it.
- (())setMovieControlMode:(NSInteger)_mode {
    // As this is undocumented and we don't have real video playback yet, let's
    // ignore it.
}

// Another undocumented one! But some apps may still use it :/
// https://stackoverflow.com/a/1390079/2241008
- (())setOrientation:(UIDeviceOrientation)_orientation animated:(bool)_animated {
}

// MPMediaPlayback implementation
- (())play {
    log!("TODO: [(MPMoviePlayerController*){:?} play]", this);
    if env.objc.borrow::<MPMoviePlayerControllerHostObject>(this).playing {
        return
    }

    if let Some(old) = env.framework_state.media_player.movie_player.active_player {
        let _: () = msg![env; old stop];
    }
    assert!(env.framework_state.media_player.movie_player.active_player.is_none());
    // Movie player is retained by the runtime until it is stopped
    retain(env, this);
    env.framework_state.media_player.movie_player.active_player = Some(this);
    // Get the old window, make a new window to temporarily take over the old
    // window (POP checks for this)
    let application: id = msg_class![env; UIApplication sharedApplication];
    let curr_window: id = msg![env; application keyWindow];

    let screen: id = msg_class![env; UIScreen mainScreen];
    let bounds: CGRect = msg![env; screen bounds];
    let window: id = msg_class![env; UIWindow alloc];
    let window: id = msg![env; window initWithFrame:bounds];
    let host_obj = env.objc.borrow_mut::<MPMoviePlayerControllerHostObject>(this);
    host_obj.fake_window = window;
    host_obj.prev_window = curr_window;
    host_obj.playing = true;
    let _: () = msg![env; window makeKeyAndVisible];

    // Act as if playback immediately completed after 1 second
    // (various apps wait for this, such as BIA and Hero of Sparta).
    let notif = (MPMoviePlayerPlaybackDidFinishNotification, this, Instant::now().checked_add(Duration::from_millis(1000)).unwrap());
    for (name, obj, _) in &mut State::get(env).pending_notifications {
        // De-duplicate similar notifications. This can happen if app is calling
        // `play` twice on the same player object (case of NOVA2).
        if *name == MPMoviePlayerPlaybackDidFinishNotification && *obj == this {
            return;
        }
    }
    // We also need to retain until the playback notification is complete.
    retain(env, this);
    retain(env, curr_window);

    State::get(env).pending_notifications.push_back(notif);
}

- (())pause {
    log!("TODO: [(MPMoviePlayerController*){:?} pause]", this);
}

- (())stop {
    log!("TODO: [(MPMoviePlayerController*){:?} stop]", this);
    if env.framework_state.media_player.movie_player.active_player.is_some() {
        // Some applications (like NOVA2) may send 2 `stop` messages for each
        // 1 `play` message for the player. In that case, we want to release
        // the active player only once.
        assert!(this == env.framework_state.media_player.movie_player.active_player.take().unwrap());
    }
}

@end

@implementation MPMoviePlayerViewController: UIViewController

- (id)initWithContentURL:(id)url {
    log!(
        "TODO: [(MPMoviePlayerViewController*){:?} initWithContentURL:{:?} ({:?})] -> nil",
        this,
        url,
        ns_url::to_rust_path(env, url),
    );
    release(env, this);
    nil // TODO
}

@end

};

/// For use by `NSRunLoop` via [super::handle_players]: check movie players'
/// status, send notifications if necessary.
pub(super) fn handle_players(env: &mut Environment) {
    let mut notifs_to_run = Vec::new();
    let pending_notifs = &mut State::get(env).pending_notifications;
    let mut i = 0;
    while i < pending_notifs.len() {
        let (name_str, object, time) = pending_notifs[i];
        if Instant::now() >= time {
            notifs_to_run.push((name_str, object));
            pending_notifs.swap_remove_back(i);
        } else {
            i += 1;
        }
    }
    for (name_str, object) in notifs_to_run {
        let name = ns_string::get_static_str(env, name_str);
        let center: id = msg_class![env; NSNotificationCenter defaultCenter];
        // TODO: should there be some user info attached?
        let _: () = msg![env; center postNotificationName:name object:object];
        if name_str == MPMoviePlayerPlaybackDidFinishNotification {
            let host_obj = env
                .objc
                .borrow_mut::<MPMoviePlayerControllerHostObject>(object);
            let window = host_obj.fake_window;
            let prev_window = host_obj.prev_window;
            host_obj.fake_window = nil;
            host_obj.prev_window = nil;
            host_obj.playing = false;
            let _: () = msg![env; prev_window makeKeyAndVisible];
            release(env, object);
            release(env, window);
            release(env, prev_window);
        }
    }
}
