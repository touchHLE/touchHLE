/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#![allow(dead_code)]
//! `GKLeaderboardViewController` and `GKAchievementViewController`.

use crate::frameworks::foundation::NSInteger;
use crate::frameworks::uikit::ui_view_controller::UIViewControllerHostObject;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, nil, objc_classes, release, retain, ClassExports, NSZonePtr,
};

type GKLeaderboardTimeScope = NSInteger;
type GKLeaderboardPlayerScope = NSInteger;

// GKLeaderboardTimeScope values
const GKLeaderboardTimeScopeToday: GKLeaderboardTimeScope = 0;
const GKLeaderboardTimeScopeWeek: GKLeaderboardTimeScope = 1;
const GKLeaderboardTimeScopeAllTime: GKLeaderboardTimeScope = 2;

// GKLeaderboardPlayerScope values
const GKLeaderboardPlayerScopeGlobal: GKLeaderboardPlayerScope = 0;
const GKLeaderboardPlayerScopeFriendsOnly: GKLeaderboardPlayerScope = 1;

// MARK: - GKLeaderboardViewController

#[derive(Default)]
struct GKLeaderboardViewControllerHostObject {
    superclass: UIViewControllerHostObject,
    /// GKLeaderboardViewControllerDelegate — weak reference
    leaderboard_delegate: id,
    /// NSString* — category filter
    category: id,
    time_scope: GKLeaderboardTimeScope,
}
impl_HostObject_with_superclass!(GKLeaderboardViewControllerHostObject);

// MARK: - GKFriendRequestComposeViewController

#[derive(Default)]
struct GKFriendRequestComposeViewControllerHostObject {
    superclass: UIViewControllerHostObject,
    /// GKFriendRequestComposeViewControllerDelegate — weak reference
    compose_view_delegate: id,
    /// NSString*
    message: id,
    /// NSInteger
    max_recipients: NSInteger,
}
impl_HostObject_with_superclass!(GKFriendRequestComposeViewControllerHostObject);

// MARK: - GKAchievementViewController

#[derive(Default)]
struct GKAchievementViewControllerHostObject {
    superclass: UIViewControllerHostObject,
    /// GKAchievementViewControllerDelegate — weak reference
    achievement_delegate: id,
}
impl_HostObject_with_superclass!(GKAchievementViewControllerHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// GKLeaderboardViewController
// =========================================================================

@implementation GKLeaderboardViewController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(GKLeaderboardViewControllerHostObject {
        time_scope: GKLeaderboardTimeScopeAllTime,
        ..Default::default()
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (())dealloc {
    let host = env.objc.borrow::<GKLeaderboardViewControllerHostObject>(this);
    let (delegate, category) = (host.leaderboard_delegate, host.category);
    release(env, category);
    // delegate is weak — no release
    let _ = delegate;
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: Delegate

- (id)leaderboardDelegate {
    env.objc.borrow::<GKLeaderboardViewControllerHostObject>(this).leaderboard_delegate
}

- (())setLeaderboardDelegate:(id)delegate {
    // Weak reference — no retain.
    env.objc.borrow_mut::<GKLeaderboardViewControllerHostObject>(this)
        .leaderboard_delegate = delegate;
}

// MARK: Category

- (id)category {
    env.objc.borrow::<GKLeaderboardViewControllerHostObject>(this).category
}

- (())setCategory:(id)category {
    let old = env.objc.borrow::<GKLeaderboardViewControllerHostObject>(this).category;
    release(env, old);
    retain(env, category);
    env.objc.borrow_mut::<GKLeaderboardViewControllerHostObject>(this).category = category;
}

// MARK: Time scope

- (GKLeaderboardTimeScope)timeScope {
    env.objc.borrow::<GKLeaderboardViewControllerHostObject>(this).time_scope
}

- (())setTimeScope:(GKLeaderboardTimeScope)scope {
    env.objc.borrow_mut::<GKLeaderboardViewControllerHostObject>(this).time_scope = scope;
}

// MARK: Presentation stubs

- (())viewDidLoad {
    log!("GKLeaderboardViewController viewDidLoad: stubbed (UI not shown)");
}

- (())viewWillAppear:(bool)_animated {
    log!("GKLeaderboardViewController viewWillAppear: stubbed");
    // Immediately dismiss so the app doesn't hang waiting for user interaction.
    let delegate = env.objc
        .borrow::<GKLeaderboardViewControllerHostObject>(this)
        .leaderboard_delegate;

    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "leaderboardViewControllerDidFinish:".to_string(),
            &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            () = msg![env; delegate leaderboardViewControllerDidFinish:this];
        }
    }
}

- (())viewDidAppear:(bool)_animated {
    // Already dismissed in viewWillAppear — nothing to do.
}

- (())presentModalViewController:(id)_vc animated:(bool)_animated {
    log!("GKLeaderboardViewController presentModalViewController: stubbed");
}

- (())dismissModalViewControllerAnimated:(bool)_animated {
    log!("GKLeaderboardViewController dismissModalViewControllerAnimated: stubbed");
}

@end

// =========================================================================
// GKFriendRequestComposeViewController
// =========================================================================

@implementation GKFriendRequestComposeViewController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<GKFriendRequestComposeViewControllerHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (())dealloc {
    let host = env.objc.borrow::<GKFriendRequestComposeViewControllerHostObject>(this);
    let message = host.message;
    release(env, message);
    // delegate is weak — no release
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: Delegate

- (id)composeViewDelegate {
    env.objc.borrow::<GKFriendRequestComposeViewControllerHostObject>(this).compose_view_delegate
}

- (())setComposeViewDelegate:(id)delegate {
    // Weak reference — no retain.
    env.objc.borrow_mut::<GKFriendRequestComposeViewControllerHostObject>(this)
        .compose_view_delegate = delegate;
}

// MARK: Properties

- (id)message {
    env.objc.borrow::<GKFriendRequestComposeViewControllerHostObject>(this).message
}

- (())setMessage:(id)message {
    let old = env.objc.borrow::<GKFriendRequestComposeViewControllerHostObject>(this).message;
    release(env, old);
    retain(env, message);
    env.objc.borrow_mut::<GKFriendRequestComposeViewControllerHostObject>(this).message = message;
}

- (NSInteger)maxRecipients {
    env.objc.borrow::<GKFriendRequestComposeViewControllerHostObject>(this).max_recipients
}

- (())setMaxRecipients:(NSInteger)max_recipients {
    env.objc.borrow_mut::<GKFriendRequestComposeViewControllerHostObject>(this).max_recipients = max_recipients;
}

// MARK: Add Recipients

- (())addRecipientsWithPlayerIDs:(id)_playerIDs {
    // В эмуляторе реальная отправка не происходит, поэтому мы просто принимаем
    // данные
}

- (())addRecipientsWithEmailAddresses:(id)_emailAddresses {
}

- (())addRecipientWithPlayerID:(id)_playerID {
}

// MARK: Presentation & Lifecycle

- (())viewDidLoad {
    log!("GKFriendRequestComposeViewController viewDidLoad: stubbed (UI not shown)");
}

- (())viewWillAppear:(bool)_animated {
    log!("GKFriendRequestComposeViewController viewWillAppear: stubbed");

    // Эмуляция закрытия окна сразу после "открытия", чтобы игра не висела
    let delegate = env.objc
        .borrow::<GKFriendRequestComposeViewControllerHostObject>(this)
        .compose_view_delegate;

    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "friendRequestComposeViewControllerDidFinish:".to_string(),
            &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            () = msg![env; delegate friendRequestComposeViewControllerDidFinish:this];
        }
    }
}

- (())viewDidAppear:(bool)_animated {
    // Уже "закрыто" в viewWillAppear — ничего не делаем.
}

- (())presentModalViewController:(id)_vc animated:(bool)_animated {
    log!("GKFriendRequestComposeViewController presentModalViewController: stubbed");
}

- (())dismissModalViewControllerAnimated:(bool)_animated {
    log!("GKFriendRequestComposeViewController dismissModalViewControllerAnimated: stubbed");
}

@end

// =========================================================================
// GKAchievementViewController
// =========================================================================

@implementation GKAchievementViewController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<GKAchievementViewControllerHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)init {
    this
}

- (())dealloc {
    // delegate is weak — no release
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: Delegate

- (id)achievementDelegate {
    env.objc.borrow::<GKAchievementViewControllerHostObject>(this).achievement_delegate
}

- (())setAchievementDelegate:(id)delegate {
    // Weak reference — no retain.
    env.objc.borrow_mut::<GKAchievementViewControllerHostObject>(this)
        .achievement_delegate = delegate;
}

// MARK: Presentation stubs

- (())viewDidLoad {
    log!("GKAchievementViewController viewDidLoad: stubbed (UI not shown)");
}

- (())viewWillAppear:(bool)_animated {
    log!("GKAchievementViewController viewWillAppear: stubbed");
    // Immediately dismiss so the app doesn't hang.
    let delegate = env.objc
        .borrow::<GKAchievementViewControllerHostObject>(this)
        .achievement_delegate;

    if delegate != nil {
        let sel = env.objc.register_host_selector(
            "achievementViewControllerDidFinish:".to_string(),
            &mut env.mem,
        );
        let responds: bool = msg![env; delegate respondsToSelector:sel];
        if responds {
            () = msg![env; delegate achievementViewControllerDidFinish:this];
        }
    }
}

- (())viewDidAppear:(bool)_animated {
    // Already dismissed in viewWillAppear — nothing to do.
}

- (())presentModalViewController:(id)_vc animated:(bool)_animated {
    log!("GKAchievementViewController presentModalViewController: stubbed");
}

- (())dismissModalViewControllerAnimated:(bool)_animated {
    log!("GKAchievementViewController dismissModalViewControllerAnimated: stubbed");
}

@end

};
