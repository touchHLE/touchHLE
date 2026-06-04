/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIPasteboard`.

use crate::objc::{id, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIPasteboard: NSObject
// TODO

+ (id)pasteboardWithName:(id)pasteboardName
        create:(bool) create {
    log!("TODO: [(UIPasteboard*){:?} pasteboardWithName:{:?} create:{:?} ]", this, pasteboardName, create);
    this
}

+ (id)string {
    log!("TODO: UIPasteboard string parameter");
    this
}

+ (id)length {
    log!("TODO: UIPasteboard length parameter");
    this
}

@end

};
