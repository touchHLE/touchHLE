/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMediaQuery`.

use crate::objc::{id, nil, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MPMediaQuery: NSObject

+ (id)playlistsQuery {
    log!("TODO: [MPMediaQuery playlistsQuery] (not implemented yet)");
    nil
}

+ (id)songsQuery {
    log!("TODO: [MPMediaQuery songsQuery] (not implemented yet)");
    nil
}

@end

};
