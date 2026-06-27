/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIScreenMode`.

use crate::frameworks::core_graphics::{CGFloat, CGSize};
use crate::objc::{id, objc_classes, ClassExports, HostObject, NSZonePtr};

#[derive(Default)]
pub struct UIScreenModeHostObject {
    pub size: CGSize,
    pub pixel_aspect_ratio: CGFloat,
}
impl HostObject for UIScreenModeHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIScreenMode: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(UIScreenModeHostObject {
        size: CGSize { width: 320.0, height: 480.0 },
        pixel_aspect_ratio: 1.0,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())dealloc {
    env.objc.dealloc_object(this, &mut env.mem)
}

// MARK: - Properties

- (CGSize)size {
    env.objc.borrow::<UIScreenModeHostObject>(this).size
}

- (CGFloat)pixelAspectRatio {
    env.objc.borrow::<UIScreenModeHostObject>(this).pixel_aspect_ratio
}

// MARK: - NSObject

- (bool)isEqual:(id)other {
    if this == other { return true; }
    let a = env.objc.borrow::<UIScreenModeHostObject>(this);
    let b = env.objc.borrow::<UIScreenModeHostObject>(other);
    a.size.width  == b.size.width
        && a.size.height == b.size.height
        && a.pixel_aspect_ratio == b.pixel_aspect_ratio
}

- (id)description {
    let host = env.objc.borrow::<UIScreenModeHostObject>(this);

    // Копируем значения из упакованной структуры в локальные переменные,
    // чтобы макрос format! не пытался взять невыровненную ссылку.
    let width = host.size.width;
    let height = host.size.height;
    let pixel_aspect_ratio = host.pixel_aspect_ratio;

    let s = format!(
        "<UIScreenMode: size=({}, {}), pixelAspectRatio={}>",
        width, height, pixel_aspect_ratio
    );
    let ns = crate::frameworks::foundation::ns_string::from_rust_string(env, s);
    crate::objc::autorelease(env, ns)
}

@end

};

/// Allocate a `UIScreenMode` with a specific size and pixel aspect ratio.
/// For use by `UIScreen` when building its `availableModes` array.
pub fn from_size(env: &mut crate::Environment, size: CGSize, pixel_aspect_ratio: CGFloat) -> id {
    let mode: id = crate::objc::msg_class![env; UIScreenMode alloc];
    let host = env.objc.borrow_mut::<UIScreenModeHostObject>(mode);
    host.size = size;
    host.pixel_aspect_ratio = pixel_aspect_ratio;
    crate::objc::autorelease(env, mode)
}
