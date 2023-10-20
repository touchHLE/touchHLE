/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::frameworks::core_foundation::CFRange;
use crate::frameworks::foundation::NSUInteger;

// Close enough
pub type NSRange = CFRange;

pub const NSNotFound: NSUInteger = 0x7FFFFFFF;
