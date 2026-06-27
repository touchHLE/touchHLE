/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! GLKit.
//!
//! Apple documents `GLKView` as "a default implementation for views that
//! draw their content using OpenGL ES" and `GLKViewController` as "a view
//! controller that manages an OpenGL ES rendering loop":
//! <https://developer.apple.com/documentation/glkit/glkview>
//! <https://developer.apple.com/documentation/glkit/glkviewcontroller>
//!
//! touchHLE does not implement the full GLKit stack, but apps built against
//! iOS 5+ commonly link the framework and allocate these classes at startup
//! (previously this hit the `UnimplementedClass` "SUPER HACK" warning and
//! left the app without a working render loop). This module provides real
//! class implementations with the documented inheritance (`GLKView:
//! UIView`, `GLKViewController: UIViewController`) and the documented
//! properties, plus the `GLKMatrix4Identity` constant that math-only users
//! of GLKit import.

use crate::dyld::{ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::core_graphics::CGRect;
use crate::frameworks::foundation::{NSInteger, NSTimeInterval};
use crate::frameworks::uikit::ui_view::UIViewHostObject;
use crate::frameworks::uikit::ui_view_controller::UIViewControllerHostObject;
use crate::mem::{ConstVoidPtr, SafeRead};
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, nil, objc_classes, release, retain, ClassExports,
    NSZonePtr,
};
use crate::Environment;

// GLKMatrix4Identity — 4x4 identity matrix of 32-bit floats (16 × 4 = 64
// bytes). Layout matches Apple's `_GLKMatrix4 { float m[16]; }`.
#[repr(C)]
struct GLKMatrix4 {
    m: [f32; 16],
}
unsafe impl SafeRead for GLKMatrix4 {}

fn glk_matrix4_identity(env: &mut Environment) -> ConstVoidPtr {
    let identity = GLKMatrix4 {
        m: [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0, //
        ],
    };
    env.mem.alloc_and_write(identity).cast_void().cast_const()
}

pub const CONSTANTS: ConstantExports = &[(
    "_GLKMatrix4Identity",
    HostConstant::Custom(glk_matrix4_identity),
)];

pub const FUNCTIONS: FunctionExports = &[];

/// `GLKViewDrawableColorFormat` etc. are plain integer enums in GLKit's
/// headers (`GLKView.h`); `GLKViewDrawableColorFormatRGBA8888` (the
/// documented default for `drawableColorFormat`) is 0, as are the
/// "FormatNone" defaults of the depth/stencil/multisample properties.
type GLKViewDrawableFormat = NSInteger;

struct GLKViewHostObject {
    superclass: UIViewHostObject,
    /// `EAGLContext*` — retained (documented as a strong property).
    context: id,
    /// `id<GLKViewDelegate>` — NOT retained. Apple documents the property
    /// as `unowned(unsafe)`.
    delegate: id,
    drawable_color_format: GLKViewDrawableFormat,
    drawable_depth_format: GLKViewDrawableFormat,
    drawable_stencil_format: GLKViewDrawableFormat,
    drawable_multisample: GLKViewDrawableFormat,
    enable_set_needs_display: bool,
}
impl_HostObject_with_superclass!(GLKViewHostObject);
impl Default for GLKViewHostObject {
    fn default() -> Self {
        GLKViewHostObject {
            superclass: Default::default(),
            context: nil,
            delegate: nil,
            // Defaults documented in GLKView.h: RGBA8888 color, no depth,
            // no stencil, no multisampling.
            drawable_color_format: 0,
            drawable_depth_format: 0,
            drawable_stencil_format: 0,
            drawable_multisample: 0,
            // Apple docs: "the default value is YES".
            enable_set_needs_display: true,
        }
    }
}

struct GLKViewControllerHostObject {
    superclass: UIViewControllerHostObject,
    /// `id<GLKViewControllerDelegate>` — NOT retained
    /// (`unowned(unsafe)` per Apple docs).
    delegate: id,
    preferred_frames_per_second: NSInteger,
    pause_on_will_resign_active: bool,
    resume_on_did_become_active: bool,
    paused: bool,
    frames_displayed: NSInteger,
}
impl_HostObject_with_superclass!(GLKViewControllerHostObject);
impl Default for GLKViewControllerHostObject {
    fn default() -> Self {
        GLKViewControllerHostObject {
            superclass: Default::default(),
            delegate: nil,
            // Apple docs: "The default value is 30" frames per second.
            preferred_frames_per_second: 30,
            // Apple docs: both pause/resume-on-app-state flags default
            // to YES.
            pause_on_will_resign_active: true,
            resume_on_did_become_active: true,
            paused: false,
            frames_displayed: 0,
        }
    }
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation GLKView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<GLKViewHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithFrame:(CGRect)frame context:(id)context {
    let this: id = msg![env; this initWithFrame:frame];
    () = msg![env; this setContext:context];
    this
}

- (id)context {
    env.objc.borrow::<GLKViewHostObject>(this).context
}
- (())setContext:(id)context { // EAGLContext*
    let old = env.objc.borrow::<GLKViewHostObject>(this).context;
    if context != nil {
        retain(env, context);
    }
    env.objc.borrow_mut::<GLKViewHostObject>(this).context = context;
    if old != nil {
        release(env, old);
    }
}

- (id)delegate {
    env.objc.borrow::<GLKViewHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    // Apple documents this property as unowned(unsafe): do not retain.
    env.objc.borrow_mut::<GLKViewHostObject>(this).delegate = delegate;
}

- (GLKViewDrawableFormat)drawableColorFormat {
    env.objc.borrow::<GLKViewHostObject>(this).drawable_color_format
}
- (())setDrawableColorFormat:(GLKViewDrawableFormat)format {
    env.objc.borrow_mut::<GLKViewHostObject>(this).drawable_color_format = format;
}
- (GLKViewDrawableFormat)drawableDepthFormat {
    env.objc.borrow::<GLKViewHostObject>(this).drawable_depth_format
}
- (())setDrawableDepthFormat:(GLKViewDrawableFormat)format {
    env.objc.borrow_mut::<GLKViewHostObject>(this).drawable_depth_format = format;
}
- (GLKViewDrawableFormat)drawableStencilFormat {
    env.objc.borrow::<GLKViewHostObject>(this).drawable_stencil_format
}
- (())setDrawableStencilFormat:(GLKViewDrawableFormat)format {
    env.objc.borrow_mut::<GLKViewHostObject>(this).drawable_stencil_format = format;
}
- (GLKViewDrawableFormat)drawableMultisample {
    env.objc.borrow::<GLKViewHostObject>(this).drawable_multisample
}
- (())setDrawableMultisample:(GLKViewDrawableFormat)format {
    env.objc.borrow_mut::<GLKViewHostObject>(this).drawable_multisample = format;
}

- (NSInteger)drawableWidth {
    let bounds: CGRect = msg![env; this bounds];
    bounds.size.width as NSInteger
}
- (NSInteger)drawableHeight {
    let bounds: CGRect = msg![env; this bounds];
    bounds.size.height as NSInteger
}

- (bool)enableSetNeedsDisplay {
    env.objc.borrow::<GLKViewHostObject>(this).enable_set_needs_display
}
- (())setEnableSetNeedsDisplay:(bool)value {
    env.objc.borrow_mut::<GLKViewHostObject>(this).enable_set_needs_display = value;
}

- (())bindDrawable {
    // Apple docs: binds the underlying framebuffer object so the app can
    // render into it. touchHLE's EAGL path renders straight into the
    // app's own framebuffer/renderbuffer objects, so there is no separate
    // GLKit-managed FBO to bind here.
    log_dbg!("[(GLKView*){:?} bindDrawable] (no-op)", this);
}

- (())deleteDrawable {
    log_dbg!("[(GLKView*){:?} deleteDrawable] (no-op)", this);
}

- (())display {
    // Apple docs: GLKView "uses the regular view drawing cycle…, calling
    // its drawRect: method whenever the contents of the view need to be
    // updated" and apps either override drawRect: in a subclass or
    // implement the delegate's glkView:drawInRect:.
    let bounds: CGRect = msg![env; this bounds];
    let delegate = env.objc.borrow::<GLKViewHostObject>(this).delegate;
    if delegate != nil {
        () = msg![env; delegate glkView:this drawInRect:bounds];
    } else {
        () = msg![env; this drawRect:bounds];
    }
}

- (())dealloc {
    let context = env.objc.borrow::<GLKViewHostObject>(this).context;
    if context != nil {
        release(env, context);
    }
    // delegate is unowned: nothing to release.
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

@implementation GLKViewController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<GLKViewControllerHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)delegate {
    env.objc.borrow::<GLKViewControllerHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    // Apple documents this property as unowned(unsafe): do not retain.
    env.objc.borrow_mut::<GLKViewControllerHostObject>(this).delegate = delegate;
}

- (NSInteger)preferredFramesPerSecond {
    env.objc.borrow::<GLKViewControllerHostObject>(this).preferred_frames_per_second
}
- (())setPreferredFramesPerSecond:(NSInteger)fps {
    // Apple docs: setting 0 (or a negative value) means "use the default",
    // which is 30 fps.
    let fps = if fps <= 0 { 30 } else { fps };
    env.objc
        .borrow_mut::<GLKViewControllerHostObject>(this)
        .preferred_frames_per_second = fps;
}
- (NSInteger)framesPerSecond {
    // "The actual rate that the view controller attempts to call the view
    // to update its contents" — we don't decimate, so report the
    // preferred rate.
    env.objc.borrow::<GLKViewControllerHostObject>(this).preferred_frames_per_second
}

- (bool)isPaused {
    env.objc.borrow::<GLKViewControllerHostObject>(this).paused
}
- (())setPaused:(bool)paused {
    let host = env.objc.borrow_mut::<GLKViewControllerHostObject>(this);
    let was_paused = host.paused;
    host.paused = paused;
    if was_paused == paused {
        return;
    }
    // Apple docs (GLKViewControllerDelegate): the delegate receives
    // glkViewController:willPause: when the pause state changes.
    let delegate = env.objc.borrow::<GLKViewControllerHostObject>(this).delegate;
    if delegate != nil {
        if let Some(sel) = env.objc.lookup_selector("glkViewController:willPause:") {
            let responds: bool = msg![env; delegate respondsToSelector:sel];
            if responds {
                () = msg![env; delegate glkViewController:this willPause:paused];
            }
        }
    }
}

- (bool)pauseOnWillResignActive {
    env.objc.borrow::<GLKViewControllerHostObject>(this).pause_on_will_resign_active
}
- (())setPauseOnWillResignActive:(bool)value {
    env.objc
        .borrow_mut::<GLKViewControllerHostObject>(this)
        .pause_on_will_resign_active = value;
}
- (bool)resumeOnDidBecomeActive {
    env.objc.borrow::<GLKViewControllerHostObject>(this).resume_on_did_become_active
}
- (())setResumeOnDidBecomeActive:(bool)value {
    env.objc
        .borrow_mut::<GLKViewControllerHostObject>(this)
        .resume_on_did_become_active = value;
}

- (NSInteger)framesDisplayed {
    env.objc.borrow::<GLKViewControllerHostObject>(this).frames_displayed
}
- (NSTimeInterval)timeSinceLastUpdate {
    // We do not drive the GLKit animation loop yet; report a plausible
    // single-frame delta so dt-based game logic keeps advancing.
    let fps = env.objc.borrow::<GLKViewControllerHostObject>(this).preferred_frames_per_second;
    1.0 / (fps.max(1) as NSTimeInterval)
}
- (NSTimeInterval)timeSinceLastDraw {
    let fps = env.objc.borrow::<GLKViewControllerHostObject>(this).preferred_frames_per_second;
    1.0 / (fps.max(1) as NSTimeInterval)
}
- (NSTimeInterval)timeSinceFirstResume {
    let frames = env.objc.borrow::<GLKViewControllerHostObject>(this).frames_displayed;
    let fps = env.objc.borrow::<GLKViewControllerHostObject>(this).preferred_frames_per_second;
    (frames as NSTimeInterval) / (fps.max(1) as NSTimeInterval)
}
- (NSTimeInterval)timeSinceLastResume {
    msg![env; this timeSinceFirstResume]
}

@end

};

pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
    path: "/System/Library/Frameworks/GLKit.framework/GLKit",
    aliases: &[],
    class_exports: &[CLASSES],
    constant_exports: &[CONSTANTS],
    function_exports: &[FUNCTIONS],
};
