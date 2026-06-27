/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! AVFoundation video playback classes: `AVAsset` / `AVURLAsset`,
//! `AVPlayerItem`, `AVPlayer` and `AVPlayerLayer`.
//!
//! touchHLE has no real video pipeline yet. Several games (e.g. Zynga's
//! *Horn*) build an intro / cut-scene player out of these classes during
//! launch and dereference the returned objects unconditionally — if the
//! classes are missing, the very first `+[AVURLAsset URLAssetWithURL:options:]`
//! call returns `nil`, the app sends further messages to `nil` /
//! unimplemented classes and stalls before it ever reaches its main menu.
//!
//! This module provides a high-level Objective-C surface that is just
//! functional enough for that launch path: the objects can be created,
//! retain their inputs, answer the common KVO/property getters, fire the
//! `AVPlayerItemDidPlayToEndTimeNotification` so "play the intro then
//! continue" state machines advance, and (for `AVPlayerLayer`) render as a
//! plain black CALayer so the video frame is simply blank instead of
//! crashing. No actual decoding happens.

use crate::frameworks::core_graphics::CGRect;
use crate::frameworks::foundation::ns_string;
use crate::objc::{
    autorelease, id, msg, msg_class, msg_super, nil, objc_classes, release, retain, ClassExports,
    HostObject, NSZonePtr,
};

#[derive(Default)]
struct AVAssetHostObject {
    /// NSURL* the asset was created from (retained), or nil.
    url: id,
}
impl HostObject for AVAssetHostObject {}

#[derive(Default)]
struct AVPlayerItemHostObject {
    /// AVAsset* (retained), or nil.
    asset: id,
}
impl HostObject for AVPlayerItemHostObject {}

#[derive(Default)]
struct AVPlayerHostObject {
    /// Current AVPlayerItem* (retained), or nil.
    current_item: id,
    rate: f32,
}
impl HostObject for AVPlayerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// ===========================================================================
// AVAsset / AVURLAsset
// ===========================================================================

@implementation AVAsset: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::new(AVAssetHostObject::default());
    env.objc.alloc_object(this, host, &mut env.mem)
}

- (())dealloc {
    let url = env.objc.borrow::<AVAssetHostObject>(this).url;
    release(env, url);
    env.objc.dealloc_object(this, &mut env.mem)
}

// AVAsset properties that callers commonly read. We never load real media,
// so durations/tracks are empty/zero.
- (id)tracks {
    msg_class![env; NSArray array]
}
- (id)commonMetadata {
    msg_class![env; NSArray array]
}
- (bool)isPlayable { true }
- (bool)isReadable { true }
- (bool)isExportable { false }
- (bool)hasProtectedContent { false }

- (id)tracksWithMediaType:(id)_media_type {
    msg_class![env; NSArray array]
}

- (())loadValuesAsynchronouslyForKeys:(id)_keys
                    completionHandler:(id)handler {
    // Report completion immediately so apps that gate startup on the
    // callback don't wait forever.
    if handler != nil {
        () = msg![env; handler invoke];
    }
}

- (i32)statusOfValueForKey:(id)_key error:(crate::mem::MutPtr<id>)error {
    if !error.is_null() {
        env.mem.write(error, nil);
    }
    // AVKeyValueStatusLoaded = 2
    2
}

@end

@implementation AVURLAsset: AVAsset

+ (id)URLAssetWithURL:(id)url options:(id)_options { // NSURL*, NSDictionary*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithURL:url options:_options];
    autorelease(env, new)
}

- (id)initWithURL:(id)url options:(id)_options {
    let this: id = msg_super![env; this init];
    if url != nil {
        retain(env, url);
    }
    env.objc.borrow_mut::<AVAssetHostObject>(this).url = url;
    this
}

- (id)URL {
    env.objc.borrow::<AVAssetHostObject>(this).url
}

@end

// ===========================================================================
// AVPlayerItem
// ===========================================================================

@implementation AVPlayerItem: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::new(AVPlayerItemHostObject::default());
    env.objc.alloc_object(this, host, &mut env.mem)
}

+ (id)playerItemWithAsset:(id)asset {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithAsset:asset];
    autorelease(env, new)
}

+ (id)playerItemWithURL:(id)url {
    let asset: id = msg_class![env; AVURLAsset URLAssetWithURL:url options:nil];
    msg![env; this playerItemWithAsset:asset]
}

- (id)initWithAsset:(id)asset {
    if asset != nil {
        retain(env, asset);
    }
    env.objc.borrow_mut::<AVPlayerItemHostObject>(this).asset = asset;
    this
}

- (id)initWithURL:(id)url {
    let asset: id = msg_class![env; AVURLAsset URLAssetWithURL:url options:nil];
    msg![env; this initWithAsset:asset]
}

- (())dealloc {
    let asset = env.objc.borrow::<AVPlayerItemHostObject>(this).asset;
    release(env, asset);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)asset {
    env.objc.borrow::<AVPlayerItemHostObject>(this).asset
}

// AVPlayerItemStatusReadyToPlay = 1
- (i32)status { 1 }

- (id)tracks {
    msg_class![env; NSArray array]
}

- (())addOutput:(id)_output {}
- (())removeOutput:(id)_output {}

@end

// ===========================================================================
// AVPlayer
// ===========================================================================

@implementation AVPlayer: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::new(AVPlayerHostObject::default());
    env.objc.alloc_object(this, host, &mut env.mem)
}

+ (id)playerWithPlayerItem:(id)item {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithPlayerItem:item];
    autorelease(env, new)
}

+ (id)playerWithURL:(id)url {
    let item: id = msg_class![env; AVPlayerItem playerItemWithURL:url];
    msg![env; this playerWithPlayerItem:item]
}

- (id)initWithPlayerItem:(id)item {
    if item != nil {
        retain(env, item);
    }
    env.objc.borrow_mut::<AVPlayerHostObject>(this).current_item = item;
    this
}

- (id)initWithURL:(id)url {
    let item: id = msg_class![env; AVPlayerItem playerItemWithURL:url];
    msg![env; this initWithPlayerItem:item]
}

- (())dealloc {
    let item = env.objc.borrow::<AVPlayerHostObject>(this).current_item;
    release(env, item);
    env.objc.dealloc_object(this, &mut env.mem)
}

- (id)currentItem {
    env.objc.borrow::<AVPlayerHostObject>(this).current_item
}

- (())replaceCurrentItemWithPlayerItem:(id)item {
    let old = env.objc.borrow::<AVPlayerHostObject>(this).current_item;
    if item != nil {
        retain(env, item);
    }
    release(env, old);
    env.objc.borrow_mut::<AVPlayerHostObject>(this).current_item = item;
}

// AVPlayerStatusReadyToPlay = 1
- (i32)status { 1 }

- (f32)rate {
    env.objc.borrow::<AVPlayerHostObject>(this).rate
}
- (())setRate:(f32)rate {
    env.objc.borrow_mut::<AVPlayerHostObject>(this).rate = rate;
}

- (())setActionAtItemEnd:(i64)_action {}
- (())setMuted:(bool)_muted {}
- (bool)isMuted { false }
- (())setVolume:(f32)_volume {}
- (f32)volume { 1.0 }

- (())play {
    env.objc.borrow_mut::<AVPlayerHostObject>(this).rate = 1.0;
    // We can't decode the video, so immediately signal "finished playing" so
    // apps that wait for AVPlayerItemDidPlayToEndTimeNotification to advance
    // (e.g. an intro movie before the main menu) don't get stuck.
    let item = env.objc.borrow::<AVPlayerHostObject>(this).current_item;
    if item != nil {
        let name = ns_string::get_static_str(
            env,
            "AVPlayerItemDidPlayToEndTimeNotification",
        );
        let nc: id = msg_class![env; NSNotificationCenter defaultCenter];
        () = msg![env; nc postNotificationName:name object:item];
    }
}

- (())pause {
    env.objc.borrow_mut::<AVPlayerHostObject>(this).rate = 0.0;
}

@end

// ===========================================================================
// AVPlayerLayer (CALayer subclass)
// ===========================================================================

@implementation AVPlayerLayer: CALayer

+ (id)playerLayerWithPlayer:(id)player {
    let cls: crate::objc::Class =
        env.objc.get_known_class("AVPlayerLayer", &mut env.mem);
    let layer: id = msg![env; cls alloc];
    let layer: id = msg![env; layer init];
    () = msg![env; layer setPlayer:player];
    // Render as a plain black layer (a blank video frame) instead of leaving
    // the contents undefined.
    let black: id = msg_class![env; UIColor blackColor];
    let cg: id = msg![env; black CGColor];
    () = msg![env; layer setBackgroundColor:cg];
    autorelease(env, layer)
}

- (id)player {
    env.framework_state
        .avfoundation
        .av_player_layer_players
        .get(&this)
        .copied()
        .unwrap_or(nil)
}
- (())setPlayer:(id)player {
    let old = env
        .framework_state
        .avfoundation
        .av_player_layer_players
        .get(&this)
        .copied()
        .unwrap_or(nil);
    if player != nil {
        retain(env, player);
    }
    if old != nil {
        release(env, old);
    }
    env.framework_state
        .avfoundation
        .av_player_layer_players
        .insert(this, player);
}

- (id)videoGravity {
    ns_string::get_static_str(env, "AVLayerVideoGravityResizeAspect")
}
- (())setVideoGravity:(id)_gravity {}

- (bool)isReadyForDisplay { true }

- (CGRect)videoRect {
    msg![env; this bounds]
}

@end

};
