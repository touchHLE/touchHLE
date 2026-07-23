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
    id, impl_HostObject_with_superclass, msg, msg_super, nil, objc_classes, release, retain,
    ClassExports, NSZonePtr,
};
use crate::Environment;
use std::time::Instant;

#[derive(Default)]
pub struct State {
    /// `UIImageView*` instances that are currently animating.
    /// These are retained while in this list.
    animating_views: Vec<id>,
}

#[derive(Default)]
struct UIImageViewHostObject {
    superclass: super::UIViewHostObject,
    /// `UIImage*`
    image: id,
    /// `NSArray<UIImage *>*`
    animation_images: id,
    /// Total duration of one animation cycle. `0.0` means the default
    /// (number of frames at 30fps).
    animation_duration: NSTimeInterval,
    /// Number of times to repeat the animation. `0` means repeat forever.
    animation_repeat_count: NSInteger,
    /// Set while the view is animating.
    animation_start: Option<Instant>,
    /// Index of the animation frame currently displayed, to avoid redundant
    /// layer updates.
    current_frame: Option<NSUInteger>,
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
    // Should never happen while the view is in the animating list (the list
    // retains it), but be careful anyway.
    let state = &mut env.framework_state.uikit.ui_view.ui_image_view;
    state.animating_views.retain(|&view| view != this);

    let &UIImageViewHostObject {
        superclass: _,
        image,
        animation_images,
        ..
    } = env.objc.borrow(this);
    release(env, image);
    release(env, animation_images);
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

    // While animating, the animation frames take precedence over the image.
    if env.objc.borrow::<UIImageViewHostObject>(this).animation_start.is_none() {
        set_layer_contents(env, this, new_image);
    }
}

- (id)animationImages {
    env.objc.borrow::<UIImageViewHostObject>(this).animation_images
}

- (())setAnimationImages:(id)images { // NSArray<UIImage *>*
    let host_obj = env.objc.borrow_mut::<UIImageViewHostObject>(this);
    let old_images = std::mem::replace(&mut host_obj.animation_images, images);
    retain(env, images);
    release(env, old_images);
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

- (())setAnimationRepeatCount:(NSInteger)count {
    env.objc.borrow_mut::<UIImageViewHostObject>(this).animation_repeat_count = count;
}

- (bool)isAnimating {
    env.objc.borrow::<UIImageViewHostObject>(this).animation_start.is_some()
}

- (())startAnimating {
    let images = env.objc.borrow::<UIImageViewHostObject>(this).animation_images;
    if images == nil {
        return;
    }
    let image_count: NSUInteger = msg![env; images count];
    if image_count == 0 {
        return;
    }

    let host_obj = env.objc.borrow_mut::<UIImageViewHostObject>(this);
    if host_obj.animation_start.is_some() {
        return;
    }
    host_obj.animation_start = Some(Instant::now());
    host_obj.current_frame = None;

    retain(env, this);
    let state = &mut env.framework_state.uikit.ui_view.ui_image_view;
    state.animating_views.push(this);

    // Show the first frame immediately.
    update_animation(env, this);
}

- (())stopAnimating {
    let host_obj = env.objc.borrow_mut::<UIImageViewHostObject>(this);
    if host_obj.animation_start.take().is_none() {
        return;
    }
    host_obj.current_frame = None;

    let state = &mut env.framework_state.uikit.ui_view.ui_image_view;
    state.animating_views.retain(|&view| view != this);

    // Restore the normal image.
    let image = env.objc.borrow::<UIImageViewHostObject>(this).image;
    set_layer_contents(env, this, image);

    release(env, this);
}

@end

};

fn set_layer_contents(env: &mut Environment, this: id, image: id) {
    let layer: id = msg![env; this layer];
    let cg_image: CGImageRef = if image == nil {
        CGImageRef::null()
    } else {
        msg![env; image CGImage]
    };
    () = msg![env; layer setContents:cg_image];
}

/// Advance a single animating image view; stops the animation if it has
/// played for the requested number of repeats.
fn update_animation(env: &mut Environment, this: id) {
    let images = env
        .objc
        .borrow::<UIImageViewHostObject>(this)
        .animation_images;
    if images == nil {
        () = msg![env; this stopAnimating];
        return;
    }
    let image_count: NSUInteger = msg![env; images count];
    if image_count == 0 {
        () = msg![env; this stopAnimating];
        return;
    }

    let host_obj = env.objc.borrow::<UIImageViewHostObject>(this);
    let Some(start) = host_obj.animation_start else {
        return;
    };
    let duration = if host_obj.animation_duration > 0.0 {
        host_obj.animation_duration
    } else {
        // Default: number of images divided by 30 fps
        image_count as NSTimeInterval / 30.0
    };
    let repeat_count = host_obj.animation_repeat_count;

    let elapsed = start.elapsed().as_secs_f64();
    let cycles = elapsed / duration;
    if repeat_count > 0 && cycles >= repeat_count as f64 {
        () = msg![env; this stopAnimating];
        return;
    }

    let progress = (elapsed / duration).fract();
    let frame_idx = ((progress * image_count as f64) as NSUInteger).min(image_count - 1);

    if env.objc.borrow::<UIImageViewHostObject>(this).current_frame == Some(frame_idx) {
        return;
    }
    env.objc
        .borrow_mut::<UIImageViewHostObject>(this)
        .current_frame = Some(frame_idx);

    let frame: id = msg![env; images objectAtIndex:frame_idx];
    set_layer_contents(env, this, frame);
}

/// For use by the Core Animation compositor: advance all animating image
/// views. Call this before compositing a frame.
pub fn update_animations(env: &mut Environment) {
    let views = env
        .framework_state
        .uikit
        .ui_view
        .ui_image_view
        .animating_views
        .clone();
    for view in views {
        update_animation(env, view);
    }
}
