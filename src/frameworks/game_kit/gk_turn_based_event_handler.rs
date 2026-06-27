/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `GKTurnBasedEventHandler`.
//!
//! Apple introduced `GKTurnBasedEventHandler` in iOS 5.0 as the entry
//! point for receiving Game Center turn-based match events (a new turn,
//! a match ending, or an invite accepted from outside the game). It was
//! deprecated in iOS 7.0 in favour of `GKLocalPlayer`'s
//! `GKTurnBasedEventListener`, but Unity-based games such as
//! *République* still call `[GKTurnBasedEventHandler
//! sharedTurnBasedEventHandler]` at startup to register a delegate.
//!
//! Apple reference:
//! <https://developer.apple.com/documentation/gamekit/gkturnbasedeventhandler>
//! <https://developer.apple.com/documentation/gamekit/gkturnbasedeventhandlerdelegate>
//!
//! Before this class existed in touchHLE, the `sharedTurnBasedEventHandler`
//! call hit the generic "unimplemented class" path, which zero-fills r0/r1
//! and returns. The Unity GameKit binding then treated the resulting nil
//! singleton as a fatal condition: it kept re-dispatching into a stub that
//! trapped on an `UndefinedInstruction` at a bogus low address (0x4000),
//! looping until the emulator's bypass limit (256) was hit and it panicked.
//!
//! Since touchHLE has no network connectivity and can never receive real
//! turn-based events, the singleton stores the caller's delegate but never
//! delivers any events — exactly what a real device does when the local
//! player is signed out of Game Center and no matches exist.

use crate::objc::{id, msg, nil, objc_classes, release, ClassExports, HostObject, NSZonePtr};
use crate::Environment;

#[derive(Default)]
pub struct State {
    /// Cached `[GKTurnBasedEventHandler sharedTurnBasedEventHandler]`
    /// singleton.
    singleton: Option<id>,
}

impl State {
    fn get(env: &mut Environment) -> &mut State {
        &mut env.framework_state.game_kit.turn_based_event_handler
    }
}

#[derive(Default)]
struct GKTurnBasedEventHandlerHostObject {
    /// `id<GKTurnBasedEventHandlerDelegate>` — assigned (NOT retained).
    /// Apple's archived docs declare the `delegate` property as `assign`,
    /// so the delegate is responsible for clearing it in its own
    /// `-dealloc`. We mirror that contract to avoid retaining the delegate.
    delegate: id,
}
impl HostObject for GKTurnBasedEventHandlerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation GKTurnBasedEventHandler: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(GKTurnBasedEventHandlerHostObject { delegate: nil });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: - Singleton accessor
//
// Apple reference:
// <https://developer.apple.com/documentation/gamekit/gkturnbasedeventhandler/shared()>
// "Your game never directly creates a GKTurnBasedEventHandler object.
//  Instead, retrieve the shared instance using this class method."
+ (id)sharedTurnBasedEventHandler {
    if let Some(handler) = State::get(env).singleton {
        return handler;
    }
    // `alloc` gives refcount = 1 which acts as the singleton retain; do
    // NOT autorelease, otherwise the next pool drain would free the
    // singleton.
    let handler: id = msg![env; this alloc];
    let handler: id = msg![env; handler init];
    State::get(env).singleton = Some(handler);
    log!("GKTurnBasedEventHandler sharedTurnBasedEventHandler: singleton created");
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
// Apple's docs declare this as `assign` (weak), not `retain`: the
// delegate clears the property in its own `-dealloc` to avoid dangling
// pointers. We mirror that contract.

- (id)delegate {
    env.objc.borrow::<GKTurnBasedEventHandlerHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<GKTurnBasedEventHandlerHostObject>(this).delegate = delegate;
}

// MARK: - GKTurnBasedEventHandlerDelegate forwarding selectors
//
// Apple documents three events GameKit delivers to the registered
// delegate. We expose them as no-ops on the singleton so apps that call
// them directly (rare, but happens in test harnesses) get the same
// "nothing pending" result a real signed-out device produces. No events
// are ever delivered because touchHLE has no Game Center connectivity.
//
// <https://developer.apple.com/documentation/gamekit/gkturnbasedeventhandlerdelegate/handleinvitefromgamecenter(_:)>
- (())handleInviteFromGameCenter:(id)_player_ids_to_invite {
    // No invites can arrive without Game Center connectivity.
}

// <https://developer.apple.com/documentation/gamekit/gkturnbasedeventhandlerdelegate/handleturnevent(for:didbecomeactive:)>
- (())handleTurnEventForMatch:(id)_match
                didBecomeActive:(bool)_did_become_active {
    // No turn events are ever delivered.
}

// Older single-argument variant shipped in the iOS 5.0 SDK before the
// `didBecomeActive:` parameter was added in iOS 6.0. Some games built
// against the 5.0 headers register this selector instead.
- (())handleTurnEventForMatch:(id)_match {
    // No turn events are ever delivered.
}

// <https://developer.apple.com/documentation/gamekit/gkturnbasedeventhandlerdelegate/handlematchended(_:)>
- (())handleMatchEnded:(id)_match {
    // No matches exist, so none can end.
}

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
