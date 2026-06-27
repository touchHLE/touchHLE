/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `GKChallengeEventHandler`.
//!
//! Apple introduced `GKChallengeEventHandler` in iOS 6.0 as the entry
//! point for receiving Game Center challenge events (the sender's
//! delegate is notified whenever a challenge is received, viewed, or
//! completed by the local player). It was deprecated in iOS 7.0 in
//! favour of `GKChallenge`'s class-level convenience methods, but
//! Unity-based games such as *Can Knockdown 3* still instantiate the
//! singleton at startup to register a delegate.
//!
//! Apple reference:
//! <https://developer.apple.com/documentation/gamekit/gkchallengeeventhandler>
//!
//! Since touchHLE has no network connectivity and therefore can never
//! receive real challenges, the singleton stores the caller's delegate
//! but never delivers any events — this matches what a real device
//! does when no challenges have ever been issued to the local player.

use crate::objc::{id, msg, nil, objc_classes, release, ClassExports, HostObject, NSZonePtr};
use crate::Environment;

#[derive(Default)]
pub struct State {
    /// Cached `[GKChallengeEventHandler challengeEventHandler]` singleton.
    singleton: Option<id>,
}

impl State {
    fn get(env: &mut Environment) -> &mut State {
        &mut env.framework_state.game_kit.challenge_event_handler
    }
}

#[derive(Default)]
struct GKChallengeEventHandlerHostObject {
    /// `id<GKChallengeEventHandlerDelegate>` — assigned (NOT retained)
    /// per Apple's docs:
    /// "The receiver does not retain the delegate."
    delegate: id,
}
impl HostObject for GKChallengeEventHandlerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation GKChallengeEventHandler: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(GKChallengeEventHandlerHostObject { delegate: nil });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: - Singleton accessor
//
// Apple reference:
// <https://developer.apple.com/documentation/gamekit/gkchallengeeventhandler/1521033-challengeeventhandler>
// "Returns the singleton challenge event handler."
+ (id)challengeEventHandler {
    if let Some(handler) = State::get(env).singleton {
        return handler;
    }
    // `alloc` gives refcount = 1 which acts as the singleton retain; do
    // NOT autorelease, otherwise the next pool drain would free the
    // singleton.
    let handler: id = msg![env; this alloc];
    let handler: id = msg![env; handler init];
    State::get(env).singleton = Some(handler);
    log!("GKChallengeEventHandler challengeEventHandler: singleton created");
    handler
}

// MARK: - Init / dealloc

- (id)init {
    this
}

- (())dealloc {
    // `delegate` is `assign`, so no release here.
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Delegate
//
// Apple's docs (now archived) declare this as `assign`, not `retain`:
// the delegate is responsible for clearing the property in its own
// `-dealloc` to avoid dangling pointers. We mirror that contract.

- (id)delegate {
    env.objc.borrow::<GKChallengeEventHandlerHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<GKChallengeEventHandlerHostObject>(this).delegate = delegate;
}

// MARK: - Send-to-delegate helpers used by GameKit on a real device
//
// These selectors are documented as the four events GameKit delivers
// to the registered delegate. We expose them as no-ops on the
// singleton so that apps which call them directly (rare, but happens
// in test harnesses) get the same "no challenge to act upon" result a
// real device produces when there is no pending challenge.

- (())localPlayerDidSelectChallenge:(id)_challenge {
    // Nothing to act upon — no challenges ever delivered.
}

- (())localPlayerDidReceiveChallenge:(id)_challenge {
    // Nothing to act upon — no challenges ever delivered.
}

- (())localPlayerDidCompleteChallenge:(id)_challenge {
    // Nothing to act upon — no challenges ever delivered.
}

// Apple's docs:
// "Return YES to allow the default banner to be displayed.
//  Return NO to suppress it."
// With no real challenge ever delivered we still return YES to match
// the documented default.
- (bool)shouldShowBannerForLocallyReceivedChallenge:(id)_challenge { true }
- (bool)shouldShowBannerForLocallyCompletedChallenge:(id)_challenge { true }
- (bool)shouldShowBannerForRemotelyCompletedChallenge:(id)_challenge { true }

@end

};

/// Tears down the cached singleton (kept for completeness; touchHLE has
/// no per-app teardown path today but mirrors `gk_local_player`).
#[allow(dead_code)]
pub fn reset(env: &mut Environment) {
    let state = State::get(env);
    if let Some(handler) = state.singleton.take() {
        release(env, handler);
    }
}
