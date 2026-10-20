/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::objc::{id, nil, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation SKProduct: NSObject
// TODO
@end

@implementation SKProductsRequest: NSObject
- (id)initWithProductIdentifiers:(id)_ids { // NSSet *
    // TODO
    nil
}
@end

};
