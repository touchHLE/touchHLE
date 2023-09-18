/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIScrollView`.

pub mod ui_text_view;
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, objc_classes, ClassExports, NSZonePtr,
};

pub struct UIScrollViewHostObject {
    superclass: super::UIViewHostObject,
    scroll_enabled: bool,
    content_offset: CGPoint,
    content_size: CGSize,
}
impl_HostObject_with_superclass!(UIScrollViewHostObject);
impl Default for UIScrollViewHostObject {
    fn default() -> Self {
        UIScrollViewHostObject {
            superclass: Default::default(),
            scroll_enabled: true,
            content_offset: CGPoint { x: 0.0, y: 0.0 },
            content_size: CGSize {
                width: 0.0,
                height: 0.0,
            },
        }
    }
}
pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIScrollView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIScrollViewHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}
-(()) setContentSize: (CGSize) _content_size{

}
-(()) setDelaysContentTouches: (id) _delay_content_touches{

}
-(()) setBounces: (id) _bounces{

}
- (bool)scrollEnabled {
    env.objc.borrow::<UIScrollViewHostObject>(this).scroll_enabled
}

-(()) setScrollEnabled: (bool)scroll_enabled {
    env.objc.borrow_mut::<UIScrollViewHostObject>(this).scroll_enabled = scroll_enabled;
}

- (CGPoint)contentOffset {
    env.objc.borrow::<UIScrollViewHostObject>(this).content_offset
}

- (())setContentOffset: (CGPoint)offset {
    env.objc.borrow_mut::<UIScrollViewHostObject>(this).content_offset = offset;
    () = msg![env; this setNeedsDisplay];
}

- (CGSize)contentSize {
    env.objc.borrow::<UIScrollViewHostObject>(this).content_size
}

- (())setContentSize: (CGSize)size {
    env.objc.borrow_mut::<UIScrollViewHostObject>(this).content_size = size;
}

- (())touchesMoved:(id)touches // NSSet* of UITouch*
    withEvent:(id)_event { // UIEvent*

    let scroll_enabled: bool = msg![env; this scrollEnabled];
    if !scroll_enabled
    {
        return;
    }
    let toucharr: id = msg![env; touches allObjects];
    // Assume single finger touches for now
    let touch: id = msg![env; toucharr objectAtIndex: 0];
    let bounds: CGRect = msg![env; this bounds];

    let prevlocation: CGPoint = msg![env; touch previousLocationInView: this];
    let prevx = prevlocation.x;
    let prevy = prevlocation.y;

    let newlocation: CGPoint = msg![env; touch locationInView: this];
    let y = newlocation.y;
    let x = newlocation.x;

    let deltay = y - prevy;
    let deltax = x - prevx;

    let offset: CGPoint = msg![env; this contentOffset];
    let content_size: CGSize = msg![env; this contentSize];

    // Very rudimentary scrolling. We emulate sliding up to scroll down like on the real iPhone.
    let mut newcontentoffset: CGPoint = CGPoint{x: offset.x-deltax, y:offset.y-deltay};

    // Update content offset within bounds
    newcontentoffset.y = newcontentoffset.y.min(content_size.height-bounds.size.height).max(0.0);
    newcontentoffset.x = newcontentoffset.x.min(content_size.width-bounds.size.width).max(0.0);

    // Trigger rerender only if required.
    if newcontentoffset !=  msg![env; this contentOffset]{
        () = msg![env; this setContentOffset: newcontentoffset];
        () = msg![env; this setNeedsDisplay];
    }
}

@end

};
