/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Functions under `Objective-C Runtime Utilities`, just `NSStringFromClass` right now.

use crate::{
    environment::Environment,
    objc::{id, nil, class_getName}, msg_class,
};

pub(super) fn NSStringFromClass(env: &mut Environment, class: id) -> id {
    if class == nil {
        return nil;
    }
    let class_name = class_getName(env, class);
    msg_class![env; NSString stringWithUTF8String:class_name] //Already autoreleased
}
