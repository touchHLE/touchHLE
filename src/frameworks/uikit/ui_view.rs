/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIView`.

use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::{get_static_str, to_rust_string};
use crate::mem::MutVoidPtr;
use crate::objc::{id, msg, objc_classes, release, Class, ClassExports, HostObject};

#[derive(Default)]
pub struct State {
    pub(super) views: Vec<id>,
}

pub(super) struct UIViewHostObject {
    pub(super) bounds: CGRect,
    pub(super) center: CGPoint,
    /// CALayer or subclass.
    layer: id,
}
impl HostObject for UIViewHostObject {}

fn parse_tuple(string: &str) -> Option<(f32, f32)> {
    let (a, b) = string.split_once(", ")?;
    Some((a.parse().ok()?, b.parse().ok()?))
}
fn parse_point(string: &str) -> Option<CGPoint> {
    let (x, y) = parse_tuple(string.strip_prefix('{')?.strip_suffix('}')?)?;
    Some(CGPoint { x, y })
}
fn parse_rect(string: &str) -> Option<CGRect> {
    let string = string.strip_prefix("{{")?.strip_suffix("}}")?;
    let (a, b) = string.split_once("}, {")?;
    let (x, y) = parse_tuple(a)?;
    let (width, height) = parse_tuple(b)?;
    Some(CGRect {
        origin: CGPoint { x, y },
        size: CGSize { width, height },
    })
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIView: UIResponder

+ (id)allocWithZone:(MutVoidPtr)_zone {
    let layer_class: Class = msg![env; this layerClass];
    let layer: id = msg![env; layer_class layer];

    let host_object = Box::new(UIViewHostObject {
        bounds: CGRect {
            origin: CGPoint { x: 0.0, y: 0.0 },
            size: CGSize { width: 0.0, height: 0.0 }
        },
        center: CGPoint { x: 0.0, y: 0.0 },
        layer,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (Class)layerClass {
    env.objc.get_known_class("CALayer", &mut env.mem)
}

// TODO: initWithFrame:, accessors, etc

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    // TODO: there's a category on NSCoder for decoding CGRect and CGPoint, we
    //       should implement and use that
    // TODO: avoid copying strings
    // TODO: decode the various other UIView properties

    let key_ns_string = get_static_str(env, "UIBounds");
    let value = msg![env; coder decodeObjectForKey:key_ns_string];
    let bounds = parse_rect(&to_rust_string(env, value)).unwrap();

    let key_ns_string = get_static_str(env, "UICenter");
    let value = msg![env; coder decodeObjectForKey:key_ns_string];
    let center = parse_point(&to_rust_string(env, value)).unwrap();

    let host_object: &mut UIViewHostObject = env.objc.borrow_mut(this);
    host_object.bounds = bounds;
    host_object.center = center;

    log_dbg!(
        "[(UIView*){:?} initWithCoder:{:?}] => bounds {:?}, center {:?}",
        this,
        coder,
        bounds,
        center
    );

    let layer = host_object.layer;
    () = msg![env; layer setDelegate:this];

    env.framework_state.uikit.ui_view.views.push(this);

    this
}

// TODO: setMultipleTouchEnabled
- (())setMultipleTouchEnabled:(bool)_enabled {
    // TODO: enable multitouch
}

- (())layoutSubviews {
    // On iOS 5.1 and earlier, the default implementation of this method does nothing.
}

- (())dealloc {
    let &mut UIViewHostObject { layer, .. } = env.objc.borrow_mut(this);
    release(env, layer);

    env.framework_state.uikit.ui_view.views.swap_remove(
        env.framework_state.uikit.ui_view.views.iter().position(|&v| v == this).unwrap()
    );

    // FIXME: this should do a super-call instead
    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)layer {
    env.objc.borrow_mut::<UIViewHostObject>(this).layer
}

@end

};
