/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UICustomObject`.
//!
//! `UICustomObject` is a private `NSObject` subclass used by Interface Builder
//! as a placeholder for "Custom Objects" placed in a nib (i.e. non-view objects
//! that are instantiated at nib-load time, such as model controllers).
//!
//! When a nib references `UICustomObject` it is typically wrapped inside a
//! `UIClassSwapper`: the `UIClassName` key holds the app's custom class name,
//! and the `UIOriginalClassName` key is `"UICustomObject"`. If the app's
//! custom class is not registered in touchHLE, the nib loader falls back to
//! creating an instance of `UICustomObject` directly, which is why the class
//! must be a real, instantiable class instead of being treated as a marker
//! string. This file provides that real implementation.
//!
//! The class itself does not have any state of its own — it behaves as a
//! plain `NSObject` instance — but we still define `init` and `initWithCoder:`
//! explicitly because some nibs decode a `UICustomObject` directly (without a
//! class swapper), in which case `initWithCoder:` must succeed and simply
//! return `self` rather than walk through `NSCoder`-keyed properties that the
//! coder does not contain.

use crate::objc::{id, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UICustomObject: NSObject

// `UICustomObject` doesn't decode any keys of its own. Real UIKit simply
// returns `self` here — apps that need extra state subclass `UICustomObject`
// (or use a `UIClassSwapper`) and override `initWithCoder:` themselves.
- (id)initWithCoder:(id)coder {
    log_dbg!(
        "[(UICustomObject*){:?} initWithCoder:{:?}]",
        this, coder
    );
    let _ = coder;
    this
}

- (id)init {
    log_dbg!("[(UICustomObject*){:?} init]", this);
    this
}

// `awakeFromNib` is inherited from `NSObject` and is a no-op by default; we
// only override it to log so misbehaving nibs are easier to debug.
- (())awakeFromNib {
    log_dbg!("[(UICustomObject*){:?} awakeFromNib]", this);
}

// KVC (`setValue:forKey:`) is fully implemented by `NSObject` and works for
// `UICustomObject` instances out of the box — this is how nib-loaded outlet
// connections are wired up via `UIRuntimeOutletConnection`. No override is
// required here.

@end

};
