/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIImageView`.

use crate::frameworks::core_graphics::cg_image::CGImageRef;
use crate::frameworks::core_graphics::{CGPoint, CGRect, CGSize};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::{NSInteger, NSTimeInterval, NSUInteger};
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_class, msg_super, nil, objc_classes, release,
    retain, ClassExports, NSZonePtr,
};

#[derive(Default)]
struct UIImageViewHostObject {
    superclass: super::UIViewHostObject,
    /// `UIImage*`
    image: id,
    /// `NSArray<UIImage *>*`
    animation_images: id,
    animation_duration: NSTimeInterval,
    animation_repeat_count: NSInteger,
    is_animating: bool,
    /// `NSTimer*` driving frame advancement while animating (nil when stopped).
    animation_timer: id,
    /// Index of the frame currently shown during animation.
    current_frame: NSUInteger,
    /// Number of complete loops played so far (used to honour
    /// `animationRepeatCount`).
    completed_loops: NSInteger,
    highlighted: bool,
    highlighted_image: id,
    highlighted_animation_images: id,
}
impl_HostObject_with_superclass!(UIImageViewHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIImageView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIImageViewHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame {
    let this: id = msg_super![env; this initWithFrame:frame];
    // Not sure if UIImageView does this unconditionally, or only for images
    // with alpha channels.
    () = msg![env; this setOpaque:false];
    this
}

- (())dealloc {
    // Stop and tear down any running animation timer first so it can't fire
    // against a freed view.
    () = msg![env; this stopAnimating];
    let &UIImageViewHostObject {
        superclass: _,
        image,
        animation_images,
        highlighted_image,
        highlighted_animation_images,
        ..
    } = env.objc.borrow(this);
    release(env, image);
    release(env, animation_images);
    release(env, highlighted_image);
    release(env, highlighted_animation_images);
    msg_super![env; this dealloc]
}

// NSCoding implementation
- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];

    let key_ns_string = get_static_str(env, "UIImage");
    let image: id = msg![env; coder decodeObjectForKey:key_ns_string];

    () = msg![env; this setImage:image];

    this
}

- (id)initWithImage:(id)image { // UIImage*
    let size: CGSize = msg![env; image size];
    let frame = CGRect {
        origin: CGPoint { x: 0.0, y: 0.0 },
        size
    };
    let this = msg_super![env; this initWithFrame:frame];
    () = msg![env; this setImage:image];
    // Not sure if UIImageView does this unconditionally, or only for images
    // with alpha channels.
    () = msg![env; this setOpaque:false];
    this
}

- (id)image {
    env.objc.borrow::<UIImageViewHostObject>(this).image
}

- (())setImage:(id)new_image { // UIImage*
    let host_obj = env.objc.borrow_mut::<UIImageViewHostObject>(this);
    let old_image = std::mem::replace(&mut host_obj.image, new_image);
    retain(env, new_image);
    release(env, old_image);

    let layer: id = msg![env; this layer];
    let cg_image: CGImageRef = msg![env; new_image CGImage];
    () = msg![env; layer setContents:cg_image];
}


- (bool)isHighlighted {
    env.objc.borrow::<UIImageViewHostObject>(this).highlighted
}
- (bool)highlighted {
    env.objc.borrow::<UIImageViewHostObject>(this).highlighted
}
- (())setHighlighted:(bool)highlighted {
    env.objc.borrow_mut::<UIImageViewHostObject>(this).highlighted = highlighted;
    let display_image = {
        let host = env.objc.borrow::<UIImageViewHostObject>(this);
        if highlighted && host.highlighted_image != nil {
            host.highlighted_image
        } else {
            host.image
        }
    };
    if display_image != nil {
        let layer: id = msg![env; this layer];
        let cg_image: CGImageRef = msg![env; display_image CGImage];
        () = msg![env; layer setContents:cg_image];
    }
}

- (id)highlightedImage {
    env.objc.borrow::<UIImageViewHostObject>(this).highlighted_image
}
- (())setHighlightedImage:(id)new_image {
    let old_image = std::mem::replace(
        &mut env.objc.borrow_mut::<UIImageViewHostObject>(this).highlighted_image,
        new_image,
    );
    retain(env, new_image);
    release(env, old_image);
    if env.objc.borrow::<UIImageViewHostObject>(this).highlighted {
        () = msg![env; this setHighlighted:true];
    }
}

- (id)highlightedAnimationImages {
    env.objc.borrow::<UIImageViewHostObject>(this).highlighted_animation_images
}
- (())setHighlightedAnimationImages:(id)images {
    let old_images = std::mem::replace(
        &mut env.objc.borrow_mut::<UIImageViewHostObject>(this).highlighted_animation_images,
        images,
    );
    retain(env, images);
    release(env, old_images);
}

// MARK: - Animation Properties

- (id)animationImages { // NSArray<UIImage *>*
    env.objc.borrow::<UIImageViewHostObject>(this).animation_images
}

- (())setAnimationImages:(id)images { // NSArray<UIImage *>*
    let host_obj = env.objc.borrow_mut::<UIImageViewHostObject>(this);
    let old_images = std::mem::replace(&mut host_obj.animation_images, images);
    retain(env, images);
    release(env, old_images);

    // Show the first frame so the view isn't blank before/after animating,
    // matching UIKit (which displays `image`, or the first animation frame).
    if images != nil {
        let count: NSUInteger = msg![env; images count];
        if count > 0 {
            let first_image: id = msg![env; images objectAtIndex:0u32];
            () = msg![env; this setImage:first_image];
        }
    }
}

- (NSTimeInterval)animationDuration {
    env.objc.borrow::<UIImageViewHostObject>(this).animation_duration
}

- (())setAnimationDuration:(NSTimeInterval)duration {
    env.objc.borrow_mut::<UIImageViewHostObject>(this).animation_duration = duration;
}

- (NSInteger)animationRepeatCount {
    env.objc.borrow::<UIImageViewHostObject>(this).animation_repeat_count
}

- (())setAnimationRepeatCount:(NSInteger)repeat_count {
    env.objc.borrow_mut::<UIImageViewHostObject>(this).animation_repeat_count = repeat_count;
}

// MARK: - Animation Controls

- (bool)isAnimating {
    env.objc.borrow::<UIImageViewHostObject>(this).is_animating
}

- (())startAnimating {
    // Pick the active frame list (highlighted variant takes precedence when
    // the view is highlighted and one was provided), matching UIKit.
    let (frames, highlighted) = {
        let host = env.objc.borrow::<UIImageViewHostObject>(this);
        let frames = if host.highlighted && host.highlighted_animation_images != nil {
            host.highlighted_animation_images
        } else {
            host.animation_images
        };
        (frames, host.highlighted)
    };
    let _ = highlighted;
    if frames == nil {
        env.objc.borrow_mut::<UIImageViewHostObject>(this).is_animating = true;
        return;
    }
    let count: NSUInteger = msg![env; frames count];
    if count == 0 {
        env.objc.borrow_mut::<UIImageViewHostObject>(this).is_animating = true;
        return;
    }

    // If we're already animating, restart cleanly.
    () = msg![env; this stopAnimating];

    // Per Apple's docs, when `animationDuration` is 0 the default is
    // `frameCount / 30.0` seconds (i.e. 30 fps). Each frame is shown for
    // `duration / frameCount` seconds.
    let duration = {
        let host = env.objc.borrow::<UIImageViewHostObject>(this);
        if host.animation_duration > 0.0 {
            host.animation_duration
        } else {
            count as NSTimeInterval / 30.0
        }
    };
    let per_frame: NSTimeInterval = (duration / count as NSTimeInterval).max(0.0001);

    {
        let host = env.objc.borrow_mut::<UIImageViewHostObject>(this);
        host.is_animating = true;
        host.current_frame = 0;
        host.completed_loops = 0;
    }

    // Show the first frame immediately.
    let first_image: id = msg![env; frames objectAtIndex:0u32];
    () = msg![env; this setImage:first_image];

    // Drive subsequent frames with a repeating NSTimer targeting this view.
    let sel = env
        .objc
        .lookup_selector("_touchHLE_advanceAnimationFrame:")
        .expect("UIImageView animation selector must be registered");
    let timer: id = msg_class![env; NSTimer
        scheduledTimerWithTimeInterval:per_frame
        target:this
        selector:sel
        userInfo:nil
        repeats:true];
    retain(env, timer);
    env.objc.borrow_mut::<UIImageViewHostObject>(this).animation_timer = timer;
}

- (())stopAnimating {
    let timer = {
        let host = env.objc.borrow_mut::<UIImageViewHostObject>(this);
        host.is_animating = false;
        std::mem::replace(&mut host.animation_timer, nil)
    };
    if timer != nil {
        () = msg![env; timer invalidate];
        release(env, timer);
    }
}

// Private: advances to the next animation frame. Invoked by the repeating
// NSTimer created in `startAnimating`.
- (())_touchHLE_advanceAnimationFrame:(id)_timer {
    let (frames, repeat_count) = {
        let host = env.objc.borrow::<UIImageViewHostObject>(this);
        if !host.is_animating {
            (nil, 0)
        } else {
            let frames = if host.highlighted && host.highlighted_animation_images != nil {
                host.highlighted_animation_images
            } else {
                host.animation_images
            };
            (frames, host.animation_repeat_count)
        }
    };
    if frames == nil {
        return;
    }
    let count: NSUInteger = msg![env; frames count];
    if count == 0 {
        return;
    }

    let next_frame = {
        let host = env.objc.borrow_mut::<UIImageViewHostObject>(this);
        host.current_frame += 1;
        if host.current_frame >= count {
            host.current_frame = 0;
            host.completed_loops += 1;
        }
        host.current_frame
    };

    // Honour a finite repeat count: stop once the requested number of loops
    // has completed. `animationRepeatCount == 0` means "loop forever".
    if repeat_count > 0 {
        let completed = env.objc.borrow::<UIImageViewHostObject>(this).completed_loops;
        if completed >= repeat_count {
            () = msg![env; this stopAnimating];
            return;
        }
    }

    let frame_image: id = msg![env; frames objectAtIndex:next_frame];
    () = msg![env; this setImage:frame_image];
}

@end

};
