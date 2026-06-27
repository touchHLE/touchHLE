/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIRotationGestureRecognizer`.
//!
//! Базовый класс `UIGestureRecognizer` уже определён в
//! `ui_pinch_gesture_recognizer.rs`, поэтому здесь объявлен только подкласс
//! `UIRotationGestureRecognizer`. Он наследует базовое состояние, обработку
//! `target/action` и т. п. от родителя.
//!
//! Как и в случае с `UIPinchGestureRecognizer`, диспетчеризация распознавания
//! на основе реальных касаний пока не реализована (см. комментарий в
//! `ui_view.rs`). Вместо этого мы предоставляем рабочий объект класса с
//! корректным поведением свойств `rotation` / `velocity`, чтобы приложения
//! могли создавать жест, устанавливать цели/действия, прикреплять его к
//! `UIView`, читать и записывать значения углов и не падали на отсутствующем
//! классе.
//!
//! Документация Apple:
//! - <https://developer.apple.com/documentation/uikit/uirotationgesturerecognizer>
//! - Свойство `rotation` хранится в радианах, относительно начала жеста.
//! - Свойство `velocity` (только для чтения) выражается в радианах в секунду.
//!
//! Эта реализация повторяет структуру `UIPinchGestureRecognizer`.

use crate::frameworks::core_graphics::CGFloat;
use crate::objc::{id, objc_classes, ClassExports, HostObject, NSZonePtr};

// MARK: - UIRotationGestureRecognizer host object

#[derive(Default)]
struct UIRotationGestureRecognizerHostObject {
    /// Текущий угол поворота в радианах относительно начала распознавания
    /// жеста. По умолчанию 0.
    rotation: CGFloat,
    /// Скорость поворота в радианах в секунду. Свойство `velocity` доступно
    /// только для чтения в публичном API, но мы храним его, чтобы код,
    /// эмулирующий распознавание, мог его обновлять.
    velocity: CGFloat,
}
impl HostObject for UIRotationGestureRecognizerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// =========================================================================
// MARK: - UIRotationGestureRecognizer
// =========================================================================

@implementation UIRotationGestureRecognizer: UIGestureRecognizer

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIRotationGestureRecognizerHostObject {
        rotation: 0.0,
        velocity: 0.0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// MARK: - Свойства UIRotationGestureRecognizer

- (CGFloat)rotation {
    env.objc.borrow::<UIRotationGestureRecognizerHostObject>(this).rotation
}

- (())setRotation:(CGFloat)rotation {
    env.objc.borrow_mut::<UIRotationGestureRecognizerHostObject>(this).rotation = rotation;
}

- (CGFloat)velocity {
    env.objc.borrow::<UIRotationGestureRecognizerHostObject>(this).velocity
}

@end

};
