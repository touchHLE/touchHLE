/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `MPMediaPickerController`.

use crate::frameworks::foundation::NSUInteger;
use crate::objc::{id, nil, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation MPMediaPickerController: UIViewController

// df.ipa's iPod Music feature is intentionally unsupported. Returning nil
// makes the app's optional picker path a no-op instead of entering a modal
// flow that cannot be completed without a host music library.
- (id)initWithMediaTypes:(NSUInteger)media_types {
    log!(
        "Ignoring MPMediaPickerController initWithMediaTypes:{:?} (unsupported)",
        media_types
    );
    nil
}

@end

};
