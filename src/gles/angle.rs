/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Passthrough for OpenGL ES 1.1 and 2.0 contexts on ANGLE (Metal backend).
//!
//! There's almost no validation of arguments here, because we assume the
//! driver is complete and the app uses it correctly. The exception is where we
//! expect an extension could be used that the driver might not support (e.g.
//! vendor-specific texture compression). In such cases, we should reject
//! vendor-specific things unless we've made sure we can emulate them on all
//! host platforms for touchHLE.
//!
//! All contexts live in one EGL share group (see [crate::window]), so GL
//! objects such as textures and renderbuffers can cross between ES 1.1 and
//! 2.0 contexts. Framebuffer objects are an exception (ANGLE keeps them
//! per-context), so FBO setup calls are replayed on every context.

use super::gles11_raw as gles11;
use super::gles11_raw::types::*;
use super::gles2_raw as gles2;
use super::gles_generic::{GLchar, GLES};
use super::util::{try_decode_pvrtc, PalettedTextureFormat};
use super::GLESContext;
use crate::window::{GLContext, GLVersion, Window};
use std::ffi::CStr;

/// Diagnostic: when `PEAR_DEBUG_GL` is set, logs any GL error raised while a
/// traced trait method runs, naming the method. Used to locate calls the ANGLE
/// backend rejects. Returns `None` (zero overhead) when disabled.
pub fn gl_trace(name: &'static str) -> Option<GlTraceGuard> {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let enabled = *ENABLED
        .get_or_init(|| std::env::var("PEAR_DEBUG_GL").is_ok_and(|v| !v.is_empty() && v != "0"));
    enabled.then_some(GlTraceGuard(name))
}
pub struct GlTraceGuard(&'static str);
impl Drop for GlTraceGuard {
    fn drop(&mut self) {
        unsafe {
            let mut err = gles2::GetError();
            while err != 0 {
                log!(
                    "PEAR_DEBUG_GL: {} raised {:#x} (on EGL ctx {:?})",
                    self.0,
                    err,
                    crate::egl::global().current_context()
                );
                err = gles2::GetError();
            }
        }
    }
}

// Implement an `OES_framebuffer_object` entry point so that it routes to the
// correct spelling for the bound context: the ES 1.1 companion accepts the
// `*OES` names, the ES 2.0 main context the plain ones. (The guest dispatch
// layer always calls the `*OES` trait methods.)
// Query-style FBO entry points (Gen/Is/Check/Get*/GenerateMipmap): routed
/// to the context currently in use. Framebuffer *objects* are per-context in
/// ANGLE (unlike textures, buffers and renderbuffers, which are shared), but
// names are materialized lazily at bind, so Gen calls need no replay.
macro_rules! fbo_query_entry_point {
    ($oes:ident, $plain:ident, ($($arg:ident : $ty:ty),*) $(-> $ret:ty)?) => {
        #[allow(unused_variables)]
        unsafe fn $oes(&mut self, $($arg: $ty),*) $(-> $ret)? {
            if self.ctx.client_version >= 2 && !self.ctx.using_es1 {
                gles2::$plain($($arg),*)
            } else {
                gles11::$oes($($arg),*)
            }
        }
    };
}

// State-mutating FBO entry points (Bind/Delete/attach/storage): replayed on
/// BOTH the main context and the ES 1.1 companion so both keep identical
// framebuffer objects. Without this, the companion's same-named FBO is an
// empty ghost and every draw into it fails with INVALID_FRAMEBUFFER_OPERATION.
macro_rules! fbo_entry_point {
    ($oes:ident, $plain:ident, ($($arg:ident : $ty:ty),*)) => {
        #[allow(unused_variables)]
        unsafe fn $oes(&mut self, $($arg: $ty),*) {
            if let Some(fb) = self.ctx.es1_fallback.as_ref() {
                let on_companion = self.ctx.using_es1;
                if on_companion {
                    gles11::$oes($($arg),*);
                    self.ctx.gl_ctx.make_current_raw();
                    gles2::$plain($($arg),*);
                    fb.make_current_raw();
                } else {
                    gles2::$plain($($arg),*);
                    fb.make_current_raw();
                    gles11::$oes($($arg),*);
                    self.ctx.gl_ctx.make_current_raw();
                }
            } else if self.ctx.client_version >= 2 {
                gles2::$plain($($arg),*)
            } else {
                gles11::$oes($($arg),*)
            }
        }
    };
}

pub struct ANGLEContext {
    gl_ctx: GLContext,
    /// Companion ES 1.1 context, in the same share group, created for guest
    /// ES 2.0 contexts. Old iOS drivers let games mix ES 1.1 fixed-function
    /// calls into an ES 2.0 context (Asphalt 6 renders this way); ANGLE is
    /// strict, so such calls are routed to this companion instead (see the
    /// style switching in [AngleGles]).
    es1_fallback: Option<GLContext>,
    /// Whether ES 1.1-style calls are currently routed to `es1_fallback`.
    using_es1: bool,
    /// The guest has used the programmable (shader) pipeline.
    saw_es2_shader: bool,
    /// Last program passed to `UseProgram` (0 = none); used to route draws.
    current_program: GLuint,
    /// Mask of enabled client vertex arrays (1 = vertex, 2 = color,
    /// 4 = texcoord, 8 = normal); used to route draws.
    client_arrays: u8,
    client_version: u32,
    is_loaded: bool,
}
impl ANGLEContext {
    pub fn new_with_version(window: &mut Window, version: GLVersion) -> Result<Self, String> {
        let gl_ctx = window.create_gl_context(version)?;
        let es1_fallback = match version {
            GLVersion::GLES20 => Some(window.create_gl_context(GLVersion::GLES11)?),
            GLVersion::GLES11 => None,
        };
        if debug_gl_enabled() {
            if let Some(fb) = es1_fallback.as_ref() {
                unsafe {
                    window.make_gl_context_current(fb);
                    gles11::load_with(|s| window.gl_get_proc_address(s));
                    gles2::load_with(|s| window.gl_get_proc_address(s));
                    let ver =
                        std::ffi::CStr::from_ptr(gles11::GetString(gles11::VERSION) as *const _);
                    let exts =
                        std::ffi::CStr::from_ptr(gles11::GetString(gles11::EXTENSIONS) as *const _);
                    let exts = exts.to_string_lossy();
                    log!(
                        "PEAR_DEBUG_GL: ES1 companion: \"{}\" (EGL ctx {:?}), \
                         OES_framebuffer_object={}, OES_vertex_buffer_object={}, \
                         OES_blend_subtract={}, OES_point_sprite={}, OES_blend_func_separate={}",
                        ver.to_string_lossy(),
                        crate::egl::global().current_context(),
                        exts.contains("OES_framebuffer_object"),
                        exts.contains("OES_vertex_buffer_object"),
                        exts.contains("OES_blend_subtract"),
                        exts.contains("OES_point_sprite"),
                        exts.contains("OES_blend_func_separate"),
                    );
                }
            }
            unsafe {
                window.make_gl_context_current(&gl_ctx);
                gles11::load_with(|s| window.gl_get_proc_address(s));
                gles2::load_with(|s| window.gl_get_proc_address(s));
                let ver = std::ffi::CStr::from_ptr(gles11::GetString(gles11::VERSION) as *const _);
                let exts =
                    std::ffi::CStr::from_ptr(gles11::GetString(gles11::EXTENSIONS) as *const _);
                let exts = exts.to_string_lossy();
                log!(
                    "PEAR_DEBUG_GL: main ctx: \"{}\" (EGL ctx {:?}), anisotropic={}",
                    ver.to_string_lossy(),
                    crate::egl::global().current_context(),
                    exts.contains("texture_filter_anisotropic"),
                );
            }
        }
        Ok(Self {
            gl_ctx,
            es1_fallback,
            using_es1: false,
            saw_es2_shader: false,
            current_program: 0,
            client_arrays: 0,
            client_version: version.client_version(),
            is_loaded: false,
        })
    }

    /// The context ES 1.1-style calls are currently routed to.
    fn gl_ctx_active(&self) -> &GLContext {
        if self.using_es1 {
            self.es1_fallback.as_ref().unwrap()
        } else {
            &self.gl_ctx
        }
    }
}
impl GLESContext for ANGLEContext {
    fn description() -> &'static str {
        "ANGLE (Metal)"
    }

    fn make_current<'gl_ctx, 'win: 'gl_ctx>(
        &'gl_ctx mut self,
        window: &'win mut Window,
    ) -> Box<dyn GLES + 'gl_ctx> {
        if self.is_loaded && self.gl_ctx_active().is_current() {
            return Box::new(AngleGles { ctx: self });
        }

        unsafe {
            if debug_gl_enabled() {
                log!("PEAR_DEBUG_GL: binding ANGLE ES{} ctx", self.client_version);
            }
            window.make_gl_context_current(self.gl_ctx_active());
        }
        gles11::load_with(|s| window.gl_get_proc_address(s));
        gles2::load_with(|s| window.gl_get_proc_address(s));
        self.is_loaded = true;
        Box::new(AngleGles { ctx: self })
    }

    unsafe fn make_current_unchecked_for_window<'gl_ctx>(
        &'gl_ctx mut self,
        make_current_fn: &mut dyn FnMut(&GLContext),
        loader_fn: &mut dyn FnMut(&'static str) -> *const std::ffi::c_void,
    ) -> Box<dyn GLES + 'gl_ctx> {
        if self.is_loaded && self.gl_ctx_active().is_current() {
            return Box::new(AngleGles { ctx: self });
        }

        make_current_fn(self.gl_ctx_active());
        gles11::load_with(&mut *loader_fn);
        gles2::load_with(&mut *loader_fn);
        self.is_loaded = true;
        Box::new(AngleGles { ctx: self })
    }
}

pub struct AngleGles<'gl_ctx> {
    ctx: &'gl_ctx mut ANGLEContext,
}

fn debug_gl_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("PEAR_DEBUG_GL").is_ok_and(|v| !v.is_empty() && v != "0"))
}

/// ES 1.1-only enable capabilities; enabling/disabling these switches an ES
/// 2.0 guest context to its ES 1.1 companion.
fn is_es1_only_cap(cap: GLenum) -> bool {
    matches!(
        cap,
        0x0B60   // FOG
        | 0x0B50   // LIGHTING
        | 0x0BC0   // ALPHA_TEST
        | 0x0DE1   // TEXTURE_2D
        | 0x8861   // POINT_SPRITE_OES
        | 0x0B10   // POINT_SMOOTH
        | 0x0B20   // LINE_SMOOTH
        | 0x0BA1   // NORMALIZE
        | 0x803A   // RESCALE_NORMAL
        | 0x0BF2   // COLOR_LOGIC_OP
        | 0x809D   // MULTISAMPLE
        | 0x809F   // SAMPLE_ALPHA_TO_ONE
        | 0x0B57   // COLOR_MATERIAL
    ) || (0x4000..=0x4007).contains(&cap) // LIGHT0..LIGHT7
        || (0x3000..=0x3005).contains(&cap) // CLIP_PLANE0..5
        || (0x0C60..=0x0C63).contains(&cap) // TEXTURE_GEN_*
}

/// ES 1.1-only state queries; these switch an ES 2.0 guest context to its ES
/// 1.1 companion so the query is answered there.
fn is_es1_only_pname(pname: GLenum) -> bool {
    matches!(
        pname,
        0x0BA3   // MODELVIEW_STACK_DEPTH
        | 0x0BA4   // PROJECTION_STACK_DEPTH
        | 0x0BA5   // TEXTURE_STACK_DEPTH
        | 0x0BA6   // MODELVIEW_MATRIX
        | 0x0BA7   // PROJECTION_MATRIX
        | 0x0BA8   // TEXTURE_MATRIX
        | 0x0B11   // POINT_SIZE
        | 0x0B12   // POINT_SIZE_MIN
        | 0x0B13   // POINT_SIZE_MAX
        | 0x0B22   // LINE_WIDTH_RANGE
        | 0x0B23   // LINE_WIDTH_GRANULARITY
    ) || (0x0B50..=0x0B5F).contains(&pname) // lighting/material queries
        || (0x0B60..=0x0B65).contains(&pname) // fog queries
        || (0x0BC0..=0x0BC3).contains(&pname) // alpha test queries
        || (0x3000..=0x3005).contains(&pname) // clip plane queries
        || (0x4000..=0x4FFF).contains(&pname) // light queries
        || (0x2200..=0x2205).contains(&pname) // texture env queries
        || (0x8840..=0x8844).contains(&pname) // matrix palette
        || (0x8861..=0x8863).contains(&pname) // point sprite
        || pname == 0x8191 // GENERATE_MIPMAP
}

/// Bit set in [ANGLEContext::client_arrays] for a client array enable cap.
fn client_array_bit(array: GLenum) -> u8 {
    match array {
        0x8074 => 1, // VERTEX_ARRAY
        0x8076 => 2, // COLOR_ARRAY
        0x8078 => 4, // TEXTURE_COORD_ARRAY
        0x8075 => 8, // NORMAL_ARRAY
        _ => 0,
    }
}

/// Per-context state mirrored when switching between the ES 2.0 main context
/// and its ES 1.1 companion. GL objects are shared through the EGL share
/// group, but bindings and enable state are per-context.
struct MirroredGlState {
    framebuffer: GLuint,
    renderbuffer: GLuint,
    viewport: [GLint; 4],
    scissor: [GLint; 4],
    array_buffer: GLuint,
    element_array_buffer: GLuint,
    active_texture: GLenum,
    texture_units: usize,
    texture_2d: [GLuint; 8],
    texture_cube: [GLuint; 8],
    caps: [(GLenum, GLboolean); 9],
    color_mask: [GLboolean; 4],
    depth_mask: GLboolean,
    depth_func: GLenum,
    blend_src: GLenum,
    blend_dst: GLenum,
    blend_equation: GLenum,
    line_width: GLfloat,
    polygon_offset: [GLfloat; 2],
}

const MIRRORED_CAPS: [GLenum; 9] = [
    0x0BE2, // BLEND
    0x0B71, // DEPTH_TEST
    0x0B44, // CULL_FACE
    0x0B90, // STENCIL_TEST
    0x0C11, // SCISSOR_TEST
    0x0BD0, // DITHER
    0x8037, // POLYGON_OFFSET_FILL
    0x809E, // SAMPLE_ALPHA_TO_COVERAGE
    0x80A0, // SAMPLE_COVERAGE
];
const MIRRORED_TEXTURE_UNITS: usize = 8;
/// ANGLE's ES 1.1 context exposes only four texture units
/// (`maxMultitextureUnits = 4`), so `ActiveTexture` beyond TEXTURE3 is
/// `GL_INVALID_ENUM` there.
const ES1_TEXTURE_UNITS: usize = 4;

/// `GL_TEXTURE_MAX_LEVEL[_APPLE]`: iOS exposes it on ES 1.1/2.0 via
/// `GL_APPLE_texture_max_level`, but ANGLE only accepts it from ES 3.0 on.
/// It is a mip-clamping hint, so dropping it is harmless.
const TEXTURE_MAX_LEVEL_APPLE: GLenum = 0x813D;
/// `GL_TEXTURE_MAX_ANISOTROPY_EXT` (`GL_EXT_texture_filter_anisotropic`).
const TEXTURE_MAX_ANISOTROPY_EXT: GLenum = 0x84FE;
/// Caps in the `GL_LIGHT*` block past `GL_LIGHT7`; games commonly loop
/// `glDisable(GL_LIGHT0 + i)` over a range larger than the implementation
/// supports, which is `GL_INVALID_ENUM` everywhere.
fn is_inexistent_light_cap(cap: GLenum) -> bool {
    (0x4008..=0x4FFF).contains(&cap)
}

/// Caps that the ES 1.1 companion cannot accept; binding them there would
/// raise `GL_INVALID_ENUM` and leak a stale error into the guest.
fn cap_supported_on_es1(cap: GLenum) -> bool {
    !matches!(cap, 0x809E | 0x80A0) // SAMPLE_ALPHA_TO_COVERAGE, SAMPLE_COVERAGE
}

/// Diagnostic: drain and label any GL error raised by the state mirror's own
/// internal calls (capture/apply), so they are never misattributed to the
/// guest call whose guard window contains the context switch.
/// Diagnostic: drain and label any GL error raised by the state mirror's own
/// internal calls (capture/apply), so they are never misattributed to the
/// guest call whose guard window contains the context switch.
unsafe fn audit(label: &str) {
    if debug_gl_enabled() {
        drain_errors(label);
    }
}

/// Silently discard any GL error left behind by the mirror's internal calls,
/// so they can never surface in the guest's error queue.
unsafe fn drain_errors(label: &str) {
    let debug = debug_gl_enabled();
    loop {
        let err = gles2::GetError();
        if err == 0 {
            break;
        }
        if debug {
            log!("PEAR_DEBUG_GL: internal {} raised {:#x}", label, err);
        }
    }
}

unsafe fn get_int(pname: GLenum) -> GLint {
    let mut v: GLint = 0;
    gles2::GetIntegerv(pname, &mut v);
    if debug_gl_enabled() {
        audit(&format!("GetIntegerv(0x{:x})", pname));
    }
    v
}
unsafe fn get_float(pname: GLenum) -> GLfloat {
    let mut v: GLfloat = 0.0;
    gles2::GetFloatv(pname, &mut v);
    if debug_gl_enabled() {
        audit(&format!("GetFloatv(0x{:x})", pname));
    }
    v
}
unsafe fn get_bool(pname: GLenum) -> GLboolean {
    let mut v: GLboolean = 0;
    gles2::GetBooleanv(pname, &mut v);
    if debug_gl_enabled() {
        audit(&format!("GetBooleanv(0x{:x})", pname));
    }
    v
}

unsafe fn capture_gl_state(from_es1: bool) -> MirroredGlState {
    let active_texture = get_int(0x84E0) as GLenum; // ACTIVE_TEXTURE
                                                    // The ES 1.1 companion only has four units. Querying TEXTURE4..7 there
                                                    // would leave GL_INVALID_ENUM behind for the guest to observe.
    let texture_units = if from_es1 {
        ES1_TEXTURE_UNITS
    } else {
        MIRRORED_TEXTURE_UNITS
    };
    let mut texture_2d = [0; MIRRORED_TEXTURE_UNITS];
    let mut texture_cube = [0; MIRRORED_TEXTURE_UNITS];
    for i in 0..texture_units {
        gles2::ActiveTexture(0x84C0 + i as GLenum); // TEXTURE0 + i
        texture_2d[i] = get_int(0x8069) as GLuint; // TEXTURE_BINDING_2D
        texture_cube[i] = get_int(0x8514) as GLuint; // TEXTURE_BINDING_CUBE_MAP
    }
    gles2::ActiveTexture(active_texture);
    audit("capture ActiveTexture restore");

    let mut caps = [(0, 0); 9];
    for (slot, &cap) in caps.iter_mut().zip(MIRRORED_CAPS.iter()) {
        *slot = (cap, get_bool(cap));
    }

    let mut viewport = [0; 4];
    gles2::GetIntegerv(0x0BA2, viewport.as_mut_ptr()); // VIEWPORT
    audit("capture GetIntegerv(0xba2 VIEWPORT)");
    let mut scissor = [0; 4];
    gles2::GetIntegerv(0x0C10, scissor.as_mut_ptr()); // SCISSOR_BOX
    audit("capture GetIntegerv(0xc10 SCISSOR_BOX)");
    let mut color_mask = [0; 4];
    gles2::GetBooleanv(0x0C23, color_mask.as_mut_ptr()); // COLOR_WRITEMASK
    audit("capture GetBooleanv(0xc23 COLOR_WRITEMASK)");

    MirroredGlState {
        framebuffer: get_int(0x8CA6) as GLuint,  // FRAMEBUFFER_BINDING
        renderbuffer: get_int(0x8CA7) as GLuint, // RENDERBUFFER_BINDING
        viewport,
        scissor,
        // The ES 1.1 companion has no OES_vertex_buffer_object, so these
        // pnames are INVALID_ENUM there; its bindings are always 0 anyway.
        array_buffer: if from_es1 {
            0
        } else {
            get_int(0x8894) as GLuint
        }, // ARRAY_BUFFER_BINDING
        element_array_buffer: if from_es1 {
            0
        } else {
            get_int(0x8895) as GLuint
        }, // ELEMENT_ARRAY_BUFFER_BINDING
        active_texture,
        texture_units,
        texture_2d,
        texture_cube,
        caps,
        color_mask,
        depth_mask: get_bool(0x0B72),          // DEPTH_WRITEMASK
        depth_func: get_int(0x0B74) as GLenum, // DEPTH_FUNC
        // BLEND_SRC/BLEND_DST (0x0BE1/0x0BE0) are ES 1.1-only pnames;
        // ES 2.0+ exposes the same state as BLEND_SRC_RGB/BLEND_DST_RGB.
        blend_src: if from_es1 {
            get_int(0x0BE1) as GLenum
        } else {
            get_int(0x80C9) as GLenum
        },
        blend_dst: if from_es1 {
            get_int(0x0BE0) as GLenum
        } else {
            get_int(0x80C8) as GLenum
        },
        blend_equation: get_int(0x8009) as GLenum, // BLEND_EQUATION_OES / _RGB
        line_width: get_float(0x0B21),             // LINE_WIDTH
        polygon_offset: [get_float(0x8038), get_float(0x2A00)], // FACTOR, UNITS
    }
}

unsafe fn apply_gl_state(s: &MirroredGlState, es1_target: bool) {
    if es1_target {
        gles11::BindFramebufferOES(0x8D40, s.framebuffer); // FRAMEBUFFER_OES
        gles11::BindRenderbufferOES(0x8D41, s.renderbuffer); // RENDERBUFFER_OES
    } else {
        gles2::BindFramebuffer(0x8D40, s.framebuffer);
        gles2::BindRenderbuffer(0x8D41, s.renderbuffer);
    }
    audit("apply BindFramebuffer/BindRenderbuffer");
    if !es1_target {
        // The ES 1.1 companion has no OES_vertex_buffer_object.
        gles11::BindBuffer(0x8892, s.array_buffer); // ARRAY_BUFFER
        gles11::BindBuffer(0x8893, s.element_array_buffer); // ELEMENT_ARRAY_BUFFER
    }
    for i in 0..s.texture_units {
        gles11::ActiveTexture(0x84C0 + i as GLenum);
        gles11::BindTexture(0x0DE1, s.texture_2d[i]); // TEXTURE_2D
        gles11::BindTexture(0x8513, s.texture_cube[i]); // TEXTURE_CUBE_MAP
    }
    let restore_texture = if es1_target && s.active_texture >= 0x84C0 + ES1_TEXTURE_UNITS as GLenum
    {
        0x84C0
    } else {
        s.active_texture
    };
    gles11::ActiveTexture(restore_texture);
    audit("apply texture binds");
    for &(cap, enabled) in s.caps.iter() {
        if es1_target && !cap_supported_on_es1(cap) {
            continue;
        }
        if enabled != 0 {
            gles11::Enable(cap);
        } else {
            gles11::Disable(cap);
        }
    }
    audit("apply caps Enable/Disable");
    gles11::Viewport(s.viewport[0], s.viewport[1], s.viewport[2], s.viewport[3]);
    gles11::Scissor(s.scissor[0], s.scissor[1], s.scissor[2], s.scissor[3]);
    audit("apply Viewport/Scissor");
    gles11::ColorMask(
        s.color_mask[0],
        s.color_mask[1],
        s.color_mask[2],
        s.color_mask[3],
    );
    gles11::DepthMask(s.depth_mask);
    gles11::DepthFunc(s.depth_func);
    audit("apply DepthMask/DepthFunc");
    gles11::BlendFunc(s.blend_src, s.blend_dst);
    if es1_target {
        gles11::BlendEquationOES(s.blend_equation);
    } else {
        gles2::BlendEquation(s.blend_equation);
    }
    audit("apply BlendFunc/BlendEquation");
    gles11::LineWidth(s.line_width);
    gles11::PolygonOffset(s.polygon_offset[0], s.polygon_offset[1]);
    audit("apply LineWidth/PolygonOffset");
}

impl AngleGles<'_> {
    /// Route a following ES 1.1 fixed-function call. For guest ES 2.0
    /// contexts this switches to the ES 1.1 companion, mirroring binding
    /// state across; the reverse happens on [Self::es2_call].
    unsafe fn es1_call(&mut self) {
        if self.ctx.client_version < 2 || self.ctx.using_es1 {
            return;
        }
        self.switch_context(true);
    }

    /// Route a following ES 2.0 programmable-pipeline call, switching back
    /// to the main context if ES 1.1 calls redirected us to the companion.
    unsafe fn es2_call(&mut self) {
        self.ctx.saw_es2_shader = true;
        if self.ctx.client_version < 2 || !self.ctx.using_es1 {
            return;
        }
        self.switch_context(false);
    }

    /// Draw calls follow the pipeline in use: fixed-function draws (client
    /// arrays, or no shader program bound) run on the ES 1.1 companion and
    /// shader draws on the ES 2.0 main context.
    unsafe fn draw_call(&mut self) {
        if self.ctx.client_version < 2 || !self.ctx.saw_es2_shader {
            return;
        }
        if self.ctx.client_arrays != 0 {
            self.es1_call();
        } else if self.ctx.current_program != 0 {
            self.es2_call();
        }
    }

    /// Decide where a `TexParameter*` pname must be sent.
    ///
    /// Returns `false` when the parameter has to be dropped: it either cannot
    /// be represented in ANGLE at all, or it belongs to an iOS extension that
    /// the available context does not provide.
    unsafe fn route_tex_parameter(&mut self, pname: GLenum) -> bool {
        match pname {
            // ANGLE rejects this below ES 3.0 (kES3Required), but iOS accepts
            // it on ES 1.1/2.0. It only clamps the highest mip level.
            TEXTURE_MAX_LEVEL_APPLE => false,
            // The ES 1.1 companion never has
            // GL_EXT_texture_filter_anisotropic, and neither does ANGLE's ES 1.1
            // context, so the only place this can work is the ES 2.0 main
            // context. Anisotropy is a quality hint, so dropping it when there
            // is no ES 2.0 context is fine.
            TEXTURE_MAX_ANISOTROPY_EXT => {
                if self.ctx.client_version < 2 {
                    return false;
                }
                self.switch_context(false);
                true
            }
            // ES 1.1-only texture parameters.
            0x8191 | 0x8B9D => {
                self.es1_call();
                true
            }
            _ => true,
        }
    }

    unsafe fn switch_context(&mut self, to_es1: bool) {
        let mirror = capture_gl_state(self.ctx.using_es1);
        drain_errors("capture_gl_state");
        if to_es1 {
            self.ctx.es1_fallback.as_ref().unwrap().make_current_raw();
        } else {
            self.ctx.gl_ctx.make_current_raw();
        }
        self.ctx.using_es1 = to_es1;
        apply_gl_state(&mirror, to_es1);
        // The mirror's own internal calls may raise on the destination
        // context (unsupported caps / texture units / pnames). Those are
        // implementation details of the mirror, so never let them show up
        // in the guest's error queue.
        drain_errors("apply_gl_state");
        if debug_gl_enabled() {
            log!(
                "PEAR_DEBUG_GL: routed to {} of guest ES2 context",
                if to_es1 { "ES1 companion" } else { "ES2 main" }
            );
            let version = CStr::from_ptr(gles11::GetString(gles11::VERSION) as *const _);
            log!(
                "PEAR_DEBUG_GL: now on \"{}\" (EGL ctx {:?})",
                version.to_string_lossy(),
                crate::egl::global().current_context()
            );
        }
    }
}

impl GLES for AngleGles<'_> {
    unsafe fn driver_description(&self) -> String {
        let version = CStr::from_ptr(gles11::GetString(gles11::VERSION) as *const _);
        let vendor = CStr::from_ptr(gles11::GetString(gles11::VENDOR) as *const _);
        let renderer = CStr::from_ptr(gles11::GetString(gles11::RENDERER) as *const _);
        // OpenGL ES requires the version to be prefixed "OpenGL ES", so we
        // don't need to contextualize it.
        format!(
            "{} / {} / {}",
            version.to_string_lossy(),
            vendor.to_string_lossy(),
            renderer.to_string_lossy()
        )
    }

    // Generic state manipulation
    unsafe fn GetError(&mut self) -> GLenum {
        gles11::GetError()
    }
    unsafe fn Enable(&mut self, cap: GLenum) {
        let _t = gl_trace("Enable");
        if debug_gl_enabled() {
            log!("PEAR_DEBUG_GL: Enable(0x{:x})", cap);
        }
        if is_inexistent_light_cap(cap) {
            return;
        }
        if is_es1_only_cap(cap) {
            self.es1_call();
        }
        gles11::Enable(cap)
    }
    unsafe fn IsEnabled(&mut self, cap: GLenum) -> GLboolean {
        if is_inexistent_light_cap(cap) {
            return 0;
        }
        if is_es1_only_cap(cap) {
            self.es1_call();
        }
        gles11::IsEnabled(cap)
    }
    unsafe fn Disable(&mut self, cap: GLenum) {
        let _t = gl_trace("Disable");
        if debug_gl_enabled() {
            log!("PEAR_DEBUG_GL: Disable(0x{:x})", cap);
        }
        if is_inexistent_light_cap(cap) {
            return;
        }
        if is_es1_only_cap(cap) {
            self.es1_call();
        }
        gles11::Disable(cap)
    }
    unsafe fn ClientActiveTexture(&mut self, texture: GLenum) {
        self.es1_call();
        gles11::ClientActiveTexture(texture);
    }
    unsafe fn EnableClientState(&mut self, array: GLenum) {
        let _t = gl_trace("EnableClientState");
        if debug_gl_enabled() {
            log!("PEAR_DEBUG_GL: EnableClientState(0x{:x})", array);
        }
        self.ctx.client_arrays |= client_array_bit(array);
        self.es1_call();
        gles11::EnableClientState(array)
    }
    unsafe fn DisableClientState(&mut self, array: GLenum) {
        let _t = gl_trace("DisableClientState");
        if debug_gl_enabled() {
            log!("PEAR_DEBUG_GL: DisableClientState(0x{:x})", array);
        }
        self.ctx.client_arrays &= !client_array_bit(array);
        self.es1_call();
        gles11::DisableClientState(array)
    }
    unsafe fn GetBooleanv(&mut self, self_pname: GLenum, params: *mut GLboolean) {
        let pname = self_pname;
        if debug_gl_enabled() {
            log!("PEAR_DEBUG_GL: GetBooleanv(pname=0x{:x})", pname);
        }
        if is_es1_only_pname(pname) {
            self.es1_call();
        }
        gles11::GetBooleanv(pname, params)
    }
    unsafe fn GetFloatv(&mut self, pname: GLenum, params: *mut GLfloat) {
        if debug_gl_enabled() {
            log!("PEAR_DEBUG_GL: GetFloatv(pname=0x{:x})", pname);
        }
        if is_es1_only_pname(pname) {
            self.es1_call();
        }
        gles11::GetFloatv(pname, params)
    }
    unsafe fn GetIntegerv(&mut self, pname: GLenum, params: *mut GLint) {
        if debug_gl_enabled() {
            log!("PEAR_DEBUG_GL: GetIntegerv(pname=0x{:x})", pname);
        }
        if is_es1_only_pname(pname) {
            self.es1_call();
        }
        gles11::GetIntegerv(pname, params)
    }
    unsafe fn GetTexEnviv(&mut self, target: GLenum, pname: GLenum, params: *mut GLint) {
        self.es1_call();
        gles11::GetTexEnviv(target, pname, params)
    }
    unsafe fn GetTexEnvfv(&mut self, target: GLenum, pname: GLenum, params: *mut GLfloat) {
        self.es1_call();
        gles11::GetTexEnvfv(target, pname, params)
    }
    unsafe fn GetPointerv(&mut self, pname: GLenum, params: *mut *const GLvoid) {
        self.es1_call();
        // The second argument to glGetPointerv must be a mutable pointer,
        // but gl_generator generates the wrong signature by mistake, see
        // https://github.com/brendanzab/gl-rs/issues/541
        gles11::GetPointerv(pname, params as *mut _ as *const _)
    }
    unsafe fn Hint(&mut self, target: GLenum, mode: GLenum) {
        gles11::Hint(target, mode)
    }
    unsafe fn Finish(&mut self) {
        gles11::Finish()
    }
    unsafe fn Flush(&mut self) {
        gles11::Flush()
    }
    unsafe fn GetString(&mut self, name: GLenum) -> *const GLubyte {
        gles11::GetString(name)
    }

    // Other state manipulation
    unsafe fn AlphaFunc(&mut self, func: GLenum, ref_: GLclampf) {
        self.es1_call();
        gles11::AlphaFunc(func, ref_)
    }
    unsafe fn AlphaFuncx(&mut self, func: GLenum, ref_: GLclampx) {
        self.es1_call();
        gles11::AlphaFuncx(func, ref_)
    }
    unsafe fn BlendFunc(&mut self, sfactor: GLenum, dfactor: GLenum) {
        gles11::BlendFunc(sfactor, dfactor)
    }
    unsafe fn BlendEquationOES(&mut self, mode: GLenum) {
        if self.ctx.client_version >= 2 && !self.ctx.using_es1 {
            gles2::BlendEquation(mode)
        } else {
            gles11::BlendEquationOES(mode)
        }
    }
    unsafe fn ColorMask(
        &mut self,
        red: GLboolean,
        green: GLboolean,
        blue: GLboolean,
        alpha: GLboolean,
    ) {
        gles11::ColorMask(red, green, blue, alpha)
    }
    unsafe fn ClipPlanef(&mut self, plane: GLenum, equation: *const GLfloat) {
        self.es1_call();
        gles11::ClipPlanef(plane, equation)
    }
    unsafe fn ClipPlanex(&mut self, plane: GLenum, equation: *const GLfixed) {
        self.es1_call();
        gles11::ClipPlanex(plane, equation)
    }
    unsafe fn CullFace(&mut self, mode: GLenum) {
        gles11::CullFace(mode)
    }
    unsafe fn DepthFunc(&mut self, func: GLenum) {
        gles11::DepthFunc(func)
    }
    unsafe fn DepthMask(&mut self, flag: GLboolean) {
        gles11::DepthMask(flag)
    }
    unsafe fn FrontFace(&mut self, mode: GLenum) {
        gles11::FrontFace(mode)
    }
    unsafe fn DepthRangef(&mut self, near: GLclampf, far: GLclampf) {
        gles11::DepthRangef(near, far)
    }
    unsafe fn DepthRangex(&mut self, near: GLclampx, far: GLclampx) {
        self.es1_call();
        gles11::DepthRangex(near, far)
    }
    unsafe fn PolygonOffset(&mut self, factor: GLfloat, units: GLfloat) {
        gles11::PolygonOffset(factor, units)
    }
    unsafe fn PolygonOffsetx(&mut self, factor: GLfixed, units: GLfixed) {
        gles11::PolygonOffsetx(factor, units)
    }
    unsafe fn SampleCoverage(&mut self, value: GLclampf, invert: GLboolean) {
        gles11::SampleCoverage(value, invert)
    }
    unsafe fn SampleCoveragex(&mut self, value: GLclampx, invert: GLboolean) {
        gles11::SampleCoveragex(value, invert)
    }
    unsafe fn ShadeModel(&mut self, mode: GLenum) {
        self.es1_call();
        gles11::ShadeModel(mode)
    }
    unsafe fn Scissor(&mut self, x: GLint, y: GLint, width: GLsizei, height: GLsizei) {
        gles11::Scissor(x, y, width, height)
    }
    unsafe fn Viewport(&mut self, x: GLint, y: GLint, width: GLsizei, height: GLsizei) {
        gles11::Viewport(x, y, width, height)
    }
    unsafe fn LineWidth(&mut self, val: GLfloat) {
        gles11::LineWidth(val)
    }
    unsafe fn LineWidthx(&mut self, val: GLfixed) {
        self.es1_call();
        gles11::LineWidthx(val)
    }
    unsafe fn StencilFunc(&mut self, func: GLenum, ref_: GLint, mask: GLuint) {
        gles11::StencilFunc(func, ref_, mask);
    }
    unsafe fn StencilOp(&mut self, sfail: GLenum, dpfail: GLenum, dppass: GLenum) {
        gles11::StencilOp(sfail, dpfail, dppass);
    }
    unsafe fn StencilMask(&mut self, mask: GLuint) {
        gles11::StencilMask(mask);
    }
    unsafe fn LogicOp(&mut self, opcode: GLenum) {
        self.es1_call();
        gles11::LogicOp(opcode);
    }

    // Points
    unsafe fn PointSize(&mut self, size: GLfloat) {
        self.es1_call();
        gles11::PointSize(size)
    }
    unsafe fn PointSizex(&mut self, size: GLfixed) {
        self.es1_call();
        gles11::PointSizex(size)
    }
    unsafe fn PointParameterf(&mut self, pname: GLenum, param: GLfloat) {
        self.es1_call();
        gles11::PointParameterf(pname, param)
    }
    unsafe fn PointParameterx(&mut self, pname: GLenum, param: GLfixed) {
        self.es1_call();
        gles11::PointParameterx(pname, param)
    }
    unsafe fn PointParameterfv(&mut self, pname: GLenum, params: *const GLfloat) {
        self.es1_call();
        gles11::PointParameterfv(pname, params)
    }
    unsafe fn PointParameterxv(&mut self, pname: GLenum, params: *const GLfixed) {
        self.es1_call();
        gles11::PointParameterxv(pname, params)
    }

    // Lighting and materials
    unsafe fn Fogf(&mut self, pname: GLenum, param: GLfloat) {
        self.es1_call();
        gles11::Fogf(pname, param)
    }
    unsafe fn Fogx(&mut self, pname: GLenum, param: GLfixed) {
        self.es1_call();
        gles11::Fogx(pname, param)
    }
    unsafe fn Fogfv(&mut self, pname: GLenum, params: *const GLfloat) {
        self.es1_call();
        gles11::Fogfv(pname, params)
    }
    unsafe fn Fogxv(&mut self, pname: GLenum, params: *const GLfixed) {
        self.es1_call();
        gles11::Fogxv(pname, params)
    }
    unsafe fn Lightf(&mut self, light: GLenum, pname: GLenum, param: GLfloat) {
        self.es1_call();
        gles11::Lightf(light, pname, param)
    }
    unsafe fn Lightx(&mut self, light: GLenum, pname: GLenum, param: GLfixed) {
        self.es1_call();
        gles11::Lightx(light, pname, param)
    }
    unsafe fn Lightfv(&mut self, light: GLenum, pname: GLenum, params: *const GLfloat) {
        self.es1_call();
        gles11::Lightfv(light, pname, params)
    }
    unsafe fn Lightxv(&mut self, light: GLenum, pname: GLenum, params: *const GLfixed) {
        self.es1_call();
        gles11::Lightxv(light, pname, params)
    }
    unsafe fn LightModelf(&mut self, pname: GLenum, param: GLfloat) {
        self.es1_call();
        gles11::LightModelf(pname, param)
    }
    unsafe fn LightModelx(&mut self, pname: GLenum, param: GLfixed) {
        self.es1_call();
        gles11::LightModelx(pname, param)
    }
    unsafe fn LightModelfv(&mut self, pname: GLenum, params: *const GLfloat) {
        self.es1_call();
        gles11::LightModelfv(pname, params)
    }
    unsafe fn LightModelxv(&mut self, pname: GLenum, params: *const GLfixed) {
        self.es1_call();
        gles11::LightModelxv(pname, params)
    }
    unsafe fn Materialf(&mut self, face: GLenum, pname: GLenum, param: GLfloat) {
        self.es1_call();
        gles11::Materialf(face, pname, param)
    }
    unsafe fn Materialx(&mut self, face: GLenum, pname: GLenum, param: GLfixed) {
        self.es1_call();
        gles11::Materialx(face, pname, param)
    }
    unsafe fn Materialfv(&mut self, face: GLenum, pname: GLenum, params: *const GLfloat) {
        self.es1_call();
        gles11::Materialfv(face, pname, params)
    }
    unsafe fn Materialxv(&mut self, face: GLenum, pname: GLenum, params: *const GLfixed) {
        self.es1_call();
        gles11::Materialxv(face, pname, params)
    }

    // Buffers
    unsafe fn IsBuffer(&mut self, buffer: GLuint) -> GLboolean {
        gles11::IsBuffer(buffer)
    }
    unsafe fn GenBuffers(&mut self, n: GLsizei, buffers: *mut GLuint) {
        gles11::GenBuffers(n, buffers)
    }
    unsafe fn DeleteBuffers(&mut self, n: GLsizei, buffers: *const GLuint) {
        gles11::DeleteBuffers(n, buffers)
    }
    unsafe fn BindBuffer(&mut self, target: GLenum, buffer: GLuint) {
        assert!(target == gles11::ARRAY_BUFFER || target == gles11::ELEMENT_ARRAY_BUFFER);
        gles11::BindBuffer(target, buffer)
    }
    unsafe fn BufferData(
        &mut self,
        target: GLenum,
        size: GLsizeiptr,
        data: *const GLvoid,
        usage: GLenum,
    ) {
        let _t = gl_trace("BufferData");
        assert!(target == gles11::ARRAY_BUFFER || target == gles11::ELEMENT_ARRAY_BUFFER);
        gles11::BufferData(target, size, data, usage)
    }

    unsafe fn BufferSubData(
        &mut self,
        target: GLenum,
        offset: GLintptr,
        size: GLsizeiptr,
        data: *const GLvoid,
    ) {
        assert!(target == gles11::ARRAY_BUFFER || target == gles11::ELEMENT_ARRAY_BUFFER);
        gles11::BufferSubData(target, offset, size, data)
    }

    // Non-pointers
    unsafe fn Color4f(&mut self, red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat) {
        self.es1_call();
        gles11::Color4f(red, green, blue, alpha)
    }
    unsafe fn Color4x(&mut self, red: GLfixed, green: GLfixed, blue: GLfixed, alpha: GLfixed) {
        self.es1_call();
        gles11::Color4x(red, green, blue, alpha)
    }
    unsafe fn Color4ub(&mut self, red: GLubyte, green: GLubyte, blue: GLubyte, alpha: GLubyte) {
        self.es1_call();
        gles11::Color4ub(red, green, blue, alpha)
    }
    unsafe fn Normal3f(&mut self, nx: GLfloat, ny: GLfloat, nz: GLfloat) {
        self.es1_call();
        gles11::Normal3f(nx, ny, nz)
    }
    unsafe fn Normal3x(&mut self, nx: GLfixed, ny: GLfixed, nz: GLfixed) {
        self.es1_call();
        gles11::Normal3x(nx, ny, nz)
    }

    // Pointers
    unsafe fn ColorPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        let _t = gl_trace("ColorPointer");
        self.es1_call();
        gles11::ColorPointer(size, type_, stride, pointer)
    }
    unsafe fn NormalPointer(&mut self, type_: GLenum, stride: GLsizei, pointer: *const GLvoid) {
        let _t = gl_trace("NormalPointer");
        self.es1_call();
        gles11::NormalPointer(type_, stride, pointer)
    }
    unsafe fn TexCoordPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        let _t = gl_trace("TexCoordPointer");
        self.es1_call();
        gles11::TexCoordPointer(size, type_, stride, pointer)
    }
    unsafe fn VertexPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        let _t = gl_trace("VertexPointer");
        self.es1_call();
        gles11::VertexPointer(size, type_, stride, pointer)
    }

    // Drawing
    unsafe fn DrawArrays(&mut self, mode: GLenum, first: GLint, count: GLsizei) {
        let _t = gl_trace("DrawArrays");
        self.draw_call();
        if debug_gl_enabled() {
            let mut fb: GLint = 0;
            gles2::GetIntegerv(0x8CA6, &mut fb);
            let status = if self.ctx.client_version >= 2 && !self.ctx.using_es1 {
                gles2::CheckFramebufferStatus(0x8D40)
            } else {
                gles11::CheckFramebufferStatusOES(0x8D40)
            };
            log!(
                "PEAR_DEBUG_GL: DrawArrays({:#x}, {}, {}): fb={} status={:#x}",
                mode,
                first,
                count,
                fb,
                status
            );
        }
        gles11::DrawArrays(mode, first, count)
    }
    unsafe fn DrawElements(
        &mut self,
        mode: GLenum,
        count: GLsizei,
        type_: GLenum,
        indices: *const GLvoid,
    ) {
        let _t = gl_trace("DrawElements");
        self.draw_call();
        if debug_gl_enabled() {
            let mut fb: GLint = 0;
            gles2::GetIntegerv(0x8CA6, &mut fb);
            let status = if self.ctx.client_version >= 2 && !self.ctx.using_es1 {
                gles2::CheckFramebufferStatus(0x8D40)
            } else {
                gles11::CheckFramebufferStatusOES(0x8D40)
            };
            log!(
                "PEAR_DEBUG_GL: DrawElements({:#x}, {}, {:#x}): fb={} status={:#x}",
                mode,
                count,
                type_,
                fb,
                status
            );
        }
        gles11::DrawElements(mode, count, type_, indices)
    }

    // Clearing
    unsafe fn Clear(&mut self, mask: GLbitfield) {
        gles11::Clear(mask)
    }
    unsafe fn ClearColor(
        &mut self,
        red: GLclampf,
        green: GLclampf,
        blue: GLclampf,
        alpha: GLclampf,
    ) {
        gles11::ClearColor(red, green, blue, alpha)
    }
    unsafe fn ClearColorx(
        &mut self,
        red: GLclampx,
        green: GLclampx,
        blue: GLclampx,
        alpha: GLclampx,
    ) {
        self.es1_call();
        gles11::ClearColorx(red, green, blue, alpha)
    }
    unsafe fn ClearDepthf(&mut self, depth: GLclampf) {
        gles11::ClearDepthf(depth)
    }
    unsafe fn ClearDepthx(&mut self, depth: GLclampx) {
        self.es1_call();
        gles11::ClearDepthx(depth)
    }
    unsafe fn ClearStencil(&mut self, s: GLint) {
        gles11::ClearStencil(s)
    }

    // Textures
    unsafe fn PixelStorei(&mut self, pname: GLenum, param: GLint) {
        let _t = gl_trace("PixelStorei");
        if debug_gl_enabled() {
            audit("PixelStorei stale-before");
        }
        gles11::PixelStorei(pname, param)
    }
    unsafe fn ReadPixels(
        &mut self,
        x: GLint,
        y: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        type_: GLenum,
        pixels: *mut GLvoid,
    ) {
        gles11::ReadPixels(x, y, width, height, format, type_, pixels)
    }
    unsafe fn GenTextures(&mut self, n: GLsizei, textures: *mut GLuint) {
        let _t = gl_trace("GenTextures");
        if debug_gl_enabled() {
            audit("GenTextures stale-before");
        }
        gles11::GenTextures(n, textures)
    }
    unsafe fn DeleteTextures(&mut self, n: GLsizei, textures: *const GLuint) {
        let _t = gl_trace("DeleteTextures");
        if debug_gl_enabled() {
            audit("DeleteTextures stale-before");
        }
        gles11::DeleteTextures(n, textures)
    }
    unsafe fn ActiveTexture(&mut self, texture: GLenum) {
        gles11::ActiveTexture(texture)
    }
    unsafe fn IsTexture(&mut self, texture: GLuint) -> GLboolean {
        gles11::IsTexture(texture)
    }
    unsafe fn BindTexture(&mut self, target: GLenum, texture: GLuint) {
        let _t = gl_trace("BindTexture");
        if debug_gl_enabled() {
            audit("BindTexture stale-before");
            log!("PEAR_DEBUG_GL: BindTexture({:#x}, {})", target, texture);
        }
        gles11::BindTexture(target, texture);
        if debug_gl_enabled() {
            audit("BindTexture replay");
        }
    }
    unsafe fn TexParameteri(&mut self, target: GLenum, pname: GLenum, param: GLint) {
        let _t = gl_trace("TexParameteri");
        if debug_gl_enabled() {
            log!(
                "PEAR_DEBUG_GL: TexParameteri({:#x}, {:#x}, {})",
                target,
                pname,
                param
            );
            audit("TexParameteri stale-before");
        }
        if !self.route_tex_parameter(pname) {
            return;
        }
        gles11::TexParameteri(target, pname, param);
        if debug_gl_enabled() {
            audit("TexParameteri replay");
        }
    }
    unsafe fn TexParameterf(&mut self, target: GLenum, pname: GLenum, param: GLfloat) {
        let _t = gl_trace("TexParameterf");
        if debug_gl_enabled() {
            log!(
                "PEAR_DEBUG_GL: TexParameterf({:#x}, {:#x}, {})",
                target,
                pname,
                param
            );
        }
        if !self.route_tex_parameter(pname) {
            return;
        }
        gles11::TexParameterf(target, pname, param)
    }
    unsafe fn TexParameterx(&mut self, target: GLenum, pname: GLenum, param: GLfixed) {
        if !self.route_tex_parameter(pname) {
            return;
        }
        gles11::TexParameterx(target, pname, param)
    }
    unsafe fn TexParameteriv(&mut self, target: GLenum, pname: GLenum, params: *const GLint) {
        let _t = gl_trace("TexParameteriv");
        if !self.route_tex_parameter(pname) {
            return;
        }
        gles11::TexParameteriv(target, pname, params)
    }
    unsafe fn TexParameterfv(&mut self, target: GLenum, pname: GLenum, params: *const GLfloat) {
        let _t = gl_trace("TexParameterfv");
        if !self.route_tex_parameter(pname) {
            return;
        }
        gles11::TexParameterfv(target, pname, params)
    }
    unsafe fn TexParameterxv(&mut self, target: GLenum, pname: GLenum, params: *const GLfixed) {
        if !self.route_tex_parameter(pname) {
            return;
        }
        gles11::TexParameterxv(target, pname, params)
    }
    unsafe fn TexImage2D(
        &mut self,
        target: GLenum,
        level: GLint,
        mut internalformat: GLint,
        width: GLsizei,
        height: GLsizei,
        border: GLint,
        format: GLenum,
        type_: GLenum,
        pixels: *const GLvoid,
    ) {
        let _t = gl_trace("TexImage2D");
        if format == gles11::BGRA_EXT {
            // This is needed in order to avoid white screen issue on Android!
            // As per BGRA extension specs
            // https://registry.khronos.org/OpenGL/extensions/EXT/EXT_texture_format_BGRA8888.txt,
            // both internalformat and format should be BGRA
            // Tangentially related issue
            // (actually a reverse of what we're doing here)
            // https://android-review.googlesource.com/c/platform/external/qemu/+/974666
            internalformat = gles11::BGRA_EXT as GLint
        }
        gles11::TexImage2D(
            target,
            level,
            internalformat,
            width,
            height,
            border,
            format,
            type_,
            pixels,
        )
    }
    unsafe fn TexSubImage2D(
        &mut self,
        target: GLenum,
        level: GLint,
        xoffset: GLint,
        yoffset: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        type_: GLenum,
        pixels: *const GLvoid,
    ) {
        let _t = gl_trace("TexSubImage2D");
        gles11::TexSubImage2D(
            target, level, xoffset, yoffset, width, height, format, type_, pixels,
        )
    }
    unsafe fn CompressedTexImage2D(
        &mut self,
        target: GLenum,
        level: GLint,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
        border: GLint,
        image_size: GLsizei,
        data: *const GLvoid,
    ) {
        let data = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), image_size as usize) };
        // IMG_texture_compression_pvrtc (only on Imagination/Apple GPUs)
        // TODO: It would be more efficient to use hardware decoding where
        // available (I just don't have a suitable device to try this on)
        if try_decode_pvrtc(
            self,
            target,
            level,
            internalformat,
            width,
            height,
            border,
            data,
        ) {
            log_dbg!("Decoded PVRTC");
            return;
        }

        // OES_compressed_paletted_texture is an OpenGL ES 1.1 format that
        // ANGLE's Metal backend does not necessarily support, so decompress
        // those formats here (as the old GL 2.1 backend had to).
        if let Some(PalettedTextureFormat {
            index_is_nibble,
            palette_entry_format,
            palette_entry_type,
        }) = PalettedTextureFormat::get_info(internalformat)
        {
            // This should be invalid use? (TODO)
            assert!(border == 0);

            let palette_entry_size = match palette_entry_type {
                gles11::UNSIGNED_BYTE => match palette_entry_format {
                    gles11::RGB => 3,
                    gles11::RGBA => 4,
                    _ => unreachable!(),
                },
                gles11::UNSIGNED_SHORT_5_6_5
                | gles11::UNSIGNED_SHORT_4_4_4_4
                | gles11::UNSIGNED_SHORT_5_5_5_1 => 2,
                _ => unreachable!(),
            };
            let palette_entry_count = match index_is_nibble {
                true => 16,
                false => 256,
            };
            let palette_size = palette_entry_size * palette_entry_count;

            let index_count = width as usize * height as usize;
            let (index_word_size, index_word_count) = match index_is_nibble {
                true => (1, index_count.div_ceil(2)),
                false => (4, index_count.div_ceil(4)),
            };
            let indices_size = index_word_size * index_word_count;

            // TODO: support multiple miplevels in one image
            assert!(level == 0);
            assert_eq!(data.len(), palette_size + indices_size);
            let (palette, indices) = data.split_at(palette_size);

            let mut decoded = Vec::<u8>::with_capacity(palette_entry_size * index_count);
            for i in 0..index_count {
                let index = if index_is_nibble {
                    (indices[i / 2] >> ((1 - (i % 2)) * 4)) & 0xf
                } else {
                    indices[i]
                } as usize;
                let palette_entry = &palette[index * palette_entry_size..][..palette_entry_size];
                decoded.extend_from_slice(palette_entry);
            }
            assert!(decoded.len() == palette_entry_size * index_count);

            log_dbg!("Decoded paletted texture");
            gles11::TexImage2D(
                target,
                level,
                palette_entry_format as _,
                width,
                height,
                border,
                palette_entry_format,
                palette_entry_type,
                decoded.as_ptr() as *const _,
            )
        } else {
            unimplemented!("CompressedTexImage2D internalformat: {:#x}", internalformat);
        }
    }
    unsafe fn CopyTexImage2D(
        &mut self,
        target: GLenum,
        level: GLint,
        internalformat: GLenum,
        x: GLint,
        y: GLint,
        width: GLsizei,
        height: GLsizei,
        border: GLint,
    ) {
        let _t = gl_trace("CopyTexImage2D");
        if debug_gl_enabled() {
            audit("CopyTexImage2D stale-before");
        }
        gles11::CopyTexImage2D(target, level, internalformat, x, y, width, height, border)
    }
    unsafe fn CopyTexSubImage2D(
        &mut self,
        target: GLenum,
        level: GLint,
        xoffset: GLint,
        yoffset: GLint,
        x: GLint,
        y: GLint,
        width: GLsizei,
        height: GLsizei,
    ) {
        let _t = gl_trace("CopyTexSubImage2D");
        if debug_gl_enabled() {
            audit("CopyTexSubImage2D stale-before");
        }
        gles11::CopyTexSubImage2D(target, level, xoffset, yoffset, x, y, width, height)
    }
    unsafe fn TexEnvf(&mut self, target: GLenum, pname: GLenum, param: GLfloat) {
        let _t = gl_trace("TexEnvf");
        self.es1_call();
        gles11::TexEnvf(target, pname, param)
    }
    unsafe fn TexEnvx(&mut self, target: GLenum, pname: GLenum, param: GLfixed) {
        let _t = gl_trace("TexEnvx");
        self.es1_call();
        gles11::TexEnvx(target, pname, param)
    }
    unsafe fn TexEnvi(&mut self, target: GLenum, pname: GLenum, param: GLint) {
        let _t = gl_trace("TexEnvi");
        self.es1_call();
        gles11::TexEnvi(target, pname, param)
    }
    unsafe fn TexEnvfv(&mut self, target: GLenum, pname: GLenum, params: *const GLfloat) {
        let _t = gl_trace("TexEnvfv");
        self.es1_call();
        if target == gles11::TEXTURE_FILTER_CONTROL_EXT {
            assert!(pname == gles11::TEXTURE_LOD_BIAS_EXT);
            unsafe {
                if !CStr::from_ptr(gles11::GetString(gles11::EXTENSIONS) as _)
                    .to_str()
                    .unwrap()
                    .contains("EXT_texture_lod_bias")
                {
                    log_dbg!("GL_EXT_texture_lod_bias is unsupported, skipping TexEnvfv({:#x}, {:#x}, ...) call", target, pname);
                    return;
                }
            };
        }
        gles11::TexEnvfv(target, pname, params)
    }
    unsafe fn TexEnvxv(&mut self, target: GLenum, pname: GLenum, params: *const GLfixed) {
        let _t = gl_trace("TexEnvxv");
        self.es1_call();
        gles11::TexEnvxv(target, pname, params)
    }
    unsafe fn TexEnviv(&mut self, target: GLenum, pname: GLenum, params: *const GLint) {
        let _t = gl_trace("TexEnviv");
        self.es1_call();
        gles11::TexEnviv(target, pname, params)
    }

    unsafe fn MultiTexCoord4f(
        &mut self,
        target: GLenum,
        s: GLfloat,
        t: GLfloat,
        r: GLfloat,
        q: GLfloat,
    ) {
        self.es1_call();
        gles11::MultiTexCoord4f(target, s, t, r, q)
    }
    unsafe fn MultiTexCoord4x(
        &mut self,
        target: GLenum,
        s: GLfixed,
        t: GLfixed,
        r: GLfixed,
        q: GLfixed,
    ) {
        self.es1_call();
        gles11::MultiTexCoord4x(target, s, t, r, q)
    }

    // Matrix stack operations
    unsafe fn MatrixMode(&mut self, mode: GLenum) {
        let _t = gl_trace("MatrixMode");
        self.es1_call();
        gles11::MatrixMode(mode)
    }
    unsafe fn LoadIdentity(&mut self) {
        self.es1_call();
        gles11::LoadIdentity()
    }
    unsafe fn LoadMatrixf(&mut self, m: *const GLfloat) {
        let _t = gl_trace("LoadMatrixf");
        self.es1_call();
        gles11::LoadMatrixf(m)
    }
    unsafe fn LoadMatrixx(&mut self, m: *const GLfixed) {
        self.es1_call();
        gles11::LoadMatrixx(m)
    }
    unsafe fn MultMatrixf(&mut self, m: *const GLfloat) {
        self.es1_call();
        gles11::MultMatrixf(m)
    }
    unsafe fn MultMatrixx(&mut self, m: *const GLfixed) {
        self.es1_call();
        gles11::MultMatrixx(m)
    }
    unsafe fn PushMatrix(&mut self) {
        self.es1_call();
        gles11::PushMatrix()
    }
    unsafe fn PopMatrix(&mut self) {
        self.es1_call();
        gles11::PopMatrix();
    }
    unsafe fn Orthof(
        &mut self,
        left: GLfloat,
        right: GLfloat,
        bottom: GLfloat,
        top: GLfloat,
        near: GLfloat,
        far: GLfloat,
    ) {
        self.es1_call();
        gles11::Orthof(left, right, bottom, top, near, far)
    }
    unsafe fn Orthox(
        &mut self,
        left: GLfixed,
        right: GLfixed,
        bottom: GLfixed,
        top: GLfixed,
        near: GLfixed,
        far: GLfixed,
    ) {
        self.es1_call();
        gles11::Orthox(left, right, bottom, top, near, far)
    }
    unsafe fn Frustumf(
        &mut self,
        left: GLfloat,
        right: GLfloat,
        bottom: GLfloat,
        top: GLfloat,
        near: GLfloat,
        far: GLfloat,
    ) {
        self.es1_call();
        gles11::Frustumf(left, right, bottom, top, near, far)
    }
    unsafe fn Frustumx(
        &mut self,
        left: GLfixed,
        right: GLfixed,
        bottom: GLfixed,
        top: GLfixed,
        near: GLfixed,
        far: GLfixed,
    ) {
        self.es1_call();
        gles11::Frustumx(left, right, bottom, top, near, far)
    }
    unsafe fn Rotatef(&mut self, angle: GLfloat, x: GLfloat, y: GLfloat, z: GLfloat) {
        self.es1_call();
        gles11::Rotatef(angle, x, y, z)
    }
    unsafe fn Rotatex(&mut self, angle: GLfixed, x: GLfixed, y: GLfixed, z: GLfixed) {
        self.es1_call();
        gles11::Rotatex(angle, x, y, z)
    }
    unsafe fn Scalef(&mut self, x: GLfloat, y: GLfloat, z: GLfloat) {
        self.es1_call();
        gles11::Scalef(x, y, z)
    }
    unsafe fn Scalex(&mut self, x: GLfixed, y: GLfixed, z: GLfixed) {
        self.es1_call();
        gles11::Scalex(x, y, z)
    }
    unsafe fn Translatef(&mut self, x: GLfloat, y: GLfloat, z: GLfloat) {
        self.es1_call();
        gles11::Translatef(x, y, z)
    }
    unsafe fn Translatex(&mut self, x: GLfixed, y: GLfixed, z: GLfixed) {
        self.es1_call();
        gles11::Translatex(x, y, z)
    }

    // OES_framebuffer_object: the `*OES` and plain spellings of these entry
    // points are the same function; see the fbo_entry_point! macro for the
    // client-version-based routing.
    fbo_query_entry_point!(GenFramebuffersOES, GenFramebuffers, (n: GLsizei, framebuffers: *mut GLuint));
    fbo_query_entry_point!(GenRenderbuffersOES, GenRenderbuffers, (n: GLsizei, renderbuffers: *mut GLuint));
    fbo_query_entry_point!(IsFramebufferOES, IsFramebuffer, (framebuffer: GLuint) -> GLboolean);
    fbo_query_entry_point!(IsRenderbufferOES, IsRenderbuffer, (renderbuffer: GLuint) -> GLboolean);
    fbo_entry_point!(BindFramebufferOES, BindFramebuffer, (target: GLenum, framebuffer: GLuint));
    fbo_entry_point!(BindRenderbufferOES, BindRenderbuffer, (target: GLenum, renderbuffer: GLuint));
    fbo_entry_point!(
        RenderbufferStorageOES,
        RenderbufferStorage,
        (target: GLenum, internalformat: GLenum, width: GLsizei, height: GLsizei)
    );
    fbo_entry_point!(
        FramebufferRenderbufferOES,
        FramebufferRenderbuffer,
        (target: GLenum, attachment: GLenum, renderbuffertarget: GLenum, renderbuffer: GLuint)
    );
    fbo_entry_point!(
        FramebufferTexture2DOES,
        FramebufferTexture2D,
        (target: GLenum, attachment: GLenum, textarget: GLenum, texture: GLuint, level: i32)
    );
    fbo_query_entry_point!(
        GetFramebufferAttachmentParameterivOES,
        GetFramebufferAttachmentParameteriv,
        (target: GLenum, attachment: GLenum, pname: GLenum, params: *mut GLint)
    );
    fbo_query_entry_point!(
        GetRenderbufferParameterivOES,
        GetRenderbufferParameteriv,
        (target: GLenum, pname: GLenum, params: *mut GLint)
    );
    fbo_query_entry_point!(CheckFramebufferStatusOES, CheckFramebufferStatus, (target: GLenum) -> GLenum);
    fbo_entry_point!(DeleteFramebuffersOES, DeleteFramebuffers, (n: GLsizei, framebuffers: *const GLuint));
    fbo_entry_point!(DeleteRenderbuffersOES, DeleteRenderbuffers, (n: GLsizei, renderbuffers: *const GLuint));
    fbo_query_entry_point!(GenerateMipmapOES, GenerateMipmap, (target: GLenum));

    unsafe fn GetBufferParameteriv(&mut self, target: GLenum, pname: GLenum, params: *mut GLint) {
        gles11::GetBufferParameteriv(target, pname, params)
    }
    unsafe fn MapBufferOES(&mut self, target: GLenum, access: GLenum) -> *mut GLvoid {
        gles11::MapBufferOES(target, access)
    }
    unsafe fn UnmapBufferOES(&mut self, target: GLenum) -> GLboolean {
        gles11::UnmapBufferOES(target)
    }

    // OpenGL ES 2.0 entry points, forwarded to ANGLE. These are used by
    // guest ES 2.0 contexts, so route the blend equation spellings by client
    // version like the FBO family above.
    unsafe fn CreateShader(&mut self, type_: GLenum) -> GLuint {
        let _t = gl_trace("CreateShader");
        self.es2_call();
        gles2::CreateShader(type_)
    }
    unsafe fn DeleteShader(&mut self, shader: GLuint) {
        let _t = gl_trace("DeleteShader");
        self.es2_call();
        gles2::DeleteShader(shader)
    }
    unsafe fn ShaderSource(
        &mut self,
        shader: GLuint,
        count: GLsizei,
        string: *const *const GLchar,
        length: *const GLint,
    ) {
        let _t = gl_trace("ShaderSource");
        self.es2_call();
        gles2::ShaderSource(shader, count, string, length)
    }
    unsafe fn CompileShader(&mut self, shader: GLuint) {
        let _t = gl_trace("CompileShader");
        self.es2_call();
        gles2::CompileShader(shader)
    }
    unsafe fn GetShaderiv(&mut self, shader: GLuint, pname: GLenum, params: *mut GLint) {
        let _t = gl_trace("GetShaderiv");
        self.es2_call();
        gles2::GetShaderiv(shader, pname, params)
    }
    unsafe fn GetShaderInfoLog(
        &mut self,
        shader: GLuint,
        maxLength: GLsizei,
        length: *mut GLsizei,
        infoLog: *mut GLchar,
    ) {
        let _t = gl_trace("GetShaderInfoLog");
        self.es2_call();
        gles2::GetShaderInfoLog(shader, maxLength, length, infoLog)
    }
    unsafe fn IsShader(&mut self, shader: GLuint) -> GLboolean {
        let _t = gl_trace("IsShader");
        self.es2_call();
        gles2::IsShader(shader)
    }
    unsafe fn CreateProgram(&mut self) -> GLuint {
        let _t = gl_trace("CreateProgram");
        self.es2_call();
        gles2::CreateProgram()
    }
    unsafe fn DeleteProgram(&mut self, program: GLuint) {
        let _t = gl_trace("DeleteProgram");
        self.es2_call();
        gles2::DeleteProgram(program)
    }
    unsafe fn AttachShader(&mut self, program: GLuint, shader: GLuint) {
        let _t = gl_trace("AttachShader");
        self.es2_call();
        gles2::AttachShader(program, shader)
    }
    unsafe fn DetachShader(&mut self, program: GLuint, shader: GLuint) {
        let _t = gl_trace("DetachShader");
        self.es2_call();
        gles2::DetachShader(program, shader)
    }
    unsafe fn LinkProgram(&mut self, program: GLuint) {
        let _t = gl_trace("LinkProgram");
        self.es2_call();
        gles2::LinkProgram(program)
    }
    unsafe fn UseProgram(&mut self, program: GLuint) {
        let _t = gl_trace("UseProgram");
        self.ctx.current_program = program;
        self.es2_call();
        gles2::UseProgram(program)
    }
    unsafe fn GetProgramiv(&mut self, program: GLuint, pname: GLenum, params: *mut GLint) {
        let _t = gl_trace("GetProgramiv");
        self.es2_call();
        gles2::GetProgramiv(program, pname, params)
    }
    unsafe fn GetProgramInfoLog(
        &mut self,
        program: GLuint,
        maxLength: GLsizei,
        length: *mut GLsizei,
        infoLog: *mut GLchar,
    ) {
        let _t = gl_trace("GetProgramInfoLog");
        self.es2_call();
        gles2::GetProgramInfoLog(program, maxLength, length, infoLog)
    }
    unsafe fn IsProgram(&mut self, program: GLuint) -> GLboolean {
        let _t = gl_trace("IsProgram");
        self.es2_call();
        gles2::IsProgram(program)
    }
    unsafe fn ValidateProgram(&mut self, program: GLuint) {
        self.es2_call();
        gles2::ValidateProgram(program)
    }
    unsafe fn BindAttribLocation(&mut self, program: GLuint, index: GLuint, name: *const GLchar) {
        let _t = gl_trace("BindAttribLocation");
        self.es2_call();
        gles2::BindAttribLocation(program, index, name)
    }
    unsafe fn GetAttribLocation(&mut self, program: GLuint, name: *const GLchar) -> GLint {
        let _t = gl_trace("GetAttribLocation");
        self.es2_call();
        gles2::GetAttribLocation(program, name)
    }
    unsafe fn GetUniformLocation(&mut self, program: GLuint, name: *const GLchar) -> GLint {
        let _t = gl_trace("GetUniformLocation");
        self.es2_call();
        gles2::GetUniformLocation(program, name)
    }
    unsafe fn GetActiveAttrib(
        &mut self,
        program: GLuint,
        index: GLuint,
        bufSize: GLsizei,
        length: *mut GLsizei,
        size: *mut GLint,
        type_: *mut GLenum,
        name: *mut GLchar,
    ) {
        self.es2_call();
        gles2::GetActiveAttrib(program, index, bufSize, length, size, type_, name)
    }
    unsafe fn GetActiveUniform(
        &mut self,
        program: GLuint,
        index: GLuint,
        bufSize: GLsizei,
        length: *mut GLsizei,
        size: *mut GLint,
        type_: *mut GLenum,
        name: *mut GLchar,
    ) {
        self.es2_call();
        gles2::GetActiveUniform(program, index, bufSize, length, size, type_, name)
    }
    unsafe fn EnableVertexAttribArray(&mut self, index: GLuint) {
        self.es2_call();
        gles2::EnableVertexAttribArray(index)
    }
    unsafe fn DisableVertexAttribArray(&mut self, index: GLuint) {
        self.es2_call();
        gles2::DisableVertexAttribArray(index)
    }
    unsafe fn VertexAttribPointer(
        &mut self,
        index: GLuint,
        size: GLint,
        type_: GLenum,
        normalized: GLboolean,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        let _t = gl_trace("VertexAttribPointer");
        self.es2_call();
        gles2::VertexAttribPointer(index, size, type_, normalized, stride, pointer)
    }
    unsafe fn VertexAttrib1f(&mut self, index: GLuint, x: GLfloat) {
        self.es2_call();
        gles2::VertexAttrib1f(index, x)
    }
    unsafe fn VertexAttrib2f(&mut self, index: GLuint, x: GLfloat, y: GLfloat) {
        self.es2_call();
        gles2::VertexAttrib2f(index, x, y)
    }
    unsafe fn VertexAttrib3f(&mut self, index: GLuint, x: GLfloat, y: GLfloat, z: GLfloat) {
        self.es2_call();
        gles2::VertexAttrib3f(index, x, y, z)
    }
    unsafe fn VertexAttrib4f(
        &mut self,
        index: GLuint,
        x: GLfloat,
        y: GLfloat,
        z: GLfloat,
        w: GLfloat,
    ) {
        self.es2_call();
        gles2::VertexAttrib4f(index, x, y, z, w)
    }
    unsafe fn VertexAttrib1fv(&mut self, index: GLuint, v: *const GLfloat) {
        self.es2_call();
        gles2::VertexAttrib1fv(index, v)
    }
    unsafe fn VertexAttrib2fv(&mut self, index: GLuint, v: *const GLfloat) {
        self.es2_call();
        gles2::VertexAttrib2fv(index, v)
    }
    unsafe fn VertexAttrib3fv(&mut self, index: GLuint, v: *const GLfloat) {
        self.es2_call();
        gles2::VertexAttrib3fv(index, v)
    }
    unsafe fn VertexAttrib4fv(&mut self, index: GLuint, v: *const GLfloat) {
        self.es2_call();
        gles2::VertexAttrib4fv(index, v)
    }
    unsafe fn Uniform1f(&mut self, location: GLint, v0: GLfloat) {
        self.es2_call();
        gles2::Uniform1f(location, v0)
    }
    unsafe fn Uniform2f(&mut self, location: GLint, v0: GLfloat, v1: GLfloat) {
        self.es2_call();
        gles2::Uniform2f(location, v0, v1)
    }
    unsafe fn Uniform3f(&mut self, location: GLint, v0: GLfloat, v1: GLfloat, v2: GLfloat) {
        self.es2_call();
        gles2::Uniform3f(location, v0, v1, v2)
    }
    unsafe fn Uniform4f(
        &mut self,
        location: GLint,
        v0: GLfloat,
        v1: GLfloat,
        v2: GLfloat,
        v3: GLfloat,
    ) {
        self.es2_call();
        gles2::Uniform4f(location, v0, v1, v2, v3)
    }
    unsafe fn Uniform1i(&mut self, location: GLint, v0: GLint) {
        let _t = gl_trace("Uniform1i");
        self.es2_call();
        gles2::Uniform1i(location, v0)
    }
    unsafe fn Uniform2i(&mut self, location: GLint, v0: GLint, v1: GLint) {
        self.es2_call();
        gles2::Uniform2i(location, v0, v1)
    }
    unsafe fn Uniform3i(&mut self, location: GLint, v0: GLint, v1: GLint, v2: GLint) {
        self.es2_call();
        gles2::Uniform3i(location, v0, v1, v2)
    }
    unsafe fn Uniform4i(&mut self, location: GLint, v0: GLint, v1: GLint, v2: GLint, v3: GLint) {
        self.es2_call();
        gles2::Uniform4i(location, v0, v1, v2, v3)
    }
    unsafe fn Uniform1fv(&mut self, location: GLint, count: GLsizei, value: *const GLfloat) {
        self.es2_call();
        gles2::Uniform1fv(location, count, value)
    }
    unsafe fn Uniform2fv(&mut self, location: GLint, count: GLsizei, value: *const GLfloat) {
        self.es2_call();
        gles2::Uniform2fv(location, count, value)
    }
    unsafe fn Uniform3fv(&mut self, location: GLint, count: GLsizei, value: *const GLfloat) {
        self.es2_call();
        gles2::Uniform3fv(location, count, value)
    }
    unsafe fn Uniform4fv(&mut self, location: GLint, count: GLsizei, value: *const GLfloat) {
        let _t = gl_trace("Uniform4fv");
        self.es2_call();
        gles2::Uniform4fv(location, count, value)
    }
    unsafe fn Uniform1iv(&mut self, location: GLint, count: GLsizei, value: *const GLint) {
        self.es2_call();
        gles2::Uniform1iv(location, count, value)
    }
    unsafe fn Uniform2iv(&mut self, location: GLint, count: GLsizei, value: *const GLint) {
        self.es2_call();
        gles2::Uniform2iv(location, count, value)
    }
    unsafe fn Uniform3iv(&mut self, location: GLint, count: GLsizei, value: *const GLint) {
        self.es2_call();
        gles2::Uniform3iv(location, count, value)
    }
    unsafe fn Uniform4iv(&mut self, location: GLint, count: GLsizei, value: *const GLint) {
        self.es2_call();
        gles2::Uniform4iv(location, count, value)
    }
    unsafe fn UniformMatrix2fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        self.es2_call();
        gles2::UniformMatrix2fv(location, count, transpose, value)
    }
    unsafe fn UniformMatrix3fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        self.es2_call();
        gles2::UniformMatrix3fv(location, count, transpose, value)
    }
    unsafe fn UniformMatrix4fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        let _t = gl_trace("UniformMatrix4fv");
        self.es2_call();
        gles2::UniformMatrix4fv(location, count, transpose, value)
    }
    unsafe fn BlendColor(&mut self, r: GLclampf, g: GLclampf, b: GLclampf, a: GLclampf) {
        gles2::BlendColor(r, g, b, a)
    }
    unsafe fn BlendEquation(&mut self, mode: GLenum) {
        if self.ctx.client_version >= 2 && !self.ctx.using_es1 {
            gles2::BlendEquation(mode)
        } else {
            gles11::BlendEquationOES(mode)
        }
    }
    unsafe fn BlendEquationSeparate(&mut self, modeRGB: GLenum, modeAlpha: GLenum) {
        gles2::BlendEquationSeparate(modeRGB, modeAlpha)
    }
    unsafe fn BlendFuncSeparate(
        &mut self,
        srcRGB: GLenum,
        dstRGB: GLenum,
        srcAlpha: GLenum,
        dstAlpha: GLenum,
    ) {
        gles2::BlendFuncSeparate(srcRGB, dstRGB, srcAlpha, dstAlpha)
    }
    unsafe fn StencilFuncSeparate(
        &mut self,
        face: GLenum,
        func: GLenum,
        ref_: GLint,
        mask: GLuint,
    ) {
        gles2::StencilFuncSeparate(face, func, ref_, mask)
    }
    unsafe fn StencilOpSeparate(
        &mut self,
        face: GLenum,
        sfail: GLenum,
        dpfail: GLenum,
        dppass: GLenum,
    ) {
        gles2::StencilOpSeparate(face, sfail, dpfail, dppass)
    }
    unsafe fn StencilMaskSeparate(&mut self, face: GLenum, mask: GLuint) {
        gles2::StencilMaskSeparate(face, mask)
    }
    unsafe fn GetVertexAttribiv(&mut self, index: GLuint, pname: GLenum, params: *mut GLint) {
        self.es2_call();
        gles2::GetVertexAttribiv(index, pname, params)
    }
    unsafe fn GetVertexAttribfv(&mut self, index: GLuint, pname: GLenum, params: *mut GLfloat) {
        self.es2_call();
        gles2::GetVertexAttribfv(index, pname, params)
    }
    unsafe fn GetVertexAttribPointerv(
        &mut self,
        index: GLuint,
        pname: GLenum,
        pointer: *mut *mut GLvoid,
    ) {
        self.es2_call();
        // Cast works around gl-rs generating `*mut *const GLvoid` here
        // (see also GetPointerv above).
        gles2::GetVertexAttribPointerv(index, pname, pointer as *const *mut std::ffi::c_void)
    }
    unsafe fn GetUniformiv(&mut self, program: GLuint, location: GLint, params: *mut GLint) {
        self.es2_call();
        gles2::GetUniformiv(program, location, params)
    }
    unsafe fn GetUniformfv(&mut self, program: GLuint, location: GLint, params: *mut GLfloat) {
        self.es2_call();
        gles2::GetUniformfv(program, location, params)
    }
    unsafe fn GetAttachedShaders(
        &mut self,
        program: GLuint,
        maxCount: GLsizei,
        count: *mut GLsizei,
        shaders: *mut GLuint,
    ) {
        self.es2_call();
        gles2::GetAttachedShaders(program, maxCount, count, shaders)
    }
    unsafe fn GetShaderSource(
        &mut self,
        shader: GLuint,
        bufSize: GLsizei,
        length: *mut GLsizei,
        source: *mut GLchar,
    ) {
        self.es2_call();
        gles2::GetShaderSource(shader, bufSize, length, source)
    }
    unsafe fn GetShaderPrecisionFormat(
        &mut self,
        shadertype: GLenum,
        precisiontype: GLenum,
        range: *mut GLint,
        precision: *mut GLint,
    ) {
        self.es2_call();
        gles2::GetShaderPrecisionFormat(shadertype, precisiontype, range, precision)
    }
    unsafe fn ShaderBinary(
        &mut self,
        count: GLsizei,
        shaders: *const GLuint,
        binaryformat: GLenum,
        binary: *const GLvoid,
        length: GLsizei,
    ) {
        self.es2_call();
        // ANGLE does not support shader binaries and will report a GL error,
        // which is better behavior than panicking here.
        gles2::ShaderBinary(count, shaders, binaryformat, binary, length)
    }
}
