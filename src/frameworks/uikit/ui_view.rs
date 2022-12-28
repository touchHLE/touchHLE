//! `UIView`.

use crate::frameworks::foundation::ns_string::{copy_string, string_with_rust_string};
use crate::mem::MutVoidPtr;
use crate::objc::{id, msg, objc_classes, ClassExports, HostObject};

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
    let mut bounds: Option<((f32, f32), (f32, f32))> = None;
    let mut center: Option<(f32, f32)> = None;
    for key in ["UIBounds", "UICenter"] {
        // TODO: add an easier and more efficient way to handle these keys!
        // UINib has the same problem.
        let key_ns_string = string_with_rust_string(env, key.to_string());
        let value = msg![env; coder decodeObjectForKey:key_ns_string];
        let _: () = msg![env; key_ns_string release];
        // TODO: avoid copy
        let copy = copy_string(env, value);
        let _: () = msg![env; value release];

        // TODO: there's a category on NSCoder for decoding these,
        // implement that
        if key == "UIBounds" {
            bounds = Some(parse_rect(&copy).unwrap());
        } else {
            center = Some(parse_point(&copy).unwrap());
        }
    }

    // TODO: decode the various other UIView properties

    let host_object: &mut UIViewHostObject = env.objc.borrow_mut(this);
    host_object.bounds = bounds.unwrap();
    host_object.center = center.unwrap();

    log_dbg!("[(UIView*){:?} initWithCoder:{:?}] => bounds {:?}, center {:?}", this, coder, bounds, center);

    this
}

@end

};
