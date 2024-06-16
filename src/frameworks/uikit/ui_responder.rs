/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIResponder`.

use crate::objc::{id, nil, objc_classes, ClassExports};

#[derive(Default)]
pub struct State {
    pub(crate) first_responder: id,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIResponder: NSObject

// TODO: real responder implementation etc

// These methods print debug logs because they are only likely to get called if
// a subclass didn't override them, which might mean we delivered the event to
// the wrong object or it is unhandled.

- (())touchesBegan:(id)touches // NSSet* of UITouch*
         withEvent:(id)event { // UIEvent*
    log_dbg!(
        "[{:?} touchesBegan:{:?} withEvent:{:?}] (probably unhandled)",
        this,
        touches,
        event,
    );
}

- (())touchesMoved:(id)touches // NSSet* of UITouch*
         withEvent:(id)event { // UIEvent*
    log_dbg!(
        "[{:?} touchesMoved:{:?} withEvent:{:?}] (probably unhandled)",
        this,
        touches,
        event,
    );
}

- (())touchesEnded:(id)touches // NSSet* of UITouch*
         withEvent:(id)event { // UIEvent*
    log_dbg!(
        "[{:?} touchesEnded:{:?} withEvent:{:?}] (probably unhandled)",
        this,
        touches,
        event,
    );
}

- (bool)canBecomeFirstResponder {
    false
}
- (id)nextResponder {
    nil
}
- (bool)becomeFirstResponder {
    // TODO
    false
}
- (bool)canResignFirstResponder {
    true
}
- (bool)resignFirstResponder {
    // TODO
    true
}

@end

};
