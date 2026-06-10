/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! EAGL.

use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::frameworks::core_animation::ca_eagl_layer::{
    find_fullscreen_eagl_layer, get_pixels_vec_for_presenting, present_pixels,
};
use crate::frameworks::core_graphics::{CGRect, CGSize};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::NSUInteger;
use crate::frameworks::uikit;
use crate::gles::gles11_raw as gles11; // constants only
use crate::gles::gles11_raw::types::*;
use crate::gles::present::{present_frame, FpsCounter};
use crate::gles::{create_gles1_ctx, create_gles2_ctx, gles1_on_gl2, GLESContext, GLES};
use crate::mem::MutPtr;
use crate::objc::{id, msg, nil, objc_classes, release, retain, ClassExports, HostObject};
use crate::options::Options;
use crate::Environment;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

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
const kEAGLRenderingAPIOpenGLES2: EAGLRenderingAPI = 2;
#[allow(dead_code)]
const kEAGLRenderingAPIOpenGLES3: EAGLRenderingAPI = 3;

pub(super) struct EAGLContextHostObject {
    pub(super) gles_ctx: Option<Box<dyn GLESContext>>,
    /// Which EAGL rendering API was requested. This influences how
    /// [super::gles_guest] dispatches calls and how the present-renderbuffer
    /// path saves and restores state.
    pub(super) api: EAGLRenderingAPI,
    /// Mapping of OpenGL ES renderbuffer names to `EAGLDrawable` instances
    /// (always `CAEAGLLayer*`). Retains the instance so it won't dangle.
    renderbuffer_drawable_bindings: Rc<RefCell<HashMap<GLuint, id>>>,
    fps_counter: Option<FpsCounter>,
    next_frame_due: Option<Instant>,
    pub mapped_buffers: HashMap<GLuint, (MutPtr<GLvoid>, *mut GLvoid)>,
}
impl HostObject for EAGLContextHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation EAGLContext: NSObject

+ (id)alloc {
    let host_object = Box::new(EAGLContextHostObject {
        gles_ctx: None,
        api: kEAGLRenderingAPIOpenGLES1,
        renderbuffer_drawable_bindings: Rc::new(RefCell::new(HashMap::new())),
        fps_counter: None,
        next_frame_due: None,
        mapped_buffers: HashMap::new(),
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)currentContext {
    env.framework_state.opengles.current_ctx_for_thread(env.current_thread).unwrap_or(nil)
}
+ (bool)setCurrentContext:(id)context { // EAGLContext*
    retain(env, context);

    let current_ctx = env.framework_state.opengles.current_ctx_for_thread(env.current_thread);

    if let Some(old_ctx) = std::mem::take(current_ctx) {
        release(env, old_ctx);
    }

    // reborrow
    let current_ctx = env.framework_state.opengles.current_ctx_for_thread(env.current_thread);

    if context != nil {
        *current_ctx = Some(context);
    }

    true
}

- (id)initWithAPI:(EAGLRenderingAPI)api sharegroup:(id)group {
    if api != kEAGLRenderingAPIOpenGLES1 && api != kEAGLRenderingAPIOpenGLES2 {
        log!(
            "TODO: App requested EAGL initWithAPI:{} sharegroup:{:?}, returning nil as we only support API 1 and 2",
            api,
            group
        );
        return nil;
    }

    if group == nil {
        return msg![env; this initWithAPI:api];
    }

    let window = env.window.as_mut().expect("OpenGL ES is not supported in headless mode");
    let prev_context = env.objc.borrow_mut::<EAGLContextHostObject>(group).gles_ctx.as_mut().unwrap();

    // This is sort of a hack - we set the "current" context, then immediately
    // drop it. Since we know all the code between here and creating the new
    // context, we know that there won't be any context switches, so it's fine
    // to do this.
    {
        let _prev_ctx = prev_context.make_current(window);
    }
    env.window.as_mut().unwrap().set_share_with_current_context(true);

    let mut gles_ins = if api == kEAGLRenderingAPIOpenGLES2 {
        create_gles2_ctx(env)
    } else {
        create_gles1_ctx(env)
    };

    let window = env.window.as_mut().expect("OpenGL ES is not supported in headless mode");
    {
        let gles_ctx = gles_ins.make_current(window);
        log!("Driver info: {}", unsafe { gles_ctx.driver_description() });
    }

    env.objc.borrow_mut::<EAGLContextHostObject>(this).gles_ctx = Some(gles_ins);
    env.objc.borrow_mut::<EAGLContextHostObject>(this).api = api;

    env.window.as_mut().unwrap().set_share_with_current_context(false);

    env.objc.borrow_mut::<EAGLContextHostObject>(this).renderbuffer_drawable_bindings = env.objc.borrow::<EAGLContextHostObject>(group).renderbuffer_drawable_bindings.clone();
    this
}

- (id)initWithAPI:(EAGLRenderingAPI)api {
    if api != kEAGLRenderingAPIOpenGLES1 && api != kEAGLRenderingAPIOpenGLES2 {
        log!(
            "TODO: App requested EAGL initWithAPI:{}, returning nil as we only support API 1 and 2",
            api
        );
        return nil;
    }

    let mut gles_ins = if api == kEAGLRenderingAPIOpenGLES2 {
        create_gles2_ctx(env)
    } else {
        create_gles1_ctx(env)
    };

    let window = env.window.as_mut().expect("OpenGL ES is not supported in headless mode");
    {
        let gles_ctx = gles_ins.make_current(window);
        log!("Driver info: {}", unsafe { gles_ctx.driver_description() });
    }

    env.objc.borrow_mut::<EAGLContextHostObject>(this).gles_ctx = Some(gles_ins);
    env.objc.borrow_mut::<EAGLContextHostObject>(this).api = api;

    this
}

- (EAGLRenderingAPI)API {
    env.objc.borrow::<EAGLContextHostObject>(this).api
}

- (id)sharegroup {
    // We use object itself as the sharegroup.
    // Check initWithAPI:sharegroup: for more info
    this
}

- (())dealloc {
    let host_obj = env.objc.borrow_mut::<EAGLContextHostObject>(this);
    for &(guest_buf, _host_buf) in host_obj.mapped_buffers.values() {
        env.mem.free(guest_buf);
    }
    if Rc::strong_count(&host_obj.renderbuffer_drawable_bindings) == 1 {
        let bindings = std::mem::take(&mut host_obj.renderbuffer_drawable_bindings);
        for (_renderbuffer, drawable) in bindings.take() {
            release(env, drawable);
        }
    }
    env.objc.dealloc_object(this, &mut env.mem);
}

- (bool)renderbufferStorage:(NSUInteger)target
               fromDrawable:(id)drawable { // EAGLDrawable (always CAEAGLayer*)
    log!("[EAGLContext renderbufferStorage:{:#x} fromDrawable:{:?}]", target, drawable);
    assert!(drawable != nil); // TODO: handle unbinding

    assert!(target == gles11::RENDERBUFFER_OES);

    let props: id = msg![env; drawable drawableProperties];

    let format_key = get_static_str(env, kEAGLDrawablePropertyColorFormat);
    let format_rgba8 = get_static_str(env, kEAGLColorFormatRGBA8);
    let format_rgb565 = get_static_str(env, kEAGLColorFormatRGB565);

    let format: id = msg![env; props objectForKey:format_key];
    // Theoretically this should map formats like:
    // - kColorFormatRGBA8 => RGBA8_OES
    // - kColorFormatRGB565 => RGB565_OES
    // However, the specification of EXT_framebuffer_object allows the
    // implementation to arbitrarily restrict which formats can be rendered to,
    // and it seems like RGB565 isn't supported, at least on a machine with
    // Intel HD Graphics 615 running macOS Monterey. I don't think RGBA8 is
    // guaranteed either, but it at least seems to work.
    if !msg![env; format isEqual:format_rgba8] && !msg![env; format isEqual:format_rgb565] {
        log!("[renderbufferStorage:{:?} fromDrawable:{:?}] Warning: unhandled format {:?}, using RGBA8", target, drawable, format);
    }
    let internalformat = gles11::RGBA8_OES;

    let (width, height) = {
        let bounds: CGRect = msg![env; drawable bounds];
        let CGSize { width, height } = bounds.size;
        assert!((0.0..(u32::MAX as f32)).contains(&width));
        assert!((0.0..(u32::MAX as f32)).contains(&height));
        let scale_hack = env.options.scale_hack.get();
        (width.round() as u32 * scale_hack, height.round() as u32 * scale_hack)
    };

    let window = env.window.as_mut().expect("OpenGL ES is not supported in headless mode");

    let renderbuffer = {
        // Unclear from documentation if this method requires an appropriate
        // context to already be active, but that seems to be the case
        // in practice?
        let mut gles = super::sync_context(&mut env.framework_state.opengles, &mut env.objc, window, env.current_thread);
        unsafe {
            gles.RenderbufferStorageOES(target, internalformat, width.try_into().unwrap(), height.try_into().unwrap());
            let mut renderbuffer = 0;
            gles.GetIntegerv(gles11::RENDERBUFFER_BINDING_OES, &mut renderbuffer);
            renderbuffer as _
        }
    };

    retain(env, drawable);
    let host_obj = env.objc.borrow_mut::<EAGLContextHostObject>(this);
    let maybe_old_drawable = host_obj.renderbuffer_drawable_bindings.borrow_mut().insert(
        renderbuffer,
        drawable
    );
    if let Some(old_drawable) = maybe_old_drawable {
        release(env, old_drawable);
    }

    true
}

- (bool)presentRenderbuffer:(NSUInteger)target {
    // Some games (e.g. Angry Birds 1.0) run their main loop without going
    // through the NSRunLoop, so handle_events() in the run loop never fires.
    // Poll and dispatch pending input events here, at the natural per-frame
    // boundary, so touches always reach the game.
    if env.current_thread == 0 {
        env.on_parent_stack_in_coroutine(|window, options| {
            window.poll_for_events(options);
        });
        uikit::handle_events(env);
    }

    // First-frame breadcrumb. presentRenderbuffer is called every frame, so a
    // plain log!() would flood, but the very first call is a key signal that
    // the app actually got past splash/init and is rendering.
    log_once!("[EAGLContext presentRenderbuffer:] first call (app reached first frame)");

    // Frame-count milestones. presentRenderbuffer is called every frame, so we
    // want a small, fixed number of log lines that prove the render loop is
    // still progressing (useful for distinguishing "actually hung" from
    // "running but invisible because the app's stdout doesn't reach this log
    // sink"). The milestones are roughly logarithmic so they cover the range
    // from sub-second to ~10 minutes at 60 FPS without flooding.
    {
        use std::sync::atomic::{AtomicU64, Ordering};
        static FRAME_COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = FRAME_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
        if matches!(n, 10 | 60 | 300 | 1800 | 3600 | 7200 | 18000 | 36000) {
            log!(
                "[EAGLContext presentRenderbuffer:] frame {} reached (render loop is alive)",
                n
            );
        }
    }

    assert!(target == gles11::RENDERBUFFER_OES);

    // The presented frame should be displayed ASAP, but the next one must be
    // delayed, so this needs to be checked before returning.
    let sleep_for = limit_framerate(&mut env.objc.borrow_mut::<EAGLContextHostObject>(this).next_frame_due, &env.options);

    if env.options.print_fps {
        env
            .objc
            .borrow_mut::<EAGLContextHostObject>(this)
            .fps_counter
            .get_or_insert_with(FpsCounter::start)
            .count_frame(format_args!("EAGLContext {this:?}"));
    }

    let fullscreen_layer = find_fullscreen_eagl_layer(env);

    // Unclear from documentation if this method requires the context to be
    // current, but it would be weird if it didn't?
    let window = env.window.as_mut().expect("OpenGL ES is not supported in headless mode");
    let mut gles = super::sync_context(&mut env.framework_state.opengles, &mut env.objc, window, env.current_thread);

    let renderbuffer: GLuint = unsafe {
        let mut renderbuffer = 0;
        gles.GetIntegerv(gles11::RENDERBUFFER_BINDING_OES, &mut renderbuffer);
        renderbuffer as _
    };

    std::mem::drop(gles);

    let Some(&drawable) = env
        .objc
        .borrow::<EAGLContextHostObject>(this)
        .renderbuffer_drawable_bindings
        .borrow()
        .get(&renderbuffer) else {
        log_dbg!("Can't present a renderbuffer {:?} not bound to a drawable!", renderbuffer);
        return false;
    };

    // We're presenting to the opaque CAEAGLLayer that covers the screen.
    // We can use the fast path where we skip composition and present directly.
    if drawable == fullscreen_layer {
        log_dbg!(
            "Layer {:?} is the fullscreen layer, presenting renderbuffer {:?} directly (fast path).",
            drawable,
            renderbuffer,
        );
        // re-borrow
        unsafe {
            present_renderbuffer(env);
        }
    } else {
        if fullscreen_layer != nil {
            // If there's a single layer that covers the screen, and this isn't
            // it, there's no point in presenting the output because it won't be
            // seen. Using a noisy log because it's a weird scenario and might
            // indicate a bug.
            log!(
                "Layer {:?} is not the fullscreen layer {:?}, skipping presentation of renderbuffer {:?}!",
                drawable,
                fullscreen_layer,
                renderbuffer,
            );
            if let Some(sleep_for) = sleep_for {
                env.sleep(sleep_for);
            }
            return true;
        }

        // The very slow and inefficient path: not only does glReadPixels()
        // block the thread until rendering finishes, but the result has to be
        // copied back to system RAM, and then will have to be copied to VRAM
        // again during composition. find_fullscreen_eagl_layer() exists to
        // avoid this.
        log_dbg!(
            "There is no fullscreen layer, presenting renderbuffer {:?} to layer {:?} by copying to RAM (slow path).",
            renderbuffer,
            drawable,
        );
        let pixels_vec = get_pixels_vec_for_presenting(env, drawable);
        // re-borrow
        let (pixels_vec, width, height) = {
            let mut gles = super::sync_context(&mut env.framework_state.opengles, &mut env.objc, env.window.as_mut().unwrap(), env.current_thread);
            unsafe {
                read_renderbuffer(gles.as_mut(), pixels_vec)
            }
        };
        present_pixels(env, drawable, pixels_vec, width, height);
    }

    if let Some(sleep_for) = sleep_for {
        env.sleep(sleep_for);
    }

    true
}

@end

};

/// Implement framerate limiting.
///
/// The real iPhone OS seems to force 60Hz v-sync in `presentRenderbuffer:`.
/// touchHLE does not force v-sync, and its users might not have 60Hz monitors
/// in any case, so to avoid excessive FPS or games running too fast, we need
/// to simulate it.
///
/// V-sync is essentially a limiter with no "slop", or allowance for frames
/// arriving late: if the frame misses a 60Hz interval, it must wait until the
/// next one. This is quite harsh: if frames consistently arrive very slightly
/// late, the framerate is halved!
///
/// Most games already use NSTimer, which is itself a v-sync-like limiter.
/// For the remainder, let's do something a bit kinder, for the benefit of users
/// with slow systems or which are using high scale hack settings: allow at most
/// an interval's worth of accumulated slop. Allowing infinite accumulation of
/// slop is not desirable, because if the game is running slowly for a long time
/// and suddenly speeds back up, it will then run too fast for a long time.
fn limit_framerate(next_frame_due: &mut Option<Instant>, options: &Options) -> Option<Duration> {
    let interval = if let Some(fps) = options.fps_limit {
        1.0 / fps
    } else {
        return None;
    };
    let interval_rust = Duration::from_secs_f64(interval);

    let &mut Some(current_frame_due) = next_frame_due else {
        // First frame presented: no delay yet.
        *next_frame_due = Some(Instant::now() + interval_rust);
        return None;
    };

    let now = Instant::now();
    *next_frame_due = if now > current_frame_due + interval_rust {
        // Too much slop has accumulated. Make the next frame wait for the next
        // interval.
        log_dbg!("Too much slop accumulated, skipping an interval.");
        Some(
            current_frame_due
                + Duration::from_secs_f64(
                    interval * (((now - current_frame_due).as_secs_f64() / interval).ceil()),
                ),
        )
    } else {
        // Time next frame based on when the current frame was due, not
        // the current time, so as to allow some slop.
        Some(current_frame_due + interval_rust)
    };

    if now < current_frame_due {
        // Frame was presented early, delay it to maintain framerate limit.
        Some(current_frame_due.saturating_duration_since(now))
    } else {
        // Frame was presented on time or late, don't delay.
        None
    }
}

// These helper functions make the state backup code easier to read, but
// more importantly, they make it free of mutable variables that wouldn't
// get caught by Rust's unused variable warnings, which are useful to check
// we actually restore the stuff we back up.

unsafe fn get_ptr(gles: &mut dyn GLES, pname: GLenum) -> *const GLvoid {
    let mut ptr = std::ptr::null();
    gles.GetPointerv(pname, &mut ptr);
    ptr
}
// Safety: caller's responsibility to use appropriate N.
unsafe fn get_ints<const N: usize>(gles: &mut dyn GLES, pname: GLenum) -> [GLint; N] {
    let mut res = [0; N];
    gles.GetIntegerv(pname, res.as_mut_ptr());
    res
}
// Safety: caller's responsibility to only use this for scalars.
unsafe fn get_int(gles: &mut dyn GLES, pname: GLenum) -> GLint {
    get_ints::<1>(gles, pname)[0]
}
// Safety: caller's responsibility to use appropriate N.
unsafe fn get_tex_env_ints<const N: usize>(
    gles: &mut dyn GLES,
    target: GLenum,
    pname: GLenum,
) -> [GLint; N] {
    let mut res = [0; N];
    gles.GetTexEnviv(target, pname, res.as_mut_ptr());
    res
}
// Safety: caller's responsibility to only use this for scalars.
unsafe fn get_tex_env_int(gles: &mut dyn GLES, target: GLenum, pname: GLenum) -> GLint {
    get_tex_env_ints::<1>(gles, target, pname)[0]
}
// Safety: caller's responsibility to use appropriate N.
unsafe fn get_floats<const N: usize>(gles: &mut dyn GLES, pname: GLenum) -> [GLfloat; N] {
    let mut res = [0.0; N];
    gles.GetFloatv(pname, res.as_mut_ptr());
    res
}
unsafe fn get_renderbuffer_size(gles: &mut dyn GLES) -> (GLsizei, GLsizei) {
    let mut width: GLint = 0;
    let mut height: GLint = 0;
    gles.GetRenderbufferParameterivOES(
        gles11::RENDERBUFFER_OES,
        gles11::RENDERBUFFER_WIDTH_OES,
        &mut width,
    );
    gles.GetRenderbufferParameterivOES(
        gles11::RENDERBUFFER_OES,
        gles11::RENDERBUFFER_HEIGHT_OES,
        &mut height,
    );
    (width, height)
}

/// Copies the pixels in a renderbuffer bound to `GL_RENDERBUFFER_BINDING_OES`
/// (which should be provided by the app) to a provided [Vec], trying to avoid
/// noticeably modifying OpenGL ES state while doing so.
///
/// This uses `glReadPixels()`, with all the associated performance risks. Any
/// existing content in the [Vec] will bereplaced. The format is RGBA8.
/// The returned values are the [Vec], the width and height.
///
/// The provided context must be current.
unsafe fn read_renderbuffer(gles: &mut dyn GLES, mut pixel_buffer: Vec<u8>) -> (Vec<u8>, u32, u32) {
    let renderbuffer: GLuint = get_int(gles, gles11::RENDERBUFFER_BINDING_OES) as _;
    let (width, height) = get_renderbuffer_size(gles);
    let width_u32: u32 = width.try_into().unwrap();
    let height_u32: u32 = height.try_into().unwrap();

    // To avoid confusing the guest app, we need to be able to undo any
    // state changes we make.
    let old_framebuffer: GLuint = get_int(gles, gles11::FRAMEBUFFER_BINDING_OES) as _;

    // Create a framebuffer we can use to read from the renderbuffer
    let mut src_framebuffer = 0;
    gles.GenFramebuffersOES(1, &mut src_framebuffer);
    gles.BindFramebufferOES(gles11::FRAMEBUFFER_OES, src_framebuffer);
    gles.FramebufferRenderbufferOES(
        gles11::FRAMEBUFFER_OES,
        gles11::COLOR_ATTACHMENT0_OES,
        gles11::RENDERBUFFER_OES,
        renderbuffer,
    );

    // Read the pixels
    let size = (width_u32 as usize)
        .checked_mul(height_u32 as usize)
        .unwrap()
        .checked_mul(4)
        .unwrap();
    pixel_buffer.clear();
    pixel_buffer.reserve_exact(size);
    let before = Instant::now();
    gles.ReadPixels(
        0,
        0,
        width,
        height,
        gles11::RGBA,
        gles11::UNSIGNED_BYTE,
        pixel_buffer.as_mut_ptr() as *mut _,
    );
    log_dbg!(
        "glReadPixels(0, 0, {}, {}, …) took {:?}",
        width,
        height,
        Instant::now().saturating_duration_since(before)
    );
    pixel_buffer.set_len(size);

    // Clean up the framebuffer object since we no longer need it.
    gles.DeleteFramebuffersOES(1, &src_framebuffer);

    // Restore the framebuffer binding
    gles.BindFramebufferOES(gles11::FRAMEBUFFER_OES, old_framebuffer);

    (pixel_buffer, width_u32, height_u32)
}

/// Shader-based variant of the renderbuffer presenter, used when the
/// underlying driver is a real OpenGL ES 2.0 driver (no fixed-function
/// pipeline available).
///
/// This is intentionally simpler than the fixed-function version: we save the
/// minimum amount of ES 2.0 state, draw the textured quad with a small
/// dedicated shader program, and restore. The app's matrices, vertex pointers
/// etc. are not part of ES 2.0 state and thus need no save/restore.
unsafe fn present_renderbuffer_es2(
    gles: &mut dyn GLES,
    viewport: (u32, u32, u32, u32),
    rotation_matrix: crate::matrix::Matrix<2>,
    virtual_cursor_visible_at: Option<(f32, f32, bool)>,
) {
    use crate::gles::gles2_raw as gles2;

    // Save state we are about to clobber
    let mut old_program: GLint = 0;
    gles.GetIntegerv(gles2::CURRENT_PROGRAM, &mut old_program);
    let mut old_array_buffer: GLint = 0;
    gles.GetIntegerv(gles2::ARRAY_BUFFER_BINDING, &mut old_array_buffer);
    let mut old_elem_buffer: GLint = 0;
    gles.GetIntegerv(gles2::ELEMENT_ARRAY_BUFFER_BINDING, &mut old_elem_buffer);
    let mut old_active_texture: GLint = 0;
    gles.GetIntegerv(gles2::ACTIVE_TEXTURE, &mut old_active_texture);
    let mut old_texture: GLint = 0;
    gles.GetIntegerv(gles2::TEXTURE_BINDING_2D, &mut old_texture);
    let mut old_framebuffer: GLint = 0;
    gles.GetIntegerv(gles2::FRAMEBUFFER_BINDING, &mut old_framebuffer);
    let mut old_viewport = [0i32; 4];
    gles.GetIntegerv(gles2::VIEWPORT, old_viewport.as_mut_ptr());
    let mut old_clear_color = [0.0f32; 4];
    gles.GetFloatv(gles2::COLOR_CLEAR_VALUE, old_clear_color.as_mut_ptr());
    let depth_test_was_on = gles.IsEnabled(gles2::DEPTH_TEST) != 0;
    let cull_was_on = gles.IsEnabled(gles2::CULL_FACE) != 0;
    let blend_was_on = gles.IsEnabled(gles2::BLEND) != 0;
    let scissor_was_on = gles.IsEnabled(gles2::SCISSOR_TEST) != 0;

    // Save the enabled state of every vertex attribute slot we might touch.
    // The app may have left attributes 0..N enabled; mutating them here would
    // break its next draw call.
    let mut attrib_was_enabled = [0u8; 16];
    for (i, slot) in attrib_was_enabled.iter_mut().enumerate() {
        let mut v: GLint = 0;
        gles.GetVertexAttribiv(i as GLuint, gles2::VERTEX_ATTRIB_ARRAY_ENABLED, &mut v);
        *slot = v as u8;
    }

    // Resolve renderbuffer → texture via a temporary FBO + glCopyTexImage2D,
    // exactly like the fixed-function path but using the ES 2.0 entry points.
    let mut renderbuffer: GLint = 0;
    gles.GetIntegerv(gles2::RENDERBUFFER_BINDING, &mut renderbuffer);
    let (width, height) = {
        let mut w: GLint = 0;
        let mut h: GLint = 0;
        gles.GetRenderbufferParameteriv(gles2::RENDERBUFFER, gles2::RENDERBUFFER_WIDTH, &mut w);
        gles.GetRenderbufferParameteriv(gles2::RENDERBUFFER, gles2::RENDERBUFFER_HEIGHT, &mut h);
        (w, h)
    };

    let mut src_fb: GLuint = 0;
    gles.GenFramebuffers(1, &mut src_fb);
    gles.BindFramebuffer(gles2::FRAMEBUFFER, src_fb);
    gles.FramebufferRenderbuffer(
        gles2::FRAMEBUFFER,
        gles2::COLOR_ATTACHMENT0,
        gles2::RENDERBUFFER,
        renderbuffer as GLuint,
    );

    let mut tex: GLuint = 0;
    gles.GenTextures(1, &mut tex);
    gles.ActiveTexture(gles2::TEXTURE0);
    gles.BindTexture(gles2::TEXTURE_2D, tex);
    gles.CopyTexImage2D(gles2::TEXTURE_2D, 0, gles2::RGB, 0, 0, width, height, 0);
    gles.TexParameteri(
        gles2::TEXTURE_2D,
        gles2::TEXTURE_MIN_FILTER,
        gles2::LINEAR as _,
    );
    gles.TexParameteri(
        gles2::TEXTURE_2D,
        gles2::TEXTURE_MAG_FILTER,
        gles2::LINEAR as _,
    );
    gles.TexParameteri(
        gles2::TEXTURE_2D,
        gles2::TEXTURE_WRAP_S,
        gles2::CLAMP_TO_EDGE as _,
    );
    gles.TexParameteri(
        gles2::TEXTURE_2D,
        gles2::TEXTURE_WRAP_T,
        gles2::CLAMP_TO_EDGE as _,
    );

    gles.BindFramebuffer(gles2::FRAMEBUFFER, 0);
    gles.DeleteFramebuffers(1, &src_fb);

    // Configure the destination viewport (the window) and clear.
    gles.Viewport(
        viewport.0 as _,
        viewport.1 as _,
        viewport.2 as _,
        viewport.3 as _,
    );
    gles.ClearColor(0.0, 0.0, 0.0, 1.0);
    gles.Disable(gles2::DEPTH_TEST);
    gles.Disable(gles2::CULL_FACE);
    gles.Disable(gles2::BLEND);
    gles.Disable(gles2::SCISSOR_TEST);
    gles.Clear(gles2::COLOR_BUFFER_BIT | gles2::DEPTH_BUFFER_BIT | gles2::STENCIL_BUFFER_BIT);

    // Compile the present shader program once and cache it.
    let program = ensure_present_program(gles);
    gles.UseProgram(program.program);
    gles.Uniform1i(program.u_tex, 0);
    let m = crate::matrix::Matrix::<4>::from(&rotation_matrix);
    let cols = m.columns();
    gles.UniformMatrix4fv(
        program.u_tex_mat,
        1,
        gles2::FALSE,
        cols.as_ptr() as *const _,
    );

    // Pixel-coordinate quad covering the whole viewport.
    #[rustfmt::skip]
    let verts: [f32; 24] = [
        // x, y, u, v
        -1.0, -1.0, 0.0, 0.0,
         1.0, -1.0, 1.0, 0.0,
        -1.0,  1.0, 0.0, 1.0,
         1.0, -1.0, 1.0, 0.0,
         1.0,  1.0, 1.0, 1.0,
        -1.0,  1.0, 0.0, 1.0,
    ];
    gles.BindBuffer(gles2::ARRAY_BUFFER, 0);
    gles.EnableVertexAttribArray(program.a_pos as _);
    gles.EnableVertexAttribArray(program.a_uv as _);
    gles.VertexAttribPointer(
        program.a_pos as _,
        2,
        gles2::FLOAT,
        gles2::FALSE,
        16,
        verts.as_ptr() as *const _,
    );
    gles.VertexAttribPointer(
        program.a_uv as _,
        2,
        gles2::FLOAT,
        gles2::FALSE,
        16,
        (verts.as_ptr() as *const u8).add(8) as *const _,
    );
    gles.DrawArrays(gles2::TRIANGLES, 0, 6);

    // Optional: virtual cursor.
    if let Some((cx, cy, pressed)) = virtual_cursor_visible_at {
        let (vx, vy, vw, vh) = viewport;
        let x = cx - vx as f32;
        let y = cy - vy as f32;
        let radius = 10.0_f32;
        // Build quad in NDC.
        let mut q: [f32; 24] = [
            -1.0, -1.0, 0.0, 0.0, 1.0, -1.0, 1.0, 0.0, -1.0, 1.0, 0.0, 1.0, 1.0, -1.0, 1.0, 0.0,
            1.0, 1.0, 1.0, 1.0, -1.0, 1.0, 0.0, 1.0,
        ];
        for i in (0..q.len()).step_by(4) {
            q[i] = (q[i] * radius + x) / (vw as f32 / 2.0) - 1.0;
            q[i + 1] = 1.0 - (q[i + 1] * radius + y) / (vh as f32 / 2.0);
        }
        // Use a solid black quasi-shadow via a separate program, but to keep
        // things simple just sample our present texture with very low alpha
        // — skip for now if no separate cursor shader.
        let _ = pressed;
    }

    gles.DeleteTextures(1, &tex);

    // Restore vertex attribute enabled state so the app's next draw works.
    for (i, &was) in attrib_was_enabled.iter().enumerate() {
        if was != 0 {
            gles.EnableVertexAttribArray(i as GLuint);
        } else {
            gles.DisableVertexAttribArray(i as GLuint);
        }
    }

    // Restore state we touched
    gles.UseProgram(if old_program > 0 {
        old_program as GLuint
    } else {
        0
    });
    gles.BindBuffer(gles2::ARRAY_BUFFER, old_array_buffer as GLuint);
    gles.BindBuffer(gles2::ELEMENT_ARRAY_BUFFER, old_elem_buffer as GLuint);
    gles.BindFramebuffer(gles2::FRAMEBUFFER, old_framebuffer as GLuint);
    gles.BindTexture(gles2::TEXTURE_2D, old_texture as GLuint);
    gles.ActiveTexture(old_active_texture as GLenum);
    gles.Viewport(
        old_viewport[0],
        old_viewport[1],
        old_viewport[2] as _,
        old_viewport[3] as _,
    );
    gles.ClearColor(
        old_clear_color[0],
        old_clear_color[1],
        old_clear_color[2],
        old_clear_color[3],
    );
    if depth_test_was_on {
        gles.Enable(gles2::DEPTH_TEST);
    }
    if cull_was_on {
        gles.Enable(gles2::CULL_FACE);
    }
    if blend_was_on {
        gles.Enable(gles2::BLEND);
    }
    if scissor_was_on {
        gles.Enable(gles2::SCISSOR_TEST);
    }
}

#[derive(Copy, Clone)]
struct PresentProgram {
    program: GLuint,
    a_pos: GLint,
    a_uv: GLint,
    u_tex: GLint,
    u_tex_mat: GLint,
}

thread_local! {
    static PRESENT_PROGRAM: std::cell::Cell<Option<PresentProgram>> =
        const { std::cell::Cell::new(None) };
}

unsafe fn ensure_present_program(gles: &mut dyn GLES) -> PresentProgram {
    use crate::gles::gles2_raw as gles2;
    if let Some(p) = PRESENT_PROGRAM.with(|c| c.get()) {
        return p;
    }

    let vs_src = b"\
        attribute vec2 aPos;\n\
        attribute vec2 aUV;\n\
        uniform mat4 uTexMat;\n\
        varying vec2 vUV;\n\
        void main() {\n\
            gl_Position = vec4(aPos, 0.0, 1.0);\n\
            vUV = (uTexMat * vec4(aUV, 0.0, 1.0)).xy;\n\
        }\0";
    let fs_src = b"\
        precision mediump float;\n\
        varying vec2 vUV;\n\
        uniform sampler2D uTex;\n\
        void main() {\n\
            gl_FragColor = texture2D(uTex, vUV);\n\
        }\0";

    let vs = gles.CreateShader(gles2::VERTEX_SHADER);
    let vs_ptr = vs_src.as_ptr() as *const _;
    let vs_len = (vs_src.len() - 1) as GLint;
    gles.ShaderSource(vs, 1, &vs_ptr, &vs_len);
    gles.CompileShader(vs);
    let mut ok: GLint = 0;
    gles.GetShaderiv(vs, gles2::COMPILE_STATUS, &mut ok);
    if ok == 0 {
        let mut buf = [0u8; 1024];
        let mut len: GLsizei = 0;
        gles.GetShaderInfoLog(vs, 1024, &mut len, buf.as_mut_ptr() as *mut _);
        let s = std::str::from_utf8(std::slice::from_raw_parts(
            buf.as_ptr() as *const u8,
            len as _,
        ))
        .unwrap_or("?");
        panic!("present_es2 vertex shader compile failed: {s}");
    }

    let fs = gles.CreateShader(gles2::FRAGMENT_SHADER);
    let fs_ptr = fs_src.as_ptr() as *const _;
    let fs_len = (fs_src.len() - 1) as GLint;
    gles.ShaderSource(fs, 1, &fs_ptr, &fs_len);
    gles.CompileShader(fs);
    gles.GetShaderiv(fs, gles2::COMPILE_STATUS, &mut ok);
    if ok == 0 {
        let mut buf = [0u8; 1024];
        let mut len: GLsizei = 0;
        gles.GetShaderInfoLog(fs, 1024, &mut len, buf.as_mut_ptr() as *mut _);
        let s = std::str::from_utf8(std::slice::from_raw_parts(
            buf.as_ptr() as *const u8,
            len as _,
        ))
        .unwrap_or("?");
        panic!("present_es2 fragment shader compile failed: {s}");
    }

    let prog = gles.CreateProgram();
    gles.AttachShader(prog, vs);
    gles.AttachShader(prog, fs);
    // Bind to high attribute slots so we never collide with the app's
    // attribute layout (which typically starts at 0).
    gles.BindAttribLocation(prog, 6, b"aPos\0".as_ptr() as *const _);
    gles.BindAttribLocation(prog, 7, b"aUV\0".as_ptr() as *const _);
    gles.LinkProgram(prog);
    gles.GetProgramiv(prog, gles2::LINK_STATUS, &mut ok);
    if ok == 0 {
        let mut buf = [0u8; 1024];
        let mut len: GLsizei = 0;
        gles.GetProgramInfoLog(prog, 1024, &mut len, buf.as_mut_ptr() as *mut _);
        let s = std::str::from_utf8(std::slice::from_raw_parts(
            buf.as_ptr() as *const u8,
            len as _,
        ))
        .unwrap_or("?");
        panic!("present_es2 program link failed: {s}");
    }

    let a_pos = gles.GetAttribLocation(prog, b"aPos\0".as_ptr() as *const _);
    let a_uv = gles.GetAttribLocation(prog, b"aUV\0".as_ptr() as *const _);
    let u_tex = gles.GetUniformLocation(prog, b"uTex\0".as_ptr() as *const _);
    let u_tex_mat = gles.GetUniformLocation(prog, b"uTexMat\0".as_ptr() as *const _);

    let result = PresentProgram {
        program: prog,
        a_pos,
        a_uv,
        u_tex,
        u_tex_mat,
    };
    PRESENT_PROGRAM.with(|c| c.set(Some(result)));
    result
}

/// Copies the pixels in a renderbuffer bound to `GL_RENDERBUFFER_BINDING_OES`
/// (which should be provided by the app) to a texture and presents it with
/// [present_frame], trying to avoid noticeably modifying OpenGL ES state while
/// doing so. The front and back buffers are then swapped.
unsafe fn present_renderbuffer(env: &mut Environment) {
    // Save these for when we need to draw the frame
    let viewport = env.window.as_mut().unwrap().viewport();
    let rotation_matrix = env.window.as_mut().unwrap().rotation_matrix();
    let virtual_cursor_visible_at = env.window.as_mut().unwrap().virtual_cursor_visible_at();

    let gles_ctx = super::get_thread_context(
        &mut env.framework_state.opengles,
        &mut env.objc,
        env.current_thread,
    );

    let mut gles_boxed = gles_ctx.make_current(env.window.as_mut().unwrap());
    let gles = gles_boxed.as_mut();

    // On a real OpenGL ES 2.0 driver (Android etc.) the fixed-function code
    // path below cannot be used — there is no glMatrixMode / glColor4f /
    // glEnableClientState / glVertexPointer. Use a small dedicated
    // shader-based presenter instead.
    if gles.is_es2() {
        present_renderbuffer_es2(gles, viewport, rotation_matrix, virtual_cursor_visible_at);
        std::mem::drop(gles_boxed);
        env.window.as_ref().unwrap().swap_window();
        return;
    }

    // We can't directly copy the content of the renderbuffer to the default
    // framebuffer (the window), but if we attach it to a framebuffer object, we
    // can use glCopyTexImage2D() to copy it to a texture, which we can then
    // draw to the default framebuffer via a textured quad, which can be
    // rotated, scaled or letterboxed as appropriate.

    let renderbuffer: GLuint = get_int(gles, gles11::RENDERBUFFER_BINDING_OES) as _;
    let (width, height) = get_renderbuffer_size(gles);

    // To avoid confusing the guest app, we need to be able to undo any
    // state changes we make.
    let old_framebuffer: GLuint = get_int(gles, gles11::FRAMEBUFFER_BINDING_OES) as _;
    let old_texture_2d: GLuint = get_int(gles, gles11::TEXTURE_BINDING_2D) as _;

    // Create a framebuffer we can use to read from the renderbuffer
    let mut src_framebuffer = 0;
    gles.GenFramebuffersOES(1, &mut src_framebuffer);
    gles.BindFramebufferOES(gles11::FRAMEBUFFER_OES, src_framebuffer);
    gles.FramebufferRenderbufferOES(
        gles11::FRAMEBUFFER_OES,
        gles11::COLOR_ATTACHMENT0_OES,
        gles11::RENDERBUFFER_OES,
        renderbuffer,
    );

    // Create a texture with a copy of the pixels in the framebuffer
    let mut texture: GLuint = 0;
    gles.GenTextures(1, &mut texture);
    gles.BindTexture(gles11::TEXTURE_2D, texture);
    gles.CopyTexImage2D(
        gles11::TEXTURE_2D,
        0,
        gles11::RGB as _,
        0,
        0,
        width,
        height,
        0,
    );
    // The texture will not have any mip levels so we must ensure the filter
    // does not use them, else rendering will fail.
    gles.TexParameteri(
        gles11::TEXTURE_2D,
        gles11::TEXTURE_MIN_FILTER,
        gles11::LINEAR as _,
    );

    // Clean up the framebuffer object since we no longer need it.
    // This also sets the framebuffer bindings back to zero, so rendering
    // will go to the default framebuffer (the window).
    gles.DeleteFramebuffersOES(1, &src_framebuffer);

    // Reset various things that could affect the quad or virtual cursor we're
    // going to draw. Back up the old state while doing so, so it can be
    // restored later. The app's subsequent drawing will be messed up if we
    // don't restore it.

    // GL_CURRENT_PROGRAM (0x8B8D) is an ES 2.0 / GL 2.0 piece of state. ES 1.x
    // contexts won't have any program bound, in which case this is a harmless
    // no-op. ES 2.0 apps do have one bound and we must clear it so our
    // fixed-function quad-drawing code below works.
    const CURRENT_PROGRAM: GLenum = 0x8B8D;
    let old_program: GLuint = get_int(gles, CURRENT_PROGRAM) as _;
    if old_program != 0 {
        gles.UseProgram(0);
    }

    let old_arrays = {
        let mut old_arrays = [gles11::FALSE; gles1_on_gl2::ARRAYS.len()];
        for (is_enabled, info) in old_arrays.iter_mut().zip(gles1_on_gl2::ARRAYS.iter()) {
            gles.GetBooleanv(info.name, is_enabled);
            gles.DisableClientState(info.name);
        }
        old_arrays
    };
    let old_capabilities = {
        let mut old_capabilities = [gles11::FALSE; gles1_on_gl2::CAPABILITIES.len()];
        for (is_enabled, &name) in old_capabilities
            .iter_mut()
            .zip(gles1_on_gl2::CAPABILITIES.iter())
        {
            gles.GetBooleanv(name, is_enabled);
            gles.Disable(name);
        }
        old_capabilities
    };
    let old_matrix_mode: GLenum = get_int(gles, gles11::MATRIX_MODE) as _;
    for mode in [gles11::MODELVIEW, gles11::PROJECTION, gles11::TEXTURE] {
        gles.MatrixMode(mode);
        gles.PushMatrix();
        gles.LoadIdentity();
    }
    let old_color: [GLfloat; 4] = get_floats(gles, gles11::CURRENT_COLOR);
    gles.Color4f(1.0, 1.0, 1.0, 1.0);

    // Back up other things that will be modified while drawing.
    let old_viewport: (GLint, GLint, GLsizei, GLsizei) = {
        let [x, y, width, height] = get_ints(gles, gles11::VIEWPORT);
        (x, y, width as _, height as _)
    };
    let old_clear_color: [GLfloat; 4] = get_floats(gles, gles11::COLOR_CLEAR_VALUE);
    let old_array_buffer: GLuint = get_int(gles, gles11::ARRAY_BUFFER_BINDING) as _;
    let old_vertex_array_binding: GLuint = get_int(gles, gles11::VERTEX_ARRAY_BUFFER_BINDING) as _;
    let old_vertex_array_size: GLint = get_int(gles, gles11::VERTEX_ARRAY_SIZE);
    let old_vertex_array_type: GLenum = get_int(gles, gles11::VERTEX_ARRAY_TYPE) as _;
    let old_vertex_array_stride: GLsizei = get_int(gles, gles11::VERTEX_ARRAY_STRIDE) as _;
    let old_vertex_array_pointer = get_ptr(gles, gles11::VERTEX_ARRAY_POINTER);
    let old_tex_coord_array_binding: GLuint =
        get_int(gles, gles11::TEXTURE_COORD_ARRAY_BUFFER_BINDING) as _;
    let old_tex_coord_array_size: GLint = get_int(gles, gles11::TEXTURE_COORD_ARRAY_SIZE);
    let old_tex_coord_array_type: GLenum = get_int(gles, gles11::TEXTURE_COORD_ARRAY_TYPE) as _;
    let old_tex_coord_array_stride: GLsizei =
        get_int(gles, gles11::TEXTURE_COORD_ARRAY_STRIDE) as _;
    let old_tex_coord_array_pointer = get_ptr(gles, gles11::TEXTURE_COORD_ARRAY_POINTER);
    let old_blend_sfactor: GLenum = get_int(gles, gles11::BLEND_SRC) as _;
    let old_blend_dfactor: GLenum = get_int(gles, gles11::BLEND_DST) as _;

    let old_tex_env_mode = get_tex_env_int(gles, gles11::TEXTURE_ENV, gles11::TEXTURE_ENV_MODE);
    // if the mode is REPLACE, we don't have to reset the other texture
    // environment values
    let tex_env_mode_arr = [gles11::REPLACE; 1];
    gles.TexEnviv(
        gles11::TEXTURE_ENV,
        gles11::TEXTURE_ENV_MODE,
        tex_env_mode_arr.as_ptr().cast(),
    );

    // Draw the quad
    present_frame(gles, viewport, rotation_matrix, virtual_cursor_visible_at);

    // Clean up the texture
    gles.DeleteTextures(1, &texture);

    // Restore all the state saved before rendering
    for (&is_enabled, info) in old_arrays.iter().zip(gles1_on_gl2::ARRAYS.iter()) {
        match is_enabled {
            gles11::TRUE => gles.EnableClientState(info.name),
            gles11::FALSE => gles.DisableClientState(info.name),
            _ => unreachable!(),
        }
    }
    for (&is_enabled, &name) in old_capabilities
        .iter()
        .zip(gles1_on_gl2::CAPABILITIES.iter())
    {
        match is_enabled {
            gles11::TRUE => gles.Enable(name),
            gles11::FALSE => gles.Disable(name),
            _ => unreachable!(),
        }
    }
    for mode in [gles11::MODELVIEW, gles11::PROJECTION, gles11::TEXTURE] {
        gles.MatrixMode(mode);
        gles.PopMatrix();
    }
    gles.MatrixMode(old_matrix_mode);
    gles.Color4f(old_color[0], old_color[1], old_color[2], old_color[3]);
    gles.Viewport(
        old_viewport.0,
        old_viewport.1,
        old_viewport.2,
        old_viewport.3,
    );
    gles.ClearColor(
        old_clear_color[0],
        old_clear_color[1],
        old_clear_color[2],
        old_clear_color[3],
    );
    // GL_ARRAY_BUFFER is implicitly used by the Pointer functions but is also
    // an independent binding.
    gles.BindBuffer(gles11::ARRAY_BUFFER, old_vertex_array_binding);
    gles.VertexPointer(
        old_vertex_array_size,
        old_vertex_array_type,
        old_vertex_array_stride,
        old_vertex_array_pointer,
    );
    gles.BindBuffer(gles11::ARRAY_BUFFER, old_tex_coord_array_binding);
    gles.TexCoordPointer(
        old_tex_coord_array_size,
        old_tex_coord_array_type,
        old_tex_coord_array_stride,
        old_tex_coord_array_pointer,
    );
    gles.BindBuffer(gles11::ARRAY_BUFFER, old_array_buffer);
    gles.BlendFunc(old_blend_sfactor, old_blend_dfactor);

    let old_tex_env_mode_arr = [old_tex_env_mode; 1];
    gles.TexEnviv(
        gles11::TEXTURE_ENV,
        gles11::TEXTURE_ENV_MODE,
        old_tex_env_mode_arr.as_ptr().cast(),
    );

    std::mem::drop(gles_boxed);

    // SDL2's documentation warns 0 should be bound to the draw framebuffer
    // when swapping the window, so this is the perfect moment.
    env.window.as_ref().unwrap().swap_window();

    let mut gles_boxed = gles_ctx.make_current(env.window.as_mut().unwrap());
    let gles = gles_boxed.as_mut();

    // Restore the other bindings
    gles.BindTexture(gles11::TEXTURE_2D, old_texture_2d);
    gles.BindFramebufferOES(gles11::FRAMEBUFFER_OES, old_framebuffer);

    // Restore the previously bound shader program (ES 2.0).
    if old_program != 0 {
        gles.UseProgram(old_program);
    }

    // { let err = gles.GetError(); if err != 0 { panic!("{:#x}", err); } }
}

pub fn EAGLGetVersion(env: &mut Environment, major: MutPtr<u32>, minor: MutPtr<u32>) {
    let version_major: u32 = 1;
    let version_minor: u32 = 1;

    if !major.is_null() {
        env.mem.write(major, version_major);
    }
    if !minor.is_null() {
        env.mem.write(minor, version_minor);
    }

    log!(
        "EAGLGetVersion called: major={}, minor={}",
        version_major,
        version_minor
    );
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(EAGLGetVersion(_, _))];
