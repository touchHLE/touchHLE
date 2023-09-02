/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AVAudioPlayer`

use crate::{
    objc::{id, nil, ClassExports},
    objc_classes,
};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation AVAudioPlayer: NSObject

+ (id)alloc {
    log!("TODO: [AVAudioPlayer alloc]");
    nil
}

@end

};
