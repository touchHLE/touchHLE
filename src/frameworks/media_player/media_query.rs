/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMediaQuery`.
//!
//! Apple docs: MPMediaQuery specifies a set of media items from the iPod
//! library by way of media property predicates. Class methods like
//! `playlistsQuery` and `songsQuery` return pre-configured query objects.
//!
//! Our implementation returns valid MPMediaQuery instances whose `items`
//! and `collections` properties return empty arrays (no media on the
//! emulated device). This allows apps that enumerate the user's music
//! library to proceed without crashing.

use crate::objc::{
    autorelease, id, msg, msg_class, objc_classes, ClassExports, HostObject, NSZonePtr,
};

struct MPMediaQueryHostObject;
impl HostObject for MPMediaQueryHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MPMediaQuery: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::new(MPMediaQueryHostObject);
    env.objc.alloc_object(this, host, &mut env.mem)
}

+ (id)playlistsQuery {
    let obj: id = msg_class![env; MPMediaQuery alloc];
    let obj: id = msg![env; obj init];
    autorelease(env, obj)
}

+ (id)songsQuery {
    let obj: id = msg_class![env; MPMediaQuery alloc];
    let obj: id = msg![env; obj init];
    autorelease(env, obj)
}

+ (id)albumsQuery {
    let obj: id = msg_class![env; MPMediaQuery alloc];
    let obj: id = msg![env; obj init];
    autorelease(env, obj)
}

+ (id)artistsQuery {
    let obj: id = msg_class![env; MPMediaQuery alloc];
    let obj: id = msg![env; obj init];
    autorelease(env, obj)
}

// Instance methods that apps call on the query result:

- (id)items {
    // Return empty NSArray — no media items available
    msg_class![env; NSArray array]
}

- (id)collections {
    // Return empty NSArray — no collections (playlists/albums) available
    msg_class![env; NSArray array]
}

- (())addFilterPredicate:(id)_predicate {
    // No-op: we have no items to filter anyway
}

- (())setGroupingType:(i32)_grouping_type {
    // No-op
}

@end

};
