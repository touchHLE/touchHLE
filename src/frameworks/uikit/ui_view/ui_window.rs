/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIWindow`.

use crate::frameworks::core_graphics::CGRect;
use crate::objc::{id, msg, msg_super, objc_classes, ClassExports};

#[derive(Default)]
pub struct State {
    /// List of visible windows for internal purposes. Non-retaining!
    ///
    /// This is public because Core Animation also uses it.
    pub visible_windows: Vec<id>,
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIWindow: UIView

// TODO: more?

- (id)initWithFrame:(CGRect)frame {
    // setHidden: may get called during the super call and panics if the window
    // is not in the list, so it must be added to it before that call.
    let visible_list = &mut env.framework_state.uikit.ui_view.ui_window.visible_windows;
    visible_list.push(this);
    log_dbg!(
        "New window: {:?}. New set of visible windows: {:?}",
        this,
        visible_list,
    );

    msg_super![env; this initWithFrame:frame]
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    // setHidden: may get called during the super call and panics if the window
    // is not in the list, so it must be added to it before that call.
    let visible_list = &mut env.framework_state.uikit.ui_view.ui_window.visible_windows;
    visible_list.push(this);
    log_dbg!(
        "New window: {:?}. New set of visible windows: {:?}",
        this,
        visible_list,
    );

    msg_super![env; this initWithCoder:coder]
}

- (())dealloc {
    if !msg![env; this isHidden] {
        let visible_list = &mut env.framework_state.uikit.ui_view.ui_window.visible_windows;
        let idx = visible_list.iter().position(|&w| w == this).unwrap();
        visible_list.remove(idx);
        log_dbg!(
            "Deallocating window {:?}. New set of visible windows: {:?}",
            this,
            visible_list,
        );
    }
    msg_super![env; this dealloc]
}

- (())setHidden:(bool)is_hidden {
    let was_hidden: bool = msg![env; this isHidden];
    () = msg_super![env; this setHidden:is_hidden];

    let visible_list = &mut env.framework_state.uikit.ui_view.ui_window.visible_windows;
    if is_hidden && !was_hidden {
        let idx = visible_list.iter().position(|&w| w == this).unwrap();
        visible_list.remove(idx);
        log_dbg!(
            "Window {:?} is now hidden. New set of visible windows: {:?}",
            this,
            visible_list,
        );
    } else if !is_hidden && was_hidden {
        visible_list.push(this);
        log_dbg!(
            "Window {:?} is no longer hidden. New set of visible windows: {:?}",
            this,
            visible_list,
        );
    }
}

- (())makeKeyAndVisible {
    // TODO: Set the "key" window once it's relevant. We don't currently have
    // send any non-touch events to windows, so there's no meaning in it yet.

    msg![env; this setHidden:false]
}

@end

};
