/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIView`.

use crate::frameworks::core_graphics::{CGFloat, CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::{get_static_str, to_rust_string};
use crate::objc::{
    id, msg, nil, objc_classes, release, Class, ClassExports, HostObject, NSZonePtr,
};

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

+ (id)allocWithZone:(NSZonePtr)_zone {
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

// TODO: accessors etc

- (id)initWithFrame:(CGRect)frame {
    let center = CGPoint {
        x: frame.origin.x + frame.size.width / 2.0,
        y: frame.origin.y + frame.size.height / 2.0,
    };
    let bounds = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size: frame.size,
    };

    let host_object: &mut UIViewHostObject = env.objc.borrow_mut(this);
    host_object.bounds = bounds;
    host_object.center = center;

    log_dbg!(
        "[(UIView*){:?} initWithFrame:{:?}] => bounds {:?}, center {:?}",
        this,
        frame,
        bounds,
        center,
    );

    let layer = host_object.layer;
    () = msg![env; layer setDelegate:this];

    env.framework_state.uikit.ui_view.views.push(this);

    this
}

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

- (())addSubview:(id)view {
    // FIXME: there should be an actual hierarchy that retains the view
    log!("TODO: [(UIView*){:?} addSubview:{:?}]", this, view);
    // FIXME: These should be called systematically using setNeedsLayout: and
    //        layoutIfNeeded.
    let _: () = msg![env; this layoutSubviews];
    let _: () = msg![env; view layoutSubviews];
}

- (())bringSubviewToFront:(id)view {
    log_dbg!("TODO: [(UIView*){:?} bringSubviewToFront:{:?}]", this, view);
}

- (())removeFromSuperview {
    // FIXME: this should actually remove the view from some hierarchy and
    //        release it
    log!("TODO: [(UIView*){:?} removeFromSuperview]", this);
}

- (())dealloc {
    let &mut UIViewHostObject { layer, .. } = env.objc.borrow_mut(this);
    release(env, layer);

    env.framework_state.uikit.ui_view.views.swap_remove(
        env.framework_state.uikit.ui_view.views.iter().position(|&v| v == this).unwrap()
    );

    env.objc.dealloc_object(this, &mut env.mem);
}

- (id)layer {
    env.objc.borrow_mut::<UIViewHostObject>(this).layer
}

- (bool)opaque {
    true
}
- (())setOpaque:(bool)opaque {
    // TODO: implement this once views are actually rendered
    assert!(opaque);
}

- (CGFloat)alpha {
    1.0
}
- (())setAlpha:(CGFloat)_alpha {
    // TODO: implement this once views are actually rendered
}

- (id)backgroundColor {
    nil // this is the actual default (equivalent to transparency)
}
- (())setBackgroundColor:(id)_color { // UIColor*
    // TODO: implement this once views are actually rendered
}

// FIXME: should these pass through from CALayer, vice-versa, or neither?
// (See similar comment in ca_layer.rs)
- (CGRect)bounds {
    env.objc.borrow::<UIViewHostObject>(this).bounds
}
- (CGPoint)center {
    env.objc.borrow::<UIViewHostObject>(this).center
}

@end

@implementation UIAlertView: UIView
- (id)initWithTitle:(id)title
                      message:(id)message
                     delegate:(id)delegate
            cancelButtonTitle:(id)cancelButtonTitle
            otherButtonTitles:(id)otherButtonTitles {

    log!("TODO: [(UIAlertView*){:?} initWithTitle:{:?} message:{:?} delegate:{:?} cancelButtonTitle:{:?} otherButtonTitles:{:?}]", this, title, message, delegate, cancelButtonTitle, otherButtonTitles);

    let msg = to_rust_string(env, message);
    let title = to_rust_string(env, title);

    log!("UIAlertView: title: {:?}, message: {:?}", title, msg);

    let host_object: &mut UIViewHostObject = env.objc.borrow_mut(this);
    let layer = host_object.layer;
    () = msg![env; layer setDelegate:this];

    env.framework_state.uikit.ui_view.views.push(this);

    this
}
- (())show {
    log!("TODO: [(UIAlertView*){:?} show]", this);
}
@end

};
