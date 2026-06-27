/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIActivityIndicatorView`.

use crate::frameworks::foundation::NSInteger;
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, objc_classes, ClassExports,
    NSZonePtr,
};

type UIActivityIndicatorViewStyle = NSInteger;

#[derive(Default)]
pub struct UIActivityIndicatorViewHostObject {
    superclass: super::ui_view::UIViewHostObject,
    animating: bool,
    hides_when_stopped: bool,
    style: UIActivityIndicatorViewStyle,
}
impl_HostObject_with_superclass!(UIActivityIndicatorViewHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIActivityIndicatorView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIActivityIndicatorViewHostObject {
        superclass: Default::default(),
        animating: false,
        hides_when_stopped: true, // По умолчанию в iOS это свойство равно YES
        style: 0, // UIActivityIndicatorViewStyleWhiteLarge
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithActivityIndicatorStyle:(UIActivityIndicatorViewStyle)_style {
    // Вызываем базовый init
    let _: id = msg![env; this init];
    // Устанавливаем стиль (пока просто заглушка, если нужно расширить)
    let _: () = msg![env; this setActivityIndicatorViewStyle: _style];
    this
}

- (())setActivityIndicatorViewStyle:(UIActivityIndicatorViewStyle)style {
    env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this).style = style;
}
- (UIActivityIndicatorViewStyle)activityIndicatorViewStyle {
    env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).style
}

- (())startAnimating {
    let host = env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this);
    host.animating = true;

    // Если индикатор запускается, он должен стать видимым
    let _: () = msg![env; this setHidden: false];

    log!("UIActivityIndicatorView: started animating [{:?}]", this);
}

- (())stopAnimating {
    let host = env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this);
    host.animating = false;
    let hides = host.hides_when_stopped;

    log!("UIActivityIndicatorView: stopped animating [{:?}]", this);

    // Если установлен флаг hidesWhenStopped, скрываем View
    if hides {
        let _: () = msg![env; this setHidden: true];
    }
}

- (bool)isAnimating {
    env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).animating
}

- (())setHidesWhenStopped:(bool)hides {
    env.objc.borrow_mut::<UIActivityIndicatorViewHostObject>(this).hides_when_stopped = hides;
}

- (bool)hidesWhenStopped {
    env.objc.borrow::<UIActivityIndicatorViewHostObject>(this).hides_when_stopped
}

@end

};
