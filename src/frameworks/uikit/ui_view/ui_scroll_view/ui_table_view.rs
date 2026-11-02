/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Basic `UITableView` configuration. Cell rendering is not yet implemented.

use crate::frameworks::core_graphics::{CGFloat, CGRect};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_super, nil, objc_classes, ClassExports, NSZonePtr,
};

struct UITableViewHostObject {
    superclass: super::UIScrollViewHostObject,
    // The data source is not retained, matching UIScrollView's delegate.
    data_source: id,
    style: NSInteger,
    row_height: CGFloat,
    separator_style: NSInteger,
}
impl_HostObject_with_superclass!(UITableViewHostObject);

impl Default for UITableViewHostObject {
    fn default() -> Self {
        Self {
            superclass: Default::default(),
            data_source: nil,
            style: 0,
            row_height: 44.0,
            separator_style: 1,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UITableView: UIScrollView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UITableViewHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame style:(NSInteger)style {
    let this: id = msg_super![env; this initWithFrame:frame];
    env.objc.borrow_mut::<UITableViewHostObject>(this).style = style;
    this
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];
    let key = get_static_str(env, "UIStyle");
    let style: NSInteger = msg![env; coder decodeIntForKey:key];
    env.objc.borrow_mut::<UITableViewHostObject>(this).style = style;
    let key = get_static_str(env, "UIRowHeight");
    let has_height: bool = msg![env; coder containsValueForKey:key];
    if has_height {
        let height: CGFloat = msg![env; coder decodeFloatForKey:key];
        () = msg![env; this setRowHeight:height];
    }
    let key = get_static_str(env, "UISeparatorStyle");
    let has_style: bool = msg![env; coder containsValueForKey:key];
    if has_style {
        let style: NSInteger = msg![env; coder decodeIntForKey:key];
        () = msg![env; this setSeparatorStyle:style];
    }
    this
}

- (NSInteger)style {
    env.objc.borrow::<UITableViewHostObject>(this).style
}

- (id)dataSource {
    env.objc.borrow::<UITableViewHostObject>(this).data_source
}
- (())setDataSource:(id)source {
    env.objc.borrow_mut::<UITableViewHostObject>(this).data_source = source;
}

- (CGFloat)rowHeight {
    env.objc.borrow::<UITableViewHostObject>(this).row_height
}
- (())setRowHeight:(CGFloat)height {
    env.objc.borrow_mut::<UITableViewHostObject>(this).row_height = height;
}

- (NSInteger)separatorStyle {
    env.objc.borrow::<UITableViewHostObject>(this).separator_style
}
- (())setSeparatorStyle:(NSInteger)style {
    env.objc.borrow_mut::<UITableViewHostObject>(this).separator_style = style;
}

@end

};
