//! EAGL.

use super::{GLES1OnGL2, GLES};
use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::NSUInteger;
use crate::objc::{id, msg, nil, objc_classes, release, retain, ClassExports, HostObject};
use crate::window::gles11; // for constants

// These are used by the EAGLDrawable protocol implemented by CAEAGLayer.
// Since these have the ABI of constant symbols rather than literal constants,
// the values shouldn't matter, and haven't been checked against real iPhone OS.
pub const kEAGLDrawablePropertyColorFormat: &str = "ColorFormat";
pub const kEAGLDrawablePropertyRetainedBacking: &str = "RetainedBacking";
pub const kEAGLColorFormatRGBA8: &str = "RGBA8";
pub const kEAGLColorFormatRGB565: &str = "RGB565";

pub const CONSTANTS: ConstantExports = &[
    (
        "_kEAGLDrawablePropertyColorFormat",
        HostConstant::NSString(kEAGLDrawablePropertyColorFormat),
    ),
    (
        "_kEAGLDrawablePropertyRetainedBacking",
        HostConstant::NSString(kEAGLDrawablePropertyRetainedBacking),
    ),
    (
        "_kEAGLColorFormatRGBA8",
        HostConstant::NSString(kEAGLColorFormatRGBA8),
    ),
    (
        "_kEAGLColorFormatRGB565",
        HostConstant::NSString(kEAGLColorFormatRGB565),
    ),
];

type EAGLRenderingAPI = u32;
const kEAGLRenderingAPIOpenGLES1: EAGLRenderingAPI = 1;
#[allow(dead_code)]
const kEAGLRenderingAPIOpenGLES2: EAGLRenderingAPI = 2;
#[allow(dead_code)]
const kEAGLRenderingAPIOpenGLES3: EAGLRenderingAPI = 3;

pub struct EAGLContextHostObject {
    gles_ctx: Option<Box<dyn GLES>>,
}
impl HostObject for EAGLContextHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation EAGLContext: NSObject

+ (id)alloc {
    let host_object = Box::new(EAGLContextHostObject { gles_ctx: None });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (bool)setCurrentContext:(id)context { // EAGLContext*
    assert!(context != nil); // TODO: nil handling
    assert!(env.current_thread == 0); // TODO: per-thread contexts

    retain(env, context);

    // Clear flag value, we're changing context anyway.
    let _ = env.window.is_app_gl_ctx_no_longer_current();

    let old_ctx = std::mem::take(&mut env.framework_state.opengles.current_ctx);
    if let Some((old_eagl, old_gles)) = old_ctx {
        let host_obj = env.objc.borrow_mut::<EAGLContextHostObject>(old_eagl);
        assert!(host_obj.gles_ctx.is_none());
        host_obj.gles_ctx = Some(old_gles);
        release(env, old_eagl);
    }

    let host_obj = env.objc.borrow_mut::<EAGLContextHostObject>(context);
    let gles_ctx = std::mem::take(&mut host_obj.gles_ctx).unwrap();
    gles_ctx.make_current(&mut env.window);
    env.framework_state.opengles.current_ctx = Some((context, gles_ctx));

    true
}

- (id)initWithAPI:(EAGLRenderingAPI)api {
    assert!(api == kEAGLRenderingAPIOpenGLES1);

    let gles1_ctx = Box::new(GLES1OnGL2::new(&mut env.window));

    *env.objc.borrow_mut(this) = EAGLContextHostObject {
        gles_ctx: Some(gles1_ctx),
    };

    this
}

- (bool)renderbufferStorage:(NSUInteger)target
               fromDrawable:(id)drawable { // EAGLDrawable (always CAEAGLayer*)
    assert!(target == gles11::RENDERBUFFER_OES);

    let props: id = msg![env; drawable drawableProperties];

    let format_key = get_static_str(env, kEAGLDrawablePropertyColorFormat);
    let format_rgba8 = get_static_str(env, kEAGLColorFormatRGBA8);
    let format_rgb565 = get_static_str(env, kEAGLColorFormatRGB565);

    let format: id = msg![env; props objectForKey:format_key];
    let internalformat = if msg![env; format isEqualTo:format_rgba8] {
        gles11::RGBA8_OES
    } else if msg![env; format isEqualTo:format_rgb565] {
        gles11::RGB565_OES
    } else { // default/fallback
        gles11::RGBA8_OES
    };

    // FIXME: get width and height from the layer!
    let (width, height) = (320, 480);

    // Unclear from documentation if this method requires an appropriate context
    // to already be active, but that seems to be the case in practice?
    let (_eagl, ref mut gles) = env.framework_state.opengles.current_ctx.as_mut().unwrap();
    if env.window.is_app_gl_ctx_no_longer_current() {
        log_dbg!("Restoring guest app OpenGL context.");
        gles.make_current(&mut env.window);
    }
    unsafe {
        gles.RenderbufferStorageOES(target, internalformat, width, height)
    }

    true
}

@end

};
