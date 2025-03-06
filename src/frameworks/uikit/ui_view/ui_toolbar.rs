/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIToolbar`.

use crate::frameworks::core_graphics::CGRect;
use crate::objc::{id, msg_super, objc_classes, ClassExports};

pub const CLASSES: ClassExports = objc_classes! {

    (env, this, _cmd);

    @implementation UIToolbar: UIView

    - (id)initWithFrame:(CGRect)frame {
        log!("TODO: [(UIToolbar*){:?} initWithFrame:{:?}]", this, frame);
        msg_super![env; this initWithFrame:frame]
    }

    - (())setItems:(id)items {
        log!("TODO: [(UIToolbar*){:?} setItems:{:?}]", this, items);
    }

    - (())setBarStyle:(id)style {
        log!("TODO: [(UIToolbar*){:?} setBarStyle:{:?}]", this, style);
    }

    @end

};
