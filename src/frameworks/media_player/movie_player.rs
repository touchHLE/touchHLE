/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMoviePlayerController` and `MPMoviePlayerViewController`.
//!
//! touchHLE does not yet ship an H.264 decoder, so we cannot actually render
//! the movie's video frames. We do however reproduce the asynchronous lifecycle
//! that Apple's MediaPlayer framework guarantees, so that guest code which
//! waits for the documented notifications proceeds correctly. The lifecycle is
//! modelled after Apple's `MPMoviePlayerController` documentation:
//!
//! 1. After `initWithContentURL:` or `setContentURL:` the player asynchronously
//!    posts `MPMoviePlayerLoadStateDidChangeNotification` once the content is
//!    determined to be playable (transitioning `loadState` from `Unknown` to
//!    `Playable | PlaythroughOK`).
//! 2. The player then posts `MPMovieNaturalSizeAvailableNotification`,
//!    `MPMovieDurationAvailableNotification` and
//!    `MPMoviePlayerReadyForDisplayDidChangeNotification` so callers that
//!    listen for them know the metadata is now valid.
//! 3. For backwards compatibility with apps written against the
//!    `MPMoviePlayerController` introduced in iPhone OS 2 the player also
//!    posts the (now deprecated) `MPMoviePlayerContentPreloadDidFinishNotification`.
//! 4. If `shouldAutoplay` is `YES`, playback transitions to
//!    `MPMoviePlaybackStatePlaying`, posting
//!    `MPMoviePlayerNowPlayingMovieDidChangeNotification` and
//!    `MPMoviePlayerPlaybackStateDidChangeNotification`.
//! 5. Finally `MPMoviePlayerPlaybackDidFinishNotification` is posted with a
//!    `userInfo` dictionary containing
//!    `MPMoviePlayerPlaybackDidFinishReasonUserInfoKey` set to either
//!    `MPMovieFinishReasonPlaybackEnded` (file existed) or
//!    `MPMovieFinishReasonPlaybackError` (file was missing on disk).
//!
//! See:
//! * <https://developer.apple.com/documentation/mediaplayer/mpmoviefinishreason>
//! * <https://developer.apple.com/documentation/mediaplayer/mpmovieloadstate>
//! * <https://developer.apple.com/documentation/mediaplayer/mpmovieplayercontroller>

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_graphics::CGSize;
use crate::frameworks::foundation::{ns_string, ns_url, NSInteger, NSUInteger};
use crate::frameworks::uikit::ui_device::UIDeviceOrientation;
use crate::objc::{
    id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};
use crate::Environment;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Variants of `MPMoviePlayerController` notifications that we schedule on the
/// run loop. The `id` is the player instance; we keep a +1 retain for each
/// queued entry and release it after posting.
#[derive(Copy, Clone)]
enum PendingNotification {
    LoadStateChange(id),
    NaturalSizeAvailable(id),
    DurationAvailable(id),
    ReadyForDisplayChange(id),
    /// Posted for backwards-compatibility with the original
    /// `MPMoviePlayerController` API.
    ContentPreloadDidFinish(id),
    NowPlayingMovieChange(id),
    PlaybackStateChange(id),
    /// `reason` is one of the `MPMovieFinishReason*` values.
    PlaybackDidFinish {
        player: id,
        reason: NSInteger,
    },
}

impl PendingNotification {
    fn player(&self) -> id {
        match *self {
            PendingNotification::LoadStateChange(p)
            | PendingNotification::NaturalSizeAvailable(p)
            | PendingNotification::DurationAvailable(p)
            | PendingNotification::ReadyForDisplayChange(p)
            | PendingNotification::ContentPreloadDidFinish(p)
            | PendingNotification::NowPlayingMovieChange(p)
            | PendingNotification::PlaybackStateChange(p) => p,
            PendingNotification::PlaybackDidFinish { player, .. } => player,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            PendingNotification::LoadStateChange(_) => MPMoviePlayerLoadStateDidChangeNotification,
            PendingNotification::NaturalSizeAvailable(_) => {
                MPMovieNaturalSizeAvailableNotification
            }
            PendingNotification::DurationAvailable(_) => MPMovieDurationAvailableNotification,
            PendingNotification::ReadyForDisplayChange(_) => {
                MPMoviePlayerReadyForDisplayDidChangeNotification
            }
            PendingNotification::ContentPreloadDidFinish(_) => {
                MPMoviePlayerContentPreloadDidFinishNotification
            }
            PendingNotification::NowPlayingMovieChange(_) => {
                MPMoviePlayerNowPlayingMovieDidChangeNotification
            }
            PendingNotification::PlaybackStateChange(_) => {
                MPMoviePlayerPlaybackStateDidChangeNotification
            }
            PendingNotification::PlaybackDidFinish { .. } => {
                MPMoviePlayerPlaybackDidFinishNotification
            }
        }
    }
}

#[derive(Default)]
pub struct State {
    /// The currently-active player. Some apps (e.g. NFSU) rely on us holding a
    /// strong reference to keep the player alive between callbacks.
    active_player: Option<id>,
    /// FIFO of scheduled notifications. The previous implementation used a
    /// `swap_remove_back` model, but that altered the relative ordering of
    /// notifications. Some apps observe the order (e.g. `LoadState` must
    /// arrive before `Duration`), so the queue is now drained strictly in
    /// FIFO order. Each entry retains the player by +1.
    pending_notifications: VecDeque<(PendingNotification, Instant)>,
}
impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.framework_state.media_player.movie_player
    }
}

type MPMovieScalingMode = NSInteger;
type MPMovieControlStyle = NSInteger;
type MPMovieSourceType = NSInteger;
type MPMovieRepeatMode = NSInteger;

type MPMoviePlaybackState = NSInteger;
const MPMoviePlaybackStateStopped: MPMoviePlaybackState = 0;
const MPMoviePlaybackStatePlaying: MPMoviePlaybackState = 1;
const MPMoviePlaybackStatePaused: MPMoviePlaybackState = 2;

/// `MPMovieLoadState` bitmask, matching Apple's `MPMoviePlayerController.h`.
type MPMovieLoadState = NSUInteger;
const MPMovieLoadStateUnknown: MPMovieLoadState = 0;
const MPMovieLoadStatePlayable: MPMovieLoadState = 1 << 0;
const MPMovieLoadStatePlaythroughOK: MPMovieLoadState = 1 << 1;
#[allow(dead_code)]
const MPMovieLoadStateStalled: MPMovieLoadState = 1 << 2;

/// `MPMovieFinishReason` from Apple's MediaPlayer framework.
const MPMovieFinishReasonPlaybackEnded: NSInteger = 0;
const MPMovieFinishReasonPlaybackError: NSInteger = 1;
#[allow(dead_code)]
const MPMovieFinishReasonUserExited: NSInteger = 2;

// Notification names — values copy Apple's symbol names so that
// `[NSNotificationCenter addObserverForName:]` callers match the right key.
pub const MPMoviePlayerPlaybackDidFinishNotification: &str =
    "MPMoviePlayerPlaybackDidFinishNotification";
pub const MPMoviePlayerContentPreloadDidFinishNotification: &str =
    "MPMoviePlayerContentPreloadDidFinishNotification";
pub const MPMoviePlayerScalingModeDidChangeNotification: &str =
    "MPMoviePlayerScalingModeDidChangeNotification";
pub const MPMoviePlayerPlaybackStateDidChangeNotification: &str =
    "MPMoviePlayerPlaybackStateDidChangeNotification";
pub const MPMoviePlayerLoadStateDidChangeNotification: &str =
    "MPMoviePlayerLoadStateDidChangeNotification";
pub const MPMoviePlayerNowPlayingMovieDidChangeNotification: &str =
    "MPMoviePlayerNowPlayingMovieDidChangeNotification";
pub const MPMoviePlayerWillEnterFullscreenNotification: &str =
    "MPMoviePlayerWillEnterFullscreenNotification";
pub const MPMoviePlayerDidEnterFullscreenNotification: &str =
    "MPMoviePlayerDidEnterFullscreenNotification";
pub const MPMoviePlayerWillExitFullscreenNotification: &str =
    "MPMoviePlayerWillExitFullscreenNotification";
pub const MPMoviePlayerDidExitFullscreenNotification: &str =
    "MPMoviePlayerDidExitFullscreenNotification";
const MPMovieDurationAvailableNotification: &str = "MPMovieDurationAvailableNotification";
const MPMoviePlayerReadyForDisplayDidChangeNotification: &str =
    "MPMoviePlayerReadyForDisplayDidChangeNotification";
const MPMovieNaturalSizeAvailableNotification: &str = "MPMovieNaturalSizeAvailableNotification";
const MPMovieMediaTypesAvailableNotification: &str = "MPMovieMediaTypesAvailableNotification";
const MPMovieSourceTypeAvailableNotification: &str = "MPMovieSourceTypeAvailableNotification";
const MPMoviePlayerPlaybackDidFinishReasonUserInfoKey: &str =
    "MPMoviePlayerPlaybackDidFinishReasonUserInfoKey";
// `MPMediaPlayback` (a protocol adopted by `MPMoviePlayerController`)
// posts this notification when `-isPreparedToPlay` flips. Declared in
// `MPMediaPlayback.h`, iOS 3.2+. Canonical NSString value matches the
// symbol name, see
// <https://developer.apple.com/documentation/mediaplayer/mpmediaplaybackispreparedtoplaydidchangenotification>.
const MPMediaPlaybackIsPreparedToPlayDidChangeNotification: &str =
    "MPMediaPlaybackIsPreparedToPlayDidChangeNotification";
// `requestThumbnailImagesAtTimes:timeOption:` result-delivery
// notification + `userInfo` keys. Declared in `MPMoviePlayerController.h`
// (iOS 3.2+, deprecated in 9.0). See
// <https://developer.apple.com/documentation/mediaplayer/mpmovieplayerthumbnailimagerequestdidfinishnotification>.
const MPMoviePlayerThumbnailImageRequestDidFinishNotification: &str =
    "MPMoviePlayerThumbnailImageRequestDidFinishNotification";
const MPMoviePlayerThumbnailImageKey: &str = "MPMoviePlayerThumbnailImageKey";
const MPMoviePlayerThumbnailErrorKey: &str = "MPMoviePlayerThumbnailErrorKey";
const MPMoviePlayerThumbnailTimeKey: &str = "MPMoviePlayerThumbnailTimeKey";

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
        "_MPMoviePlayerPlaybackStateDidChangeNotification",
        HostConstant::NSString(MPMoviePlayerPlaybackStateDidChangeNotification),
    ),
    (
        "_MPMoviePlayerLoadStateDidChangeNotification",
        HostConstant::NSString(MPMoviePlayerLoadStateDidChangeNotification),
    ),
    (
        "_MPMoviePlayerNowPlayingMovieDidChangeNotification",
        HostConstant::NSString(MPMoviePlayerNowPlayingMovieDidChangeNotification),
    ),
    (
        "_MPMoviePlayerWillEnterFullscreenNotification",
        HostConstant::NSString(MPMoviePlayerWillEnterFullscreenNotification),
    ),
    (
        "_MPMoviePlayerDidEnterFullscreenNotification",
        HostConstant::NSString(MPMoviePlayerDidEnterFullscreenNotification),
    ),
    (
        "_MPMoviePlayerWillExitFullscreenNotification",
        HostConstant::NSString(MPMoviePlayerWillExitFullscreenNotification),
    ),
    (
        "_MPMoviePlayerDidExitFullscreenNotification",
        HostConstant::NSString(MPMoviePlayerDidExitFullscreenNotification),
    ),
    (
        "_MPMoviePlayerPlaybackDidFinishReasonUserInfoKey",
        HostConstant::NSString(MPMoviePlayerPlaybackDidFinishReasonUserInfoKey),
    ),
    (
        "_MPMovieDurationAvailableNotification",
        HostConstant::NSString(MPMovieDurationAvailableNotification),
    ),
    (
        "_MPMoviePlayerReadyForDisplayDidChangeNotification",
        HostConstant::NSString(MPMoviePlayerReadyForDisplayDidChangeNotification),
    ),
    (
        "_MPMovieNaturalSizeAvailableNotification",
        HostConstant::NSString(MPMovieNaturalSizeAvailableNotification),
    ),
    (
        "_MPMovieMediaTypesAvailableNotification",
        HostConstant::NSString(MPMovieMediaTypesAvailableNotification),
    ),
    (
        "_MPMovieSourceTypeAvailableNotification",
        HostConstant::NSString(MPMovieSourceTypeAvailableNotification),
    ),
    (
        "_MPMediaPlaybackIsPreparedToPlayDidChangeNotification",
        HostConstant::NSString(MPMediaPlaybackIsPreparedToPlayDidChangeNotification),
    ),
    (
        "_MPMoviePlayerThumbnailImageRequestDidFinishNotification",
        HostConstant::NSString(MPMoviePlayerThumbnailImageRequestDidFinishNotification),
    ),
    (
        "_MPMoviePlayerThumbnailImageKey",
        HostConstant::NSString(MPMoviePlayerThumbnailImageKey),
    ),
    (
        "_MPMoviePlayerThumbnailErrorKey",
        HostConstant::NSString(MPMoviePlayerThumbnailErrorKey),
    ),
    (
        "_MPMoviePlayerThumbnailTimeKey",
        HostConstant::NSString(MPMoviePlayerThumbnailTimeKey),
    ),
];

#[derive(Default)]
struct MPMoviePlayerControllerHostObject {
    // NSURL *
    content_url: id,
    // UIView *
    view: id,
    background_view: id,
    scaling_mode: MPMovieScalingMode,
    control_style: MPMovieControlStyle,
    source_type: MPMovieSourceType,
    repeat_mode: MPMovieRepeatMode,
    should_autoplay: bool,
    initial_playback_time: f64,
    playback_state: MPMoviePlaybackState,
    load_state: MPMovieLoadState,
    /// `naturalSize` reported to the app. We don't have a real decoder so we
    /// fall back to a 4:3 placeholder that comfortably fits within an iPhone
    /// screen; this prevents `division by zero` aspect-ratio code paths.
    natural_size: CGSize,
    duration: f64,
    ready_for_display: bool,
    /// `true` once we have scheduled the post-load notification burst, so
    /// `prepareToPlay` / `play` / `setContentURL:` don't queue it twice.
    preload_scheduled: bool,
    /// `true` once a `PlaybackDidFinish` notification has been queued for
    /// the current content URL, to prevent duplicates when both autoplay
    /// and an explicit `play` happen.
    finish_scheduled: bool,
}
impl HostObject for MPMoviePlayerControllerHostObject {}

/// Default natural size for movies we cannot decode. Matches the 480x320 frame
/// of the iPhone (4:3 letterboxed).
const PLACEHOLDER_NATURAL_SIZE: CGSize = CGSize {
    width: 480.0,
    height: 320.0,
};
/// Fallback duration when we don't know the real one. Real Apple movies report
/// the encoded duration here; we use a small non-zero value to avoid divide-by-
/// zero crashes in app code that builds a progress bar from `currentPlaybackTime
/// / duration`.
const PLACEHOLDER_DURATION: f64 = 1.0;

/// Ensure the player has a valid dummy view, creating one lazily if needed.
/// Returns the view id (always non-nil after this call).
fn ensure_view(env: &mut Environment, this: id) -> id {
    let existing = env
        .objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .view;
    if existing != nil {
        return existing;
    }
    let view_alloc: id = msg_class![env; UIView alloc];
    let view: id = msg![env; view_alloc init];
    retain(env, view);
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .view = view;
    view
}

fn ensure_background_view(env: &mut Environment, this: id) -> id {
    let existing = env
        .objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .background_view;
    if existing != nil {
        return existing;
    }
    let view_alloc: id = msg_class![env; UIView alloc];
    let view: id = msg![env; view_alloc init];
    retain(env, view);
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .background_view = view;
    view
}

/// Enqueue a pending notification, retaining the player so it stays alive
/// until [handle_players] dispatches it.
fn enqueue(env: &mut Environment, notification: PendingNotification, when: Instant) {
    retain(env, notification.player());
    State::get(env)
        .pending_notifications
        .push_back((notification, when));
}

/// Schedule the asynchronous lifecycle of a newly-prepared movie. This is the
/// sequence Apple's `MPMoviePlayerController` posts once it has determined the
/// content is playable. We add small staggered delays so observers that watch
/// for these notifications via `addObserverForName:` (rather than KVO) can
/// distinguish them.
fn schedule_preload_sequence(env: &mut Environment, this: id) {
    {
        let host = env.objc.borrow_mut::<MPMoviePlayerControllerHostObject>(this);
        if host.preload_scheduled {
            return;
        }
        host.preload_scheduled = true;
    }

    // Check whether the file actually exists on disk so we can report a
    // meaningful finish reason later.
    let playback_error_reason = {
        let url = env
            .objc
            .borrow::<MPMoviePlayerControllerHostObject>(this)
            .content_url;
        if url == nil {
            Some(MPMovieFinishReasonPlaybackError)
        } else {
            let path = ns_url::to_rust_path(env, url);
            if env.fs.is_file(&path) {
                None
            } else {
                log!(
                    "MPMoviePlayerController: content URL {:?} is not a readable \
                     file in the guest filesystem; will report MPMovieFinishReasonPlaybackError.",
                    path.as_str()
                );
                Some(MPMovieFinishReasonPlaybackError)
            }
        }
    };

    let now = Instant::now();
    let base = now + Duration::from_millis(20);

    if playback_error_reason.is_some() {
        // For a missing file, real iOS still posts a single finish
        // notification (with reason = PlaybackError); load-state etc. are
        // never posted. Mirror that.
        env.objc
            .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
            .finish_scheduled = true;
        enqueue(
            env,
            PendingNotification::PlaybackDidFinish {
                player: this,
                reason: MPMovieFinishReasonPlaybackError,
            },
            base,
        );
        return;
    }

    // Promote loadState to "Playable | PlaythroughOK".
    {
        let host = env.objc.borrow_mut::<MPMoviePlayerControllerHostObject>(this);
        host.load_state = MPMovieLoadStatePlayable | MPMovieLoadStatePlaythroughOK;
    }
    enqueue(env, PendingNotification::LoadStateChange(this), base);

    // Natural size / duration / ready-for-display all become valid together
    // because we have no decoder pipeline to fill them in over time.
    let metadata_at = base + Duration::from_millis(10);
    enqueue(env, PendingNotification::NaturalSizeAvailable(this), metadata_at);
    enqueue(env, PendingNotification::DurationAvailable(this), metadata_at);
    {
        let host = env.objc.borrow_mut::<MPMoviePlayerControllerHostObject>(this);
        host.ready_for_display = true;
    }
    enqueue(env, PendingNotification::ReadyForDisplayChange(this), metadata_at);

    // Legacy `ContentPreloadDidFinish` (iPhone OS 2 API).
    enqueue(
        env,
        PendingNotification::ContentPreloadDidFinish(this),
        metadata_at,
    );

    // If shouldAutoplay is the default (true) and the app didn't disable it,
    // transition to "playing" automatically.
    let should_autoplay = env
        .objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .should_autoplay;
    if should_autoplay {
        schedule_playback(env, this, metadata_at + Duration::from_millis(10));
    } else {
        // If autoplay is disabled the app is expected to call `play` itself.
        // Apps that never call `play` (e.g. Spore Origins, which only waits
        // for `ContentPreloadDidFinish`) still need a terminal
        // `PlaybackDidFinish` so they can advance past the intro screen.
        // Schedule it as a fallback after the metadata notifications; if the
        // app does call `play`, we'll skip the duplicate.
        env.objc
            .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
            .finish_scheduled = true;
        enqueue(
            env,
            PendingNotification::PlaybackDidFinish {
                player: this,
                reason: MPMovieFinishReasonPlaybackEnded,
            },
            metadata_at + Duration::from_millis(150),
        );
    }
}

fn schedule_playback(env: &mut Environment, this: id, start_at: Instant) {
    let already_finish_scheduled = {
        let host = env.objc.borrow_mut::<MPMoviePlayerControllerHostObject>(this);
        host.playback_state = MPMoviePlaybackStatePlaying;
        let was = host.finish_scheduled;
        host.finish_scheduled = true;
        was
    };

    enqueue(env, PendingNotification::NowPlayingMovieChange(this), start_at);
    enqueue(env, PendingNotification::PlaybackStateChange(this), start_at);

    if !already_finish_scheduled {
        // Treat the (undecoded) movie as having played to its end. Reason 0
        // = MPMovieFinishReasonPlaybackEnded.
        enqueue(
            env,
            PendingNotification::PlaybackDidFinish {
                player: this,
                reason: MPMovieFinishReasonPlaybackEnded,
            },
            start_at + Duration::from_millis(150),
        );
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MPMoviePlayerController: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(MPMoviePlayerControllerHostObject {
        content_url: nil,
        view: nil,
        background_view: nil,
        scaling_mode: 0,
        control_style: 0,
        source_type: 0,
        repeat_mode: 0,
        should_autoplay: true,

        initial_playback_time: -1.0,
        playback_state: MPMoviePlaybackStateStopped,
        load_state: MPMovieLoadStateUnknown,
        natural_size: PLACEHOLDER_NATURAL_SIZE,
        duration: PLACEHOLDER_DURATION,
        ready_for_display: false,
        preload_scheduled: false,
        finish_scheduled: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithContentURL:(id)url { // NSURL*
    log_dbg!(
        "[(MPMoviePlayerController*){:?} initWithContentURL:{:?} ({:?})]",
        this,
        url,
        ns_url::to_rust_path(env, url),
    );

    let this: id = msg![env; this init];
    retain(env, url);

    {
        let host = env.objc.borrow_mut::<MPMoviePlayerControllerHostObject>(this);
        host.content_url = url;
    }

    ensure_view(env, this);
    ensure_background_view(env, this);

    schedule_preload_sequence(env, this);

    this
}

- (())setContentURL:(id)url { // NSURL*
    log_dbg!(
        "[(MPMoviePlayerController*){:?} setContentURL:{:?} ({:?})]",
        this,
        url,
        ns_url::to_rust_path(env, url),
    );

    let (old_url, was_preloaded) = {
        let host = env
            .objc
            .borrow_mut::<MPMoviePlayerControllerHostObject>(this);
        let old = host.content_url;
        host.content_url = url;
        host.preload_scheduled = false;
        host.finish_scheduled = false;
        host.load_state = MPMovieLoadStateUnknown;
        host.ready_for_display = false;
        let was = host.playback_state == MPMoviePlaybackStatePlaying;
        host.playback_state = MPMoviePlaybackStateStopped;
        (old, was)
    };
    retain(env, url);
    if old_url != nil {
        release(env, old_url);
    }

    if was_preloaded {
        // A state change is observable when we tear down the previous
        // playback session.
        enqueue(env, PendingNotification::PlaybackStateChange(this), Instant::now());
    }

    schedule_preload_sequence(env, this);
}

- (())dealloc {
    // No need to drain pending notifications: each pending entry holds a
    // +1 retain on the player, so dealloc can only run once all queued
    // notifications have been delivered and released.
    let url = env
        .objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .content_url;
    release(env, url);

    let view = env
        .objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .view;
    release(env, view);

    let bg_view = env
        .objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .background_view;
    release(env, bg_view);

    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)contentURL {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .content_url
}

- (id)backgroundColor {
    msg_class![env; UIColor blackColor]
}
- (())setBackgroundColor:(id)_color { // UIColor*
    // Background color is a visual property; we have no movie view to tint.
}

// --- Scaling mode ---

- (MPMovieScalingMode)scalingMode {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .scaling_mode
}
- (())setScalingMode:(MPMovieScalingMode)mode {
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .scaling_mode = mode;
}

// --- Control style ---

- (MPMovieControlStyle)controlStyle {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .control_style
}
- (())setControlStyle:(MPMovieControlStyle)style {
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .control_style = style;
}

// --- Source type ---

- (MPMovieSourceType)movieSourceType {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .source_type
}
- (())setMovieSourceType:(MPMovieSourceType)source_type {
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .source_type = source_type;
}

// --- Repeat mode ---

- (MPMovieRepeatMode)repeatMode {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .repeat_mode
}
- (())setRepeatMode:(MPMovieRepeatMode)mode {
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .repeat_mode = mode;
}

// --- Autoplay ---

- (bool)shouldAutoplay {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .should_autoplay
}
- (())setShouldAutoplay:(bool)autoplay {
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .should_autoplay = autoplay;
}

// --- Misc setters ---

- (())setUseApplicationAudioSession:(bool)_use_session {
    // No audio session integration needed in the emulator.
}

- (())setFullscreen:(bool)_fullscreen {
    // Fullscreen is always implied; no UI chrome to hide.
}

- (())setFullscreen:(bool)_fullscreen animated:(bool)_animated {
}

// --- View ---

// Returns the player's backing view. Created lazily if initWithContentURL:
// somehow failed to allocate it, so this always returns a non-nil UIView.
- (id)view {
    ensure_view(env, this)
}

- (id)backgroundView {
    ensure_background_view(env, this)
}
- (())setBackgroundView:(id)view {
    let old = env.objc.borrow::<MPMoviePlayerControllerHostObject>(this).background_view;
    if old != nil {
        release(env, old);
    }
    retain(env, view);
    env.objc.borrow_mut::<MPMoviePlayerControllerHostObject>(this).background_view = view;
}

// --- Playback state / time ---

- (MPMoviePlaybackState)playbackState {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .playback_state
}

- (MPMovieLoadState)loadState {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .load_state
}

- (CGSize)naturalSize {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .natural_size
}

- (f64)currentPlaybackTime {
    // We don't have a real clock; report 0 while stopped, otherwise mid-movie.
    let state = env
        .objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .playback_state;
    if state == MPMoviePlaybackStatePlaying {
        env.objc
            .borrow::<MPMoviePlayerControllerHostObject>(this)
            .duration
            / 2.0
    } else {
        0.0
    }
}
- (())setCurrentPlaybackTime:(f64)_time {
    // No real playback to seek in.
}

- (f64)initialPlaybackTime {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .initial_playback_time
}
- (())setInitialPlaybackTime:(f64)time {
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .initial_playback_time = time;
}

- (f64)duration {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .duration
}
- (f64)playableDuration {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .duration
}
- (bool)isPreparedToPlay {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .load_state
        & (MPMovieLoadStatePlayable | MPMovieLoadStatePlaythroughOK)
        != 0
}
- (bool)readyForDisplay {
    env.objc
        .borrow::<MPMoviePlayerControllerHostObject>(this)
        .ready_for_display
}

- (())prepareToPlay {
    // Per Apple docs `prepareToPlay` is the asynchronous load entry point.
    // Make sure the preload sequence has been scheduled.
    schedule_preload_sequence(env, this);
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
    let already_playing = {
        let host = env
            .objc
            .borrow::<MPMoviePlayerControllerHostObject>(this);
        host.playback_state == MPMoviePlaybackStatePlaying
    };

    // If preload hasn't been scheduled yet (e.g. content was provided via
    // `setContentURL:` without ever calling `prepareToPlay`), run the
    // full sequence now.
    schedule_preload_sequence(env, this);

    if !already_playing {
        let when = Instant::now() + Duration::from_millis(20);
        schedule_playback(env, this, when);
    }
}

- (())pause {
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .playback_state = MPMoviePlaybackStatePaused;
    enqueue(env, PendingNotification::PlaybackStateChange(this), Instant::now());
}

- (())stop {
    env.objc
        .borrow_mut::<MPMoviePlayerControllerHostObject>(this)
        .playback_state = MPMoviePlaybackStateStopped;
    enqueue(env, PendingNotification::PlaybackStateChange(this), Instant::now());
    if env
        .framework_state
        .media_player
        .movie_player
        .active_player == Some(this)
    {
        env.framework_state.media_player.movie_player.active_player = None;
        release(env, this);
    }
}

@end

@implementation MPMoviePlayerViewController: UIViewController

- (id)initWithContentURL:(id)url {
    log_dbg!(
        "[(MPMoviePlayerViewController*){:?} initWithContentURL:{:?} ({:?})]",
        this,
        url,
        ns_url::to_rust_path(env, url),
    );
    // Call designated initializer of UIViewController superclass.
    let this: id = msg![env; this init];

    // Per Apple docs, MPMoviePlayerViewController creates and manages its
    // own MPMoviePlayerController. Create one and store it so the
    // `moviePlayer` property can return it.
    // https://developer.apple.com/documentation/mediaplayer/mpmovieplayerviewcontroller
    let player: id = msg_class![env; MPMoviePlayerController alloc];
    let player: id = msg![env; player initWithContentURL:url];
    // Store as associated value via a dynamic property slot.
    // We use setValue:forKey: with a special key.
    let key = ns_string::get_static_str(env, "_touchHLE_moviePlayer");
    () = msg![env; this setValue:player forKey:key];
    release(env, player); // setValue:forKey: retains

    this
}

// Apple docs: "The movie player controller object used to present the movie."
// @property(nonatomic, readonly) MPMoviePlayerController *moviePlayer
// https://developer.apple.com/documentation/mediaplayer/mpmovieplayerviewcontroller/1619165-movieplayer
- (id)moviePlayer {
    let key = ns_string::get_static_str(env, "_touchHLE_moviePlayer");
    msg![env; this valueForKey:key]
}

@end

};

/// For use by `NSRunLoop` via [super::handle_players]: check movie players'
/// status, send notifications if necessary.
pub(super) fn handle_players(env: &mut Environment) {
    // Pop all notifications whose time has come, preserving FIFO order.
    let mut ready: Vec<PendingNotification> = Vec::new();
    {
        let pending = &mut State::get(env).pending_notifications;
        let now = Instant::now();
        while let Some(&(notif, when)) = pending.front() {
            if when <= now {
                ready.push(notif);
                pending.pop_front();
            } else {
                break;
            }
        }
    }

    for notif in ready {
        let player = notif.player();
        let name_str = notif.name();

        // For PlaybackDidFinish we must update playbackState BEFORE posting,
        // so observers that read [player playbackState] see Stopped.
        if matches!(notif, PendingNotification::PlaybackDidFinish { .. }) {
            env.objc
                .borrow_mut::<MPMoviePlayerControllerHostObject>(player)
                .playback_state = MPMoviePlaybackStateStopped;
        }

        let name = ns_string::get_static_str(env, name_str);
        let center: id = msg_class![env; NSNotificationCenter defaultCenter];

        if let PendingNotification::PlaybackDidFinish { reason, .. } = notif {
            // userInfo[MPMoviePlayerPlaybackDidFinishReasonUserInfoKey] = reason
            let reason_num: id = msg_class![env; NSNumber numberWithInt:(reason)];
            let reason_key = ns_string::get_static_str(
                env,
                MPMoviePlayerPlaybackDidFinishReasonUserInfoKey,
            );
            let user_info: id = msg_class![env; NSDictionary
                dictionaryWithObject:reason_num
                forKey:reason_key];
            let _: () = msg![env; center postNotificationName:name
                                                       object:player
                                                     userInfo:user_info];
        } else {
            let _: () = msg![env; center postNotificationName:name
                                                       object:player];
        }

        // Release the retain we took when queuing this notification.
        release(env, player);
    }
}
