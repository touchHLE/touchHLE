/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::frameworks::uikit::ui_view_controller::UIViewControllerHostObject;
use crate::objc::{id, msg, nil, objc_classes, release, retain, ClassExports, NSZonePtr};
use crate::{impl_HostObject_with_superclass, msg_super};

#[derive(Default)]
struct UINavigationControllerHostObject {
    superclass: UIViewControllerHostObject,
    stack: Vec<id>,
    nav_bar_hidden: bool,
}
impl_HostObject_with_superclass!(UINavigationControllerHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UINavigationController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UINavigationControllerHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())setNavigationBarHidden:(bool)hidden {
    let host = env.objc.borrow_mut::<UINavigationControllerHostObject>(this);
    host.nav_bar_hidden = hidden;
}

-(id)topViewController {
    env.objc.borrow::<UINavigationControllerHostObject>(this).stack.last().cloned().unwrap_or(nil)
}

-(id)initWithRootViewController:(id)controller {
    retain(env, controller);
    let host = env.objc.borrow_mut::<UINavigationControllerHostObject>(this);
    host.stack.push(controller);
    let myView = msg![env; this view];
    let subView: id = msg![env; controller view];
    () = msg![env; myView addSubview: subView];

    this
}

-(())dealloc {
    let mut stack = std::mem::take(&mut env.objc.borrow_mut::<UINavigationControllerHostObject>(this).stack);
    for controller in stack.drain(..) {
        release(env, controller);
    }
    msg_super![env; this dealloc]
}

@end

};
