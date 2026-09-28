/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIToolbar`.

use crate::frameworks::foundation::NSInteger;
use crate::msg;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg_super, nil, objc_classes, release, retain,
    ClassExports, NSZonePtr,
};

pub struct UIToolbarHostObject {
    superclass: super::ui_view::UIViewHostObject,
    items: id,
    bar_style: NSInteger,
    translucent: bool,
}
impl_HostObject_with_superclass!(UIToolbarHostObject);

pub const CLASSES: ClassExports = objc_classes! {

    (env, this, _cmd);

    @implementation UIToolbar: UIView

    + (id)allocWithZone:(NSZonePtr)_zone {
        env.objc.alloc_object(
            this,
            Box::new(UIToolbarHostObject {
                superclass: Default::default(),
                items: nil,
                bar_style: 0,
                translucent: false,
            }),
            &mut env.mem
        )
    }

    - (id)initWithCoder:(id)coder {
        msg_super![env; this initWithCoder:coder]
    }

    - (())dealloc {
        let items = env.objc.borrow::<UIToolbarHostObject>(this).items;
        release(env, items);
        msg_super![env; this dealloc]
    }

    - (id)items {
        env.objc.borrow::<UIToolbarHostObject>(this).items
    }

    - (())setItems:(id)new_items {
        let toolbar = env.objc.borrow_mut::<UIToolbarHostObject>(this);
        let old_items = std::mem::replace(&mut toolbar.items, new_items);
        retain(env, new_items);
        release(env, old_items);
    }

    - (())setItems:(id)new_items animated:(bool)_animated {
        () = msg![env; this setItems:new_items];
    }

    - (NSInteger)barStyle {
        env.objc.borrow::<UIToolbarHostObject>(this).bar_style
    }

    - (())setBarStyle:(NSInteger)style {
        env.objc.borrow_mut::<UIToolbarHostObject>(this).bar_style = style;
    }

    - (bool)isTranslucent {
        env.objc.borrow::<UIToolbarHostObject>(this).translucent
    }

    - (())setTranslucent:(bool)translucent {
        env.objc.borrow_mut::<UIToolbarHostObject>(this).translucent = translucent;
    }

@end

};
