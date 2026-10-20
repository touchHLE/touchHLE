/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIPageControl`.

use crate::frameworks::foundation::NSInteger;
use crate::objc::{objc_classes, ClassExports};
use crate::todo_objc_setter;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIPageControl: UIControl

- (())setCurrentPage:(NSInteger)current_page {
    todo_objc_setter!(this, current_page);
}

- (())setNumberOfPages:(NSInteger)number_of_pages {
    todo_objc_setter!(this, number_of_pages);
}

@end

};
