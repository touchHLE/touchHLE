/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMusicPlayerController` etc.

use crate::objc::{id, nil, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MPMusicPlayerController: NSObject

+ (id)iPodMusicPlayer {
    log!(
        "TODO: [(MPMusicPlayerController*){:?} iPodMusicPlayer]",
        this
    );
    nil
}

@end

};
