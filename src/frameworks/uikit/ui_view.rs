//! `UIView`.

use crate::frameworks::foundation::ns_string::{copy_string, string_with_static_str};
use crate::mem::MutVoidPtr;
use crate::objc::{id, msg, objc_classes, release, ClassExports, HostObject};

struct UIViewHostObject {
    bounds: ((f32, f32), (f32, f32)), // TODO: should use CGRect
    center: (f32, f32),               // TODO: should use CGPoint
}
impl HostObject for UIViewHostObject {}

fn parse_tuple(string: &str) -> Option<(f32, f32)> {
    let (a, b) = string.split_once(", ")?;
    Some((a.parse().ok()?, b.parse().ok()?))
}
fn parse_point(string: &str) -> Option<(f32, f32)> {
    parse_tuple(string.strip_prefix('{')?.strip_suffix('}')?)
}
fn parse_rect(string: &str) -> Option<((f32, f32), (f32, f32))> {
    let string = string.strip_prefix("{{")?.strip_suffix("}}")?;
    let (a, b) = string.split_once("}, {")?;
    Some((parse_tuple(a)?, parse_tuple(b)?))
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIView: UIResponder

+ (id)allocWithZone:(MutVoidPtr)_zone {
    let host_object = Box::new(UIViewHostObject {
        bounds: ((0.0, 0.0), (0.0, 0.0)),
        center: (0.0, 0.0),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

// TODO: initWithFrame:, accessors, etc

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    // TODO: there's a category on NSCoder for decoding CGRect and CGPoint, we
    //       should implement and use that
    // TODO: avoid copying strings
    // TODO: decode the various other UIView properties

    let key_ns_string = string_with_static_str(env, "UIBounds");
    let value = msg![env; coder decodeObjectForKey:key_ns_string];
    let bounds = parse_rect(&copy_string(env, value)).unwrap();
    release(env, value);

    let key_ns_string = string_with_static_str(env, "UICenter");
    let value = msg![env; coder decodeObjectForKey:key_ns_string];
    let center = parse_point(&copy_string(env, value)).unwrap();
    release(env, value);

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

    this
}

@end

};
