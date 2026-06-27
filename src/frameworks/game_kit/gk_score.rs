/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
#![allow(dead_code)]
//! `GKScore`.

use crate::frameworks::foundation::ns_string;
use crate::objc::{
    autorelease, id, msg, nil, objc_classes, release, retain, ClassExports, HostObject, NSZonePtr,
};
use std::borrow::Cow;

type GKLeaderboardTimeScope = i32;
type GKLeaderboardPlayerScope = i32;

#[derive(Default)]
struct GKScoreHostObject {
    category: id,
    leaderboard_identifier: id,
    player_id: id,
    formatted_value: id,
    value: i64,
    rank: i64,
    context: u64,
    date: id,
    should_set_default_leaderboard: bool,
}
impl HostObject for GKScoreHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation GKScore: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(GKScoreHostObject {
        category: nil,
        leaderboard_identifier: nil,
        player_id: nil,
        formatted_value: nil,
        value: 0,
        rank: 0,
        context: 0,
        date: nil,
        should_set_default_leaderboard: false,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCategory:(id)category {
    retain(env, category);
    env.objc.borrow_mut::<GKScoreHostObject>(this).category = category;
    retain(env, category);
    env.objc.borrow_mut::<GKScoreHostObject>(this).leaderboard_identifier = category;

    let fmt = ns_string::from_rust_string(env, "0".to_string());
    env.objc.borrow_mut::<GKScoreHostObject>(this).formatted_value = fmt;
    this
}

- (id)initWithLeaderboardIdentifier:(id)identifier {
    msg![env; this initWithCategory:identifier]
}

- (id)initWithLeaderboardIdentifier:(id)identifier player:(id)player {
    let this: id = msg![env; this initWithLeaderboardIdentifier:identifier];
    retain(env, player);
    env.objc.borrow_mut::<GKScoreHostObject>(this).player_id = player;
    this
}

- (())dealloc {
    let host = env.objc.borrow::<GKScoreHostObject>(this);

    let (category, leaderboard_identifier, player_id, formatted_value, date) = (
        host.category,
        host.leaderboard_identifier,
        host.player_id,
        host.formatted_value,
        host.date,
    );

    release(env, category);
    if leaderboard_identifier != category {
        release(env, leaderboard_identifier);
    }
    release(env, player_id);
    release(env, formatted_value);
    release(env, date);

    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)category {
    env.objc.borrow::<GKScoreHostObject>(this).category
}

- (())setCategory:(id)category {
    let old = env.objc.borrow::<GKScoreHostObject>(this).category;
    release(env, old);

    retain(env, category);
    env.objc.borrow_mut::<GKScoreHostObject>(this).category = category;
}

- (id)leaderboardIdentifier {
    env.objc.borrow::<GKScoreHostObject>(this).leaderboard_identifier
}

- (())setLeaderboardIdentifier:(id)identifier {
    let old = env.objc.borrow::<GKScoreHostObject>(this).leaderboard_identifier;

    release(env, old);
    retain(env, identifier);
    env.objc.borrow_mut::<GKScoreHostObject>(this).leaderboard_identifier = identifier;
}

- (i64)value {
    env.objc.borrow::<GKScoreHostObject>(this).value
}

- (())setValue:(i64)value {
    env.objc.borrow_mut::<GKScoreHostObject>(this).value = value;

    let old_fmt = env.objc.borrow::<GKScoreHostObject>(this).formatted_value;
    release(env, old_fmt);
    let fmt = ns_string::from_rust_string(env, value.to_string());
    env.objc.borrow_mut::<GKScoreHostObject>(this).formatted_value = fmt;
}

- (i64)rank {
    env.objc.borrow::<GKScoreHostObject>(this).rank
}

- (u64)context {
    env.objc.borrow::<GKScoreHostObject>(this).context
}

- (())setContext:(u64)context {
    env.objc.borrow_mut::<GKScoreHostObject>(this).context = context;
}

- (id)formattedValue {
    env.objc.borrow::<GKScoreHostObject>(this).formatted_value
}

- (id)playerID {
    env.objc.borrow::<GKScoreHostObject>(this).player_id
}

- (id)player {
    env.objc.borrow::<GKScoreHostObject>(this).player_id
}

- (id)date {
    env.objc.borrow::<GKScoreHostObject>(this).date
}

- (bool)shouldSetDefaultLeaderboard {
    env.objc.borrow::<GKScoreHostObject>(this).should_set_default_leaderboard
}

- (())setShouldSetDefaultLeaderboard:(bool)value {
    env.objc.borrow_mut::<GKScoreHostObject>(this).should_set_default_leaderboard = value;
}

- (())reportScoreWithCompletionHandler:(id)_completion_handler {
    let value = env.objc.borrow::<GKScoreHostObject>(this).value;
    let cat   = env.objc.borrow::<GKScoreHostObject>(this).category;

    let cat_str: Cow<str> = if cat != nil {
        ns_string::to_rust_string(env, cat)
    } else {
        Cow::Borrowed("<nil>")
    };

    log!(
        "GKScore reportScoreWithCompletionHandler: stubbed (category={}, value={})",
        cat_str, value
    );
}

- (id)description {
    let value = env.objc.borrow::<GKScoreHostObject>(this).value;
    let cat   = env.objc.borrow::<GKScoreHostObject>(this).category;

    let cat_str: Cow<str> = if cat != nil {
        ns_string::to_rust_string(env, cat)
    } else {
        Cow::Borrowed("<nil>")
    };

    let desc = format!("<GKScore: category={} value={}>", cat_str, value);
    let ns = ns_string::from_rust_string(env, desc);
    autorelease(env, ns)
}

@end

};
