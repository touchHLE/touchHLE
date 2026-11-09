/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! EAGL.

use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::core_animation::ca_eagl_layer::{
    find_fullscreen_eagl_layer, get_pixels_vec_for_presenting, present_pixels,
};
use crate::frameworks::core_graphics::{CGRect, CGSize};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::NSUInteger;
use crate::gles::gles11_raw as gles11; // constants only
use crate::gles::gles11_raw::types::*;
use crate::gles::present::{present_frame, FpsCounter};
use crate::gles::{create_gles1_ctx, create_gles2_ctx, GLESContext, GLES};
use crate::mem::MutPtr;
use crate::objc::{id, msg, nil, objc_classes, release, retain, ClassExports, HostObject};
use crate::options::Options;
use crate::Environment;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

static PEAR_DUMP_FRAME_PREFIX: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("PEAR_DUMP_FRAME").ok().filter(|s| !s.is_empty()));

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

    if std::env::var("PEAR_DEBUG_GL").is_ok_and(|v| !v.is_empty() && v != "0") {
        let api = if context == nil {
            0
        } else {
            env.objc.borrow::<EAGLContextHostObject>(context).api
        };
        log!("PEAR_DEBUG_GL: setCurrentContext -> api={} ctx={:?}", api, context);
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

    // All GL contexts live in one EGL share group (see crate::window), so
    // sharing works automatically; the group object is only used to share the
    // renderbuffer-to-drawable bindings map.
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
        let scale = crate::window::FRAMEBUFFER_SCALE;
        (width.round() as u32 * scale, height.round() as u32 * scale)
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
    assert!(target == gles11::RENDERBUFFER_OES);

    // The presented frame should be displayed ASAP, but the next one must be
    // delayed, so this needs to be checked before returning.
    let sleep_for = limit_framerate(&mut env.objc.borrow_mut::<EAGLContextHostObject>(this).next_frame_due, &env.options);

    if env.options.print_fps || crate::gles::present::hud_enabled() {
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

    if let Some(prefix) = PEAR_DUMP_FRAME_PREFIX.as_deref() {
        static FRAME_COUNTER: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);
        let n = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if n < 4000 && n % 20 == 0 {
            let (pixels, width, height) = unsafe { read_renderbuffer(gles.as_mut(), Vec::new()) };
            let mut out = format!("{width} {height}\n").into_bytes();
            out.extend_from_slice(&pixels);
            std::fs::write(format!("{prefix}{n:05}.raw"), out).unwrap();
        }
    }

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
    let interval = {
        let fps = options.fps_limit?;
        1.0 / fps
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

// These helper functions make the remaining state queries easier to read and
// free of mutable variables that wouldn't get caught by Rust's unused variable
// warnings.

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

/// GL objects used to copy the guest's renderbuffer into the internal context
/// before drawing it to the window.
///
/// Creating a framebuffer and a texture for every presented frame made ANGLE
/// allocate and destroy a Metal texture on each frame, which costs a
/// reallocation and forces a GPU sync point. In practice that showed up as
/// periodic hitches, sometimes with a black frame while the drawable was being
/// recycled. These objects are now created once and reused for as long as the
/// source renderbuffer keeps the same identity and size; only the pixel copy
/// happens per frame.
struct PresentTarget {
    src_framebuffer: GLuint,
    texture: GLuint,
    renderbuffer: GLuint,
    width: i32,
    height: i32,
}

thread_local! {
    /// Only ever touched by the internal context, which is unique per window.
    static PRESENT_TARGET: RefCell<Option<PresentTarget>> = const { RefCell::new(None) };
}

thread_local! {
    /// Present-time accumulator used to report frame pacing. Only touched from
    /// the window's presenting thread.
    static PRESENT_STATS: RefCell<PresentStats> = const { RefCell::new(PresentStats::new()) };
}

struct PresentStats {
    frames: u32,
    total_secs: f64,
    max_secs: f64,
    slow_frames: u32,
}

impl PresentStats {
    const fn new() -> Self {
        Self {
            frames: 0,
            total_secs: 0.0,
            max_secs: 0.0,
            slow_frames: 0,
        }
    }
}

/// Accumulates present timings and logs a summary every 300 presents (about
/// 10 s at 30 Hz) when `PEAR_PRESENT_STATS` is set. A slice is "slow" when it
/// exceeds a 33 ms (30 Hz) budget.
fn record_present_time(secs: f64) {
    static ENABLED: std::sync::LazyLock<bool> =
        std::sync::LazyLock::new(|| std::env::var_os("PEAR_PRESENT_STATS").is_some());
    if !*ENABLED {
        return;
    }

    PRESENT_STATS.with_borrow_mut(|stats| {
        stats.frames += 1;
        stats.total_secs += secs;
        stats.max_secs = stats.max_secs.max(secs);
        if secs > 0.033 {
            stats.slow_frames += 1;
        }
        if stats.frames >= 300 {
            log!(
                "Present stats: {} frames, avg {:.2}ms, max {:.2}ms, {} over 33ms",
                stats.frames,
                stats.total_secs * 1000.0 / stats.frames as f64,
                stats.max_secs * 1000.0,
                stats.slow_frames
            );
            *stats = PresentStats::new();
        }
    });
}

/// Reads back a centre crop of the just-drawn back buffer and reports how much
/// of it is non-black, so that presentation can be verified without a human
/// looking at the screen. Enabled by `PEAR_CONTENT_SELFTEST`.
fn content_self_test_enabled() -> bool {
    static ENABLED: std::sync::LazyLock<bool> =
        std::sync::LazyLock::new(|| std::env::var_os("PEAR_CONTENT_SELFTEST").is_some());
    *ENABLED
}

/// Reports what the guest's own renderbuffer contains, separately from what was
/// presented. Enabled by `PEAR_PRESENT_TRACE`: if the source has content but the
/// presented frame does not, the fault is in the copy; if neither has content,
/// the guest really is drawing black.
fn present_trace_enabled() -> bool {
    static ENABLED: std::sync::LazyLock<bool> =
        std::sync::LazyLock::new(|| std::env::var_os("PEAR_PRESENT_TRACE").is_some());
    *ENABLED
}

/// # Safety
/// Must be called with the source framebuffer still bound, on the internal
/// context, before the copy destination is unbound.
unsafe fn trace_source_content(gles: &mut dyn GLES, width: i32, height: i32, renderbuffer: GLuint) {
    static COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let count = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if !count.is_multiple_of(150) {
        return;
    }
    let status = gles.CheckFramebufferStatusOES(gles11::FRAMEBUFFER_OES);
    let read_w = width.clamp(1, 64);
    let read_h = height.clamp(1, 64);
    let mut buffer = vec![0u8; (read_w * read_h * 4) as usize];
    gles.ReadPixels(
        (width - read_w) / 2,
        (height - read_h) / 2,
        read_w,
        read_h,
        gles11::RGBA,
        gles11::UNSIGNED_BYTE,
        buffer.as_mut_ptr().cast(),
    );
    let non_black = buffer
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 8 || pixel[1] > 8 || pixel[2] > 8)
        .count();
    log!(
        "Source self-test: {}/{} non-black in {}x{} of renderbuffer {:?} ({}x{}), framebuffer status {:#x}",
        non_black,
        read_w as u64 * read_h as u64,
        read_w,
        read_h,
        renderbuffer,
        width,
        height,
        status
    );
}

static CONTENT_SELF_TEST_COUNT: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(0);

/// # Safety
/// Must be called with the window's internal GL context current and before the
/// window's buffers are swapped.
unsafe fn self_test_content(gles: &mut dyn GLES, width: i32, height: i32) {
    use std::sync::atomic::Ordering;
    let count = CONTENT_SELF_TEST_COUNT.fetch_add(1, Ordering::Relaxed);
    // A report every 150 presents is dense enough for diagnosis and cheap.
    if !count.is_multiple_of(150) {
        return;
    }

    let read_w = width.clamp(1, 64);
    let read_h = height.clamp(1, 64);
    let mut buffer = vec![0u8; (read_w * read_h * 4) as usize];
    gles.ReadPixels(
        (width - read_w) / 2,
        (height - read_h) / 2,
        read_w,
        read_h,
        gles11::RGBA,
        gles11::UNSIGNED_BYTE,
        buffer.as_mut_ptr().cast(),
    );

    let non_black = buffer
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 8 || pixel[1] > 8 || pixel[2] > 8)
        .count();
    log!(
        "Content self-test: {}/{} non-black pixels in a {}x{} centre crop of {}x{}",
        non_black,
        read_w as u64 * read_h as u64,
        read_w,
        read_h,
        width,
        height
    );
}

/// Copies the pixels in the guest's color renderbuffer to a texture, then
/// presents it with [present_frame] and swaps the window's buffers.
///
/// The guest's GL state is never touched: the copy and the presentation happen
/// entirely on the window's internal ES 1.1 context (see
/// [crate::window::Window::make_internal_gl_ctx_current]), which accesses the
/// guest's renderbuffer through the shared EGL share group. That context is
/// dedicated to our own drawing, so its state doesn't need to be backed up.
unsafe fn present_renderbuffer(env: &mut Environment) {
    let present_start = Instant::now();
    // Save these for when we need to draw the frame
    let viewport = env.window.as_mut().unwrap().viewport();
    let rotation_matrix = env.window.as_mut().unwrap().rotation_matrix();
    let virtual_cursor_visible_at = env.window.as_mut().unwrap().virtual_cursor_visible_at();

    // On the guest's current context: find the current renderbuffer and its
    // size. Renderbuffer objects are shared between all contexts, so the
    // internal context can attach this one to its own framebuffer.
    let (renderbuffer, width, height) = {
        let gles_ctx = super::get_thread_context(
            &mut env.framework_state.opengles,
            &mut env.objc,
            env.current_thread,
        );
        let mut gles_boxed = gles_ctx.make_current(env.window.as_mut().unwrap());
        let gles = gles_boxed.as_mut();
        let renderbuffer: GLuint = get_int(gles, gles11::RENDERBUFFER_BINDING_OES) as _;
        let (width, height) = get_renderbuffer_size(gles);
        (renderbuffer, width, height)
    };

    // On the internal context: copy the renderbuffer's pixels to a texture and
    // draw it to the window.
    {
        let mut gles_boxed = env.window.as_mut().unwrap().make_internal_gl_ctx_current();
        let gles = gles_boxed.as_mut();

        // We can't directly copy the content of the renderbuffer to the default
        // framebuffer (the window), but if we attach it to a framebuffer
        // object, we can use glCopyTexImage2D() to copy it to a texture, which
        // we can then draw to the default framebuffer via a textured quad,
        // which can be rotated, scaled or letterboxed as appropriate.
        // The renderbuffer is the read source for the copy, so it has to stay
        // attached to a framebuffer while the copy happens. The framebuffer
        // and texture are cached and reused: only the copy is per-frame work.
        PRESENT_TARGET.with_borrow_mut(|slot| {
            let changed = match slot.as_ref() {
                Some(target) => {
                    target.renderbuffer != renderbuffer
                        || target.width != width
                        || target.height != height
                }
                None => true,
            };
            if changed {
                if let Some(old) = slot.take() {
                    let mut framebuffers = [old.src_framebuffer];
                    gles.DeleteFramebuffersOES(1, framebuffers.as_mut_ptr());
                    let mut textures = [old.texture];
                    gles.DeleteTextures(1, textures.as_mut_ptr());
                }

                let mut src_framebuffer: GLuint = 0;
                gles.GenFramebuffersOES(1, &mut src_framebuffer);
                // The attachment applies to the currently bound framebuffer,
                // so bind before attaching.
                gles.BindFramebufferOES(gles11::FRAMEBUFFER_OES, src_framebuffer);
                gles.FramebufferRenderbufferOES(
                    gles11::FRAMEBUFFER_OES,
                    gles11::COLOR_ATTACHMENT0_OES,
                    gles11::RENDERBUFFER_OES,
                    renderbuffer,
                );

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
                // The texture will not have any mip levels so we must ensure the
                // filter does not use them, else rendering will fail.
                gles.TexParameteri(
                    gles11::TEXTURE_2D,
                    gles11::TEXTURE_MIN_FILTER,
                    gles11::LINEAR as _,
                );

                *slot = Some(PresentTarget {
                    src_framebuffer,
                    texture,
                    renderbuffer,
                    width,
                    height,
                });
                log_dbg!(
                    "present target allocated for renderbuffer {:?} ({}x{})",
                    renderbuffer,
                    width,
                    height
                );
            } else {
                let target = slot.as_ref().unwrap();
                gles.BindTexture(gles11::TEXTURE_2D, target.texture);
                gles.BindFramebufferOES(gles11::FRAMEBUFFER_OES, target.src_framebuffer);
                gles.CopyTexSubImage2D(gles11::TEXTURE_2D, 0, 0, 0, 0, 0, width, height);
            }
            if present_trace_enabled() {
                trace_source_content(gles, width, height, renderbuffer);
            }
            // Rendering must go to the default framebuffer (the window) once
            // the copy is done. The texture stays bound because
            // [present_frame] draws it.
            gles.BindFramebufferOES(gles11::FRAMEBUFFER_OES, 0);
        });

        // Reset the state that could affect the quad or virtual cursor we're
        // going to draw. Unlike the old single-context present path, there is
        // no need to back any of this up: this context is used exclusively by
        // touchHLE's own drawing (see also composition.rs, which sets up the
        // same state).
        gles.Color4f(1.0, 1.0, 1.0, 1.0);
        gles.Disable(gles11::BLEND);
        for mode in [gles11::MODELVIEW, gles11::PROJECTION, gles11::TEXTURE] {
            gles.MatrixMode(mode);
            gles.LoadIdentity();
        }
        // GL_ARRAY_BUFFER is implicitly used by the Pointer functions but is
        // also an independent binding.
        gles.BindBuffer(gles11::ARRAY_BUFFER, 0);
        gles.BindBuffer(gles11::ELEMENT_ARRAY_BUFFER, 0);
        let tex_env_mode_arr = [gles11::REPLACE; 1];
        gles.TexEnviv(
            gles11::TEXTURE_ENV,
            gles11::TEXTURE_ENV_MODE,
            tex_env_mode_arr.as_ptr().cast(),
        );

        // Draw the quad
        present_frame(gles, viewport, rotation_matrix, virtual_cursor_visible_at);

        if content_self_test_enabled() {
            self_test_content(gles, viewport.2 as i32, viewport.3 as i32);
        }
    }

    let present_secs = present_start.elapsed().as_secs_f64();
    record_present_time(present_secs);
    if present_secs > 0.033 {
        log_dbg!(
            "slow present: {:.1}ms for renderbuffer {:?} ({}x{}) viewport {:?}",
            present_secs * 1000.0,
            renderbuffer,
            width,
            height,
            viewport
        );
    }

    // SDL2's documentation warns 0 should be bound to the draw framebuffer
    // when swapping the window, so this is the perfect moment. The guest's
    // context is re-bound automatically on its next GL call (see
    // super::sync_context).
    env.window.as_ref().unwrap().swap_window();
}

