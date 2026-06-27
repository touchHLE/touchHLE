/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMediaLibrary`.
//!
//! Apple docs: MPMediaLibrary provides a reference to the device's media
//! library. `defaultMediaLibrary` returns a singleton.
//!
//! Our implementation returns a valid singleton instance but with an empty
//! library — no songs, playlists, or artists. This prevents apps from
//! crashing when they call methods on the returned object.

use crate::objc::{id, msg, msg_class, objc_classes, ClassExports, HostObject, NSZonePtr};

/// Host object for MPMediaLibrary — just a marker (empty library).
struct MPMediaLibraryHostObject;
impl HostObject for MPMediaLibraryHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MPMediaLibrary: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host = Box::new(MPMediaLibraryHostObject);
    env.objc.alloc_object(this, host, &mut env.mem)
}

+ (id)defaultMediaLibrary {
    // Return a singleton. On real iOS this is a shared instance that
    // represents the iPod/Music library on the device.
    // We create a new instance each time but this is harmless — the app
    // just gets an empty library and its queries return empty results.
    let obj: id = msg_class![env; MPMediaLibrary alloc];
    let obj: id = msg![env; obj init];
    // Autorelease so caller gets +0 (class method convention)
    crate::objc::autorelease(env, obj)
}

// MPMediaLibrary instance methods that apps may call:

- (())beginGeneratingLibraryChangeNotifications {
    // No-op: we never post change notifications (library is static/empty)
}

- (())endGeneratingLibraryChangeNotifications {
    // No-op
}

- (id)lastModifiedDate {
    // Return current date. Apps use this to check if library changed.
    msg_class![env; NSDate date]
}

@end

};
