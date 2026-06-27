/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UISplitViewController`.

use crate::objc::{id, nil, objc_classes, ClassExports, HostObject, NSZonePtr};

// Структура для хранения состояния нашего контроллера
#[derive(Default)]
struct UISplitViewControllerHostObject {
    view_controllers: id,
    delegate: id,
}
impl HostObject for UISplitViewControllerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// Наследуемся от UIViewController
@implementation UISplitViewController: UIViewController

// Переопределяем аллокацию, чтобы привязать наш HostObject с состоянием
+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UISplitViewControllerHostObject {
        view_controllers: nil,
        delegate: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: - Properties

- (id)viewControllers {
    env.objc.borrow::<UISplitViewControllerHostObject>(this).view_controllers
}

- (())setViewControllers:(id)view_controllers {
    // В реальном iOS тут был бы retain старого и release нового массива,
    // но в рамках эмуляции пока достаточно просто сохранить ссылку.
    env.objc.borrow_mut::<UISplitViewControllerHostObject>(this).view_controllers = view_controllers;
}

- (id)delegate {
    env.objc.borrow::<UISplitViewControllerHostObject>(this).delegate
}

- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<UISplitViewControllerHostObject>(this).delegate = delegate;
}

@end

};
