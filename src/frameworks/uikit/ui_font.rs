//! `UIFont`.

use crate::frameworks::core_graphics::CGFloat;
use crate::objc::{autorelease, id, objc_classes, ClassExports, HostObject};

struct UIFontHostObject {
    _size: CGFloat,
}
impl HostObject for UIFontHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

// For now this is a singleton (the only instance is returned by mainScreen),
// so there are hardcoded assumptions related to that.
@implementation UIFont: NSObject

+ (id)systemFontOfSize:(CGFloat)font_size {
    // TODO: actually load and render fonts
    let new = env.objc.alloc_object(
        this,
        Box::new(UIFontHostObject {
            _size: font_size,
        }),
        &mut env.mem
    );
    autorelease(env, new)
}

@end

};
