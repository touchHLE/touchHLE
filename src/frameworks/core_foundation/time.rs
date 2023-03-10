/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Time things including `CFAbsoluteTime`.

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::foundation::NSTimeInterval;
use crate::objc::msg_class;
use crate::Environment;

pub type CFTimeInterval = NSTimeInterval;
type CFAbsoluteTime = CFTimeInterval;

fn CFAbsoluteTimeGetCurrent(env: &mut Environment) -> CFAbsoluteTime {
    let time: NSTimeInterval = msg_class![env; NSProcessInfo systemUptime];
    time
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(CFAbsoluteTimeGetCurrent())];
