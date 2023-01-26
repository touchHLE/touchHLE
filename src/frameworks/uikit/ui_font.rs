//! `UIFont`.

use crate::font::Font;
use crate::frameworks::core_graphics::{CGFloat, CGSize};
use crate::frameworks::foundation::NSInteger;
use crate::objc::{autorelease, id, objc_classes, ClassExports, HostObject};
use crate::Environment;

struct UIFontHostObject {
    size: CGFloat,
    font: Font,
}
impl HostObject for UIFontHostObject {}

/// Line break mode.
///
/// This is put here for convenience since it's font-related.
/// Apple puts it in its own header, also in UIKit.
pub type NSLineBreakMode = NSInteger;
#[allow(dead_code)]
pub const NSLineBreakByWordWrapping: NSLineBreakMode = 0;
#[allow(dead_code)]
pub const NSLineBreakByCharWrapping: NSLineBreakMode = 1;
#[allow(dead_code)]
pub const NSLineBreakByClipping: NSLineBreakMode = 3;
#[allow(dead_code)]
pub const NSLineBreakByTruncatingHead: NSLineBreakMode = 4;
#[allow(dead_code)]
pub const NSLineBreakByTruncatingTail: NSLineBreakMode = 5;
#[allow(dead_code)]
pub const NSLineBreakByTruncatingMiddle: NSLineBreakMode = 6;

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIFont: NSObject

// TODO: cache these fonts in case an app uses them in multiple sizes
+ (id)systemFontOfSize:(CGFloat)size {
    let host_object = UIFontHostObject {
        size,
        font: Font::sans_regular(),
    };
    let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    autorelease(env, new)
}
+ (id)boldSystemFontOfSize:(CGFloat)size {
    let host_object = UIFontHostObject {
        size,
        font: Font::sans_bold(),
    };
    let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    autorelease(env, new)
}
+ (id)italicSystemFontOfSize:(CGFloat)size {
    let host_object = UIFontHostObject {
        size,
        font: Font::sans_italic(),
    };
    let new = env.objc.alloc_object(this, Box::new(host_object), &mut env.mem);
    autorelease(env, new)
}

@end

};

/// Called by the `sizeWithFont:` method family on `NSString`.
pub fn size_with_font(
    env: &mut Environment,
    font: id,
    text: &str,
    _constrained: Option<(CGSize, NSLineBreakMode)>,
) -> CGSize {
    let host_object = env.objc.borrow::<UIFontHostObject>(font);

    // FIXME: line break support

    let (width, height) = host_object.font.calculate_text_size(host_object.size, text);

    CGSize { width, height }
}
