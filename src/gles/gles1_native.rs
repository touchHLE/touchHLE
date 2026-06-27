/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Passthrough for a native OpenGL ES 1.1 driver.

use super::gles11_raw as gles11;
use super::gles11_raw::types::*;
use super::gles_generic::GLES;
use super::util::{try_decode_pvrtc, PalettedTextureFormat};
use super::GLESContext;
use crate::window::{GLContext, GLVersion, Window};
use std::ffi::CStr;
use std::marker::PhantomData;

pub struct GLES1NativeContext {
    gl_ctx: GLContext,
    is_loaded: bool,
    /// Whether the underlying OpenGL ES 1.1 driver advertises
    /// `GL_IMG_texture_compression_pvrtc`. Apps shipped for iPhone OS use
    /// PVRTC textures pervasively (Apple's recommended compression format on
    /// PowerVR-based devices), so when the host driver lacks PVRTC we must
    /// software-decode the payload and upload it as plain RGBA — otherwise
    /// every PVRTC texture silently fails with `GL_INVALID_ENUM` and the
    /// app renders as black silhouettes (see Temple Run 1.0 on
    /// ARM Mali / Qualcomm Adreno, neither of which advertises this
    /// extension on their ES 1.1 surface).
    pvrtc_native: bool,
    /// Whether `pvrtc_native` has been populated yet. The check is deferred
    /// to the first `make_current` because we need a current GL context to
    /// query `GL_EXTENSIONS`.
    pvrtc_native_checked: bool,
}

impl GLESContext for GLES1NativeContext {
    fn description() -> &'static str {
        "Native OpenGL ES 1.1"
    }

    fn new(window: &mut Window) -> Result<Self, String> {
        Ok(Self {
            gl_ctx: window.create_gl_context(GLVersion::GLES11)?,
            is_loaded: false,
            pvrtc_native: false,
            pvrtc_native_checked: false,
        })
    }

    fn make_current<'gl_ctx, 'win: 'gl_ctx>(
        &'gl_ctx mut self,
        window: &'win mut Window,
    ) -> Box<dyn GLES + 'gl_ctx> {
        if self.gl_ctx.is_current() && self.is_loaded {
            return Box::new(GLES1Native {
                _gl_lifetime: PhantomData,
                pending_synthetic_error: std::cell::Cell::new(gles11::NO_ERROR),
                pvrtc_native: self.pvrtc_native,
            });
        }

        unsafe {
            window.make_gl_context_current(&self.gl_ctx);
        }
        gles11::load_with(|s| window.gl_get_proc_address(s));
        self.is_loaded = true;
        if !self.pvrtc_native_checked {
            self.pvrtc_native = unsafe { detect_pvrtc_support() };
            self.pvrtc_native_checked = true;
            log!(
                "GLES1Native: GL_IMG_texture_compression_pvrtc {} (PVRTC textures will \
                 be {} on this driver)",
                if self.pvrtc_native {
                    "advertised by host driver"
                } else {
                    "NOT advertised by host driver"
                },
                if self.pvrtc_native {
                    "uploaded directly"
                } else {
                    "software-decoded to RGBA before upload"
                },
            );
        }
        Box::new(GLES1Native {
            _gl_lifetime: PhantomData,
            pending_synthetic_error: std::cell::Cell::new(gles11::NO_ERROR),
            pvrtc_native: self.pvrtc_native,
        })
    }

    unsafe fn make_current_unchecked_for_window<'gl_ctx>(
        &'gl_ctx mut self,
        make_current_fn: &mut dyn FnMut(&GLContext),
        loader_fn: &mut dyn FnMut(&'static str) -> *const std::ffi::c_void,
    ) -> Box<dyn GLES + 'gl_ctx> {
        if self.gl_ctx.is_current() && self.is_loaded {
            return Box::new(GLES1Native {
                _gl_lifetime: PhantomData,
                pending_synthetic_error: std::cell::Cell::new(gles11::NO_ERROR),
                pvrtc_native: self.pvrtc_native,
            });
        }

        make_current_fn(&self.gl_ctx);
        gles11::load_with(loader_fn);
        self.is_loaded = true;
        if !self.pvrtc_native_checked {
            self.pvrtc_native = detect_pvrtc_support();
            self.pvrtc_native_checked = true;
        }
        Box::new(GLES1Native {
            _gl_lifetime: PhantomData,
            pending_synthetic_error: std::cell::Cell::new(gles11::NO_ERROR),
            pvrtc_native: self.pvrtc_native,
        })
    }
}

/// Query `GL_EXTENSIONS` on the currently-bound OpenGL ES 1.1 context and
/// return whether it advertises `GL_IMG_texture_compression_pvrtc`.
///
/// Must be called with a current GL context. Returns `false` on any
/// driver-reported error (NULL string, non-UTF-8 string, missing token);
/// software-decoding PVRTC is the safe-default behaviour.
unsafe fn detect_pvrtc_support() -> bool {
    let raw = gles11::GetString(gles11::EXTENSIONS);
    if raw.is_null() {
        return false;
    }
    let Ok(s) = CStr::from_ptr(raw as *const _).to_str() else {
        return false;
    };
    if s.is_empty() {
        return false;
    }
    s.split(' ')
        .any(|ext| ext == "GL_IMG_texture_compression_pvrtc")
}

pub struct GLES1Native<'gl_ctx> {
    _gl_lifetime: PhantomData<&'gl_ctx ()>,
    /// Synthetic error queue for OpenGL ES 2.0 entry points that are not
    /// supported on a native ES 1.1 driver. When a guest app calls a
    /// shader-pipeline entry point (e.g. `glCreateShader`, `glUseProgram`,
    /// `glUniform1f`) on an ES 1.1 context, the ES 2.0 specification says the
    /// implementation must report `GL_INVALID_OPERATION`. The native ES 1.1
    /// driver wouldn't know about these calls at all — it never sees them —
    /// so [`GLES1Native::GetError`] would return `GL_NO_ERROR` and the guest
    /// would silently miss the failure. Instead we maintain a one-slot
    /// pending-error queue here; the ES 2.0 overrides below store
    /// `GL_INVALID_OPERATION` into it, and our `GetError` returns it (then
    /// clears it) before falling back to the underlying driver's error queue.
    pending_synthetic_error: std::cell::Cell<GLenum>,
    /// Mirror of [`GLES1NativeContext::pvrtc_native`]; copied at make_current
    /// time so the per-call `CompressedTexImage2D` path doesn't have to
    /// re-query `GL_EXTENSIONS`.
    pvrtc_native: bool,
}

impl GLES for GLES1Native<'_> {
    fn is_native_es1(&self) -> bool {
        true
    }
    unsafe fn driver_description(&self) -> String {
        let version = CStr::from_ptr(gles11::GetString(gles11::VERSION) as *const _);
        let vendor = CStr::from_ptr(gles11::GetString(gles11::VENDOR) as *const _);
        let renderer = CStr::from_ptr(gles11::GetString(gles11::RENDERER) as *const _);
        format!(
            "{} / {} / {}",
            version.to_string_lossy(),
            vendor.to_string_lossy(),
            renderer.to_string_lossy()
        )
    }

    // Generic state manipulation
    unsafe fn GetError(&mut self) -> GLenum {
        // If a shader-pipeline ES 2.0 entry point was invoked on this
        // ES 1.1-only backend, report the synthetic `GL_INVALID_OPERATION`
        // before consulting the real driver's queue.
        let synthetic = self.pending_synthetic_error.replace(gles11::NO_ERROR);
        if synthetic != gles11::NO_ERROR {
            return synthetic;
        }
        gles11::GetError()
    }
    unsafe fn Enable(&mut self, cap: GLenum) {
        gles11::Enable(cap)
    }
    unsafe fn IsEnabled(&mut self, cap: GLenum) -> GLboolean {
        gles11::IsEnabled(cap)
    }
    unsafe fn Disable(&mut self, cap: GLenum) {
        gles11::Disable(cap)
    }
    unsafe fn ClientActiveTexture(&mut self, texture: GLenum) {
        gles11::ClientActiveTexture(texture);
    }
    unsafe fn EnableClientState(&mut self, array: GLenum) {
        gles11::EnableClientState(array)
    }
    unsafe fn DisableClientState(&mut self, array: GLenum) {
        gles11::DisableClientState(array)
    }
    unsafe fn GetBooleanv(&mut self, pname: GLenum, params: *mut GLboolean) {
        gles11::GetBooleanv(pname, params)
    }
    unsafe fn GetFloatv(&mut self, pname: GLenum, params: *mut GLfloat) {
        gles11::GetFloatv(pname, params)
    }
    unsafe fn GetIntegerv(&mut self, pname: GLenum, params: *mut GLint) {
        gles11::GetIntegerv(pname, params)
    }
    unsafe fn GetFixedv(&mut self, pname: GLenum, params: *mut GLfixed) {
        gles11::GetFixedv(pname, params)
    }
    unsafe fn GetTexEnviv(&mut self, target: GLenum, pname: GLenum, params: *mut GLint) {
        gles11::GetTexEnviv(target, pname, params)
    }
    unsafe fn GetTexEnvfv(&mut self, target: GLenum, pname: GLenum, params: *mut GLfloat) {
        gles11::GetTexEnvfv(target, pname, params)
    }
    unsafe fn GetTexEnvxv(&mut self, target: GLenum, pname: GLenum, params: *mut GLfixed) {
        gles11::GetTexEnvxv(target, pname, params)
    }
    unsafe fn GetTexParameteriv(&mut self, target: GLenum, pname: GLenum, params: *mut GLint) {
        gles11::GetTexParameteriv(target, pname, params)
    }
    unsafe fn GetTexParameterfv(&mut self, target: GLenum, pname: GLenum, params: *mut GLfloat) {
        gles11::GetTexParameterfv(target, pname, params)
    }
    unsafe fn GetTexParameterxv(&mut self, target: GLenum, pname: GLenum, params: *mut GLfixed) {
        gles11::GetTexParameterxv(target, pname, params)
    }
    unsafe fn GetClipPlanef(&mut self, plane: GLenum, equation: *mut GLfloat) {
        gles11::GetClipPlanef(plane, equation)
    }
    unsafe fn GetClipPlanex(&mut self, plane: GLenum, equation: *mut GLfixed) {
        gles11::GetClipPlanex(plane, equation)
    }
    unsafe fn GetLightfv(&mut self, light: GLenum, pname: GLenum, params: *mut GLfloat) {
        gles11::GetLightfv(light, pname, params)
    }
    unsafe fn GetLightxv(&mut self, light: GLenum, pname: GLenum, params: *mut GLfixed) {
        gles11::GetLightxv(light, pname, params)
    }
    unsafe fn GetMaterialfv(&mut self, face: GLenum, pname: GLenum, params: *mut GLfloat) {
        gles11::GetMaterialfv(face, pname, params)
    }
    unsafe fn GetMaterialxv(&mut self, face: GLenum, pname: GLenum, params: *mut GLfixed) {
        gles11::GetMaterialxv(face, pname, params)
    }
    unsafe fn GetPointerv(&mut self, pname: GLenum, params: *mut *const GLvoid) {
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

    // MALI HACK: Прячем сломанные расширения
    unsafe fn GetString(&mut self, name: GLenum) -> *const GLubyte {
        if name == gles11::EXTENSIONS {
            static mut FILTERED_EXTS: *mut std::os::raw::c_char = std::ptr::null_mut();
            if FILTERED_EXTS.is_null() {
                let orig_ptr = gles11::GetString(name);
                if !orig_ptr.is_null() {
                    let orig_str = std::ffi::CStr::from_ptr(orig_ptr as *const _).to_string_lossy();
                    // Вырезаем OES_matrix_palette чтобы заставить AC2
                    // использовать CPU анимацию
                    let filtered = orig_str.replace("GL_OES_matrix_palette", "");
                    let c_str = std::ffi::CString::new(filtered).unwrap();
                    FILTERED_EXTS = c_str.into_raw();
                } else {
                    return std::ptr::null();
                }
            }
            return FILTERED_EXTS as *const GLubyte;
        }
        gles11::GetString(name)
    }

    // Other state manipulation
    unsafe fn AlphaFunc(&mut self, func: GLenum, ref_: GLclampf) {
        gles11::AlphaFunc(func, ref_)
    }
    unsafe fn AlphaFuncx(&mut self, func: GLenum, ref_: GLclampx) {
        gles11::AlphaFuncx(func, ref_)
    }
    unsafe fn BlendFunc(&mut self, sfactor: GLenum, dfactor: GLenum) {
        gles11::BlendFunc(sfactor, dfactor)
    }
    unsafe fn BlendEquationOES(&mut self, mode: GLenum) {
        gles11::BlendEquationOES(mode);
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
        gles11::ClipPlanef(plane, equation)
    }
    unsafe fn ClipPlanex(&mut self, plane: GLenum, equation: *const GLfixed) {
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
        gles11::LogicOp(opcode);
    }

    // Points
    unsafe fn PointSize(&mut self, size: GLfloat) {
        gles11::PointSize(size)
    }
    unsafe fn PointSizex(&mut self, size: GLfixed) {
        gles11::PointSizex(size)
    }
    unsafe fn PointParameterf(&mut self, pname: GLenum, param: GLfloat) {
        gles11::PointParameterf(pname, param)
    }
    unsafe fn PointParameterx(&mut self, pname: GLenum, param: GLfixed) {
        gles11::PointParameterx(pname, param)
    }
    unsafe fn PointParameterfv(&mut self, pname: GLenum, params: *const GLfloat) {
        gles11::PointParameterfv(pname, params)
    }
    unsafe fn PointParameterxv(&mut self, pname: GLenum, params: *const GLfixed) {
        gles11::PointParameterxv(pname, params)
    }

    // Lighting and materials
    unsafe fn Fogf(&mut self, pname: GLenum, param: GLfloat) {
        gles11::Fogf(pname, param)
    }
    unsafe fn Fogx(&mut self, pname: GLenum, param: GLfixed) {
        gles11::Fogx(pname, param)
    }
    unsafe fn Fogfv(&mut self, pname: GLenum, params: *const GLfloat) {
        gles11::Fogfv(pname, params)
    }
    unsafe fn Fogxv(&mut self, pname: GLenum, params: *const GLfixed) {
        gles11::Fogxv(pname, params)
    }
    unsafe fn Lightf(&mut self, light: GLenum, pname: GLenum, param: GLfloat) {
        gles11::Lightf(light, pname, param)
    }
    unsafe fn Lightx(&mut self, light: GLenum, pname: GLenum, param: GLfixed) {
        gles11::Lightx(light, pname, param)
    }
    unsafe fn Lightfv(&mut self, light: GLenum, pname: GLenum, params: *const GLfloat) {
        gles11::Lightfv(light, pname, params)
    }
    unsafe fn Lightxv(&mut self, light: GLenum, pname: GLenum, params: *const GLfixed) {
        gles11::Lightxv(light, pname, params)
    }
    unsafe fn LightModelf(&mut self, pname: GLenum, param: GLfloat) {
        gles11::LightModelf(pname, param)
    }
    unsafe fn LightModelx(&mut self, pname: GLenum, param: GLfixed) {
        gles11::LightModelx(pname, param)
    }
    unsafe fn LightModelfv(&mut self, pname: GLenum, params: *const GLfloat) {
        gles11::LightModelfv(pname, params)
    }
    unsafe fn LightModelxv(&mut self, pname: GLenum, params: *const GLfixed) {
        gles11::LightModelxv(pname, params)
    }
    unsafe fn Materialf(&mut self, face: GLenum, pname: GLenum, param: GLfloat) {
        gles11::Materialf(face, pname, param)
    }
    unsafe fn Materialx(&mut self, face: GLenum, pname: GLenum, param: GLfixed) {
        gles11::Materialx(face, pname, param)
    }
    unsafe fn Materialfv(&mut self, face: GLenum, pname: GLenum, params: *const GLfloat) {
        gles11::Materialfv(face, pname, params)
    }
    unsafe fn Materialxv(&mut self, face: GLenum, pname: GLenum, params: *const GLfixed) {
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
        gles11::BindBuffer(target, buffer)
    }
    unsafe fn BufferData(
        &mut self,
        target: GLenum,
        size: GLsizeiptr,
        data: *const GLvoid,
        usage: GLenum,
    ) {
        gles11::BufferData(target, size, data, usage)
    }
    unsafe fn BufferSubData(
        &mut self,
        target: GLenum,
        offset: GLintptr,
        size: GLsizeiptr,
        data: *const GLvoid,
    ) {
        gles11::BufferSubData(target, offset, size, data)
    }

    // Non-pointers
    unsafe fn Color4f(&mut self, red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat) {
        gles11::Color4f(red, green, blue, alpha)
    }
    unsafe fn Color4x(&mut self, red: GLfixed, green: GLfixed, blue: GLfixed, alpha: GLfixed) {
        gles11::Color4x(red, green, blue, alpha)
    }
    unsafe fn Color4ub(&mut self, red: GLubyte, green: GLubyte, blue: GLubyte, alpha: GLubyte) {
        gles11::Color4ub(red, green, blue, alpha)
    }
    unsafe fn Normal3f(&mut self, nx: GLfloat, ny: GLfloat, nz: GLfloat) {
        gles11::Normal3f(nx, ny, nz)
    }
    unsafe fn Normal3x(&mut self, nx: GLfixed, ny: GLfixed, nz: GLfixed) {
        gles11::Normal3x(nx, ny, nz)
    }

    // Pointers - Возвращаем твою изначальную правильную защиту от крашей Mali
    unsafe fn ColorPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        if pointer.is_null() {
            let mut bound_buffer: GLint = 0;
            gles11::GetIntegerv(gles11::ARRAY_BUFFER_BINDING, &mut bound_buffer);
            if bound_buffer == 0 {
                gles11::DisableClientState(gles11::COLOR_ARRAY);
                return;
            }
        }
        gles11::ColorPointer(size, type_, stride, pointer)
    }

    unsafe fn NormalPointer(&mut self, type_: GLenum, stride: GLsizei, pointer: *const GLvoid) {
        if pointer.is_null() {
            let mut bound_buffer: GLint = 0;
            gles11::GetIntegerv(gles11::ARRAY_BUFFER_BINDING, &mut bound_buffer);
            if bound_buffer == 0 {
                gles11::DisableClientState(gles11::NORMAL_ARRAY);
                return;
            }
        }
        gles11::NormalPointer(type_, stride, pointer)
    }

    unsafe fn TexCoordPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        if pointer.is_null() {
            let mut bound_buffer: GLint = 0;
            gles11::GetIntegerv(gles11::ARRAY_BUFFER_BINDING, &mut bound_buffer);
            if bound_buffer == 0 {
                gles11::DisableClientState(gles11::TEXTURE_COORD_ARRAY);
                return;
            }
        }
        gles11::TexCoordPointer(size, type_, stride, pointer)
    }

    unsafe fn VertexPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        if pointer.is_null() {
            let mut bound_buffer: GLint = 0;
            gles11::GetIntegerv(gles11::ARRAY_BUFFER_BINDING, &mut bound_buffer);
            if bound_buffer == 0 {
                gles11::DisableClientState(gles11::VERTEX_ARRAY);
                return;
            }
        }
        gles11::VertexPointer(size, type_, stride, pointer)
    }

    unsafe fn PointSizePointerOES(
        &mut self,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        gles11::PointSizePointerOES(type_, stride, pointer)
    }

    // Drawing
    unsafe fn DrawArrays(&mut self, mode: GLenum, first: GLint, count: GLsizei) {
        gles11::DrawArrays(mode, first, count)
    }

    unsafe fn DrawElements(
        &mut self,
        mode: GLenum,
        count: GLsizei,
        type_: GLenum,
        indices: *const GLvoid,
    ) {
        if indices.is_null() {
            let mut bound_buffer: GLint = 0;
            gles11::GetIntegerv(gles11::ELEMENT_ARRAY_BUFFER_BINDING, &mut bound_buffer);
            if bound_buffer == 0 {
                return;
            }
        }
        gles11::DrawElements(mode, count, type_, indices)
    }

    // GL_OES_draw_texture
    unsafe fn DrawTexsOES(&mut self, x: i16, y: i16, z: i16, width: i16, height: i16) {
        gles11::DrawTexsOES(x, y, z, width, height)
    }
    unsafe fn DrawTexiOES(&mut self, x: GLint, y: GLint, z: GLint, width: GLint, height: GLint) {
        gles11::DrawTexiOES(x, y, z, width, height)
    }
    unsafe fn DrawTexxOES(
        &mut self,
        x: GLfixed,
        y: GLfixed,
        z: GLfixed,
        width: GLfixed,
        height: GLfixed,
    ) {
        gles11::DrawTexxOES(x, y, z, width, height)
    }
    unsafe fn DrawTexfOES(
        &mut self,
        x: GLfloat,
        y: GLfloat,
        z: GLfloat,
        width: GLfloat,
        height: GLfloat,
    ) {
        gles11::DrawTexfOES(x, y, z, width, height)
    }
    unsafe fn DrawTexsvOES(&mut self, coords: *const i16) {
        gles11::DrawTexsvOES(coords)
    }
    unsafe fn DrawTexivOES(&mut self, coords: *const GLint) {
        gles11::DrawTexivOES(coords)
    }
    unsafe fn DrawTexxvOES(&mut self, coords: *const GLfixed) {
        gles11::DrawTexxvOES(coords)
    }
    unsafe fn DrawTexfvOES(&mut self, coords: *const GLfloat) {
        gles11::DrawTexfvOES(coords)
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
        gles11::ClearColorx(red, green, blue, alpha)
    }
    unsafe fn ClearDepthf(&mut self, depth: GLclampf) {
        gles11::ClearDepthf(depth)
    }
    unsafe fn ClearDepthx(&mut self, depth: GLclampx) {
        gles11::ClearDepthx(depth)
    }
    unsafe fn ClearStencil(&mut self, s: GLint) {
        gles11::ClearStencil(s)
    }

    // Textures
    unsafe fn PixelStorei(&mut self, pname: GLenum, param: GLint) {
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
        gles11::GenTextures(n, textures)
    }
    unsafe fn DeleteTextures(&mut self, n: GLsizei, textures: *const GLuint) {
        gles11::DeleteTextures(n, textures)
    }
    unsafe fn ActiveTexture(&mut self, texture: GLenum) {
        gles11::ActiveTexture(texture)
    }
    unsafe fn IsTexture(&mut self, texture: GLuint) -> GLboolean {
        gles11::IsTexture(texture)
    }
    unsafe fn BindTexture(&mut self, target: GLenum, texture: GLuint) {
        gles11::BindTexture(target, texture)
    }
    unsafe fn TexParameteri(&mut self, target: GLenum, pname: GLenum, param: GLint) {
        gles11::TexParameteri(target, pname, param)
    }
    unsafe fn TexParameterf(&mut self, target: GLenum, pname: GLenum, param: GLfloat) {
        gles11::TexParameterf(target, pname, param)
    }
    unsafe fn TexParameterx(&mut self, target: GLenum, pname: GLenum, param: GLfixed) {
        gles11::TexParameterx(target, pname, param)
    }
    unsafe fn TexParameteriv(&mut self, target: GLenum, pname: GLenum, params: *const GLint) {
        gles11::TexParameteriv(target, pname, params)
    }
    unsafe fn TexParameterfv(&mut self, target: GLenum, pname: GLenum, params: *const GLfloat) {
        gles11::TexParameterfv(target, pname, params)
    }
    unsafe fn TexParameterxv(&mut self, target: GLenum, pname: GLenum, params: *const GLfixed) {
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
        if format == gles11::BGRA_EXT {
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
        // POSIX-style guard: a NULL data pointer with image_size==0 is
        // technically allowed by the spec for some texture-storage queries,
        // but in practice no iPhone OS app does this — it'd just be a guest
        // bug. Drop the call on the floor instead of dereferencing the
        // pointer.
        if data.is_null() && image_size > 0 {
            log!(
                "Warning: GLES1Native::CompressedTexImage2D: NULL data with \
                 non-zero image_size {image_size} (target={target:#x}, \
                 level={level}, format={internalformat:#x}, {width}x{height}); \
                 dropping upload."
            );
            return;
        }

        // Slice the guest payload exactly once. Even when the host driver
        // advertises PVRTC natively, we need a `&[u8]` for the paletted /
        // decode fallbacks below.
        let payload: &[u8] = if image_size > 0 {
            std::slice::from_raw_parts(data.cast::<u8>(), image_size as usize)
        } else {
            &[]
        };

        // PowerVR-class drivers (Apple's iPhone OS, plus desktop GL via the
        // Mesa PowerVR backend) support `GL_IMG_texture_compression_pvrtc`
        // natively, in which case the most efficient thing to do is to hand
        // the compressed payload straight to the driver. ARM Mali, Qualcomm
        // Adreno's ES 1.1 surface, the Mesa software rasteriser, etc. do
        // *not* advertise PVRTC, so we need to software-decode to RGBA
        // before uploading. The decision is based on the
        // `GL_EXTENSIONS` string queried at context creation; see
        // [`GLES1NativeContext::pvrtc_native`].
        if !self.pvrtc_native && !payload.is_empty() {
            if try_decode_pvrtc(
                self,
                target,
                level,
                internalformat,
                width,
                height,
                border,
                payload,
            ) {
                return;
            }
            // Apple-targeted apps also sometimes ship
            // `GL_OES_compressed_paletted_texture` data. Mali / Adreno ES 1.1
            // surfaces likewise don't advertise that extension, so a
            // straight passthrough would silently fail with
            // `GL_INVALID_ENUM`. Software-decode paletted textures to
            // uncompressed RGBA/RGB and upload via glTexImage2D.
            if let Some(PalettedTextureFormat {
                index_is_nibble,
                palette_entry_format,
                palette_entry_type,
            }) = PalettedTextureFormat::get_info(internalformat)
            {
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
                let palette_entry_count: usize = if index_is_nibble { 16 } else { 256 };
                let palette_size = palette_entry_size * palette_entry_count;

                let index_count = width as usize * height as usize;
                let (index_word_size, index_word_count) = if index_is_nibble {
                    (1, index_count.div_ceil(2))
                } else {
                    (4, index_count.div_ceil(4))
                };
                let indices_size = index_word_size * index_word_count;

                let expected_size = palette_size + indices_size;
                if payload.len() < expected_size {
                    log!(
                        "Warning: GLES1Native::CompressedTexImage2D: paletted \
                         format {internalformat:#x} payload too small: got {} \
                         bytes, expected at least {expected_size} for \
                         {width}x{height}; skipping upload.",
                        payload.len()
                    );
                    return;
                }

                let (palette, indices) = payload.split_at(palette_size);

                let mut decoded = Vec::<u8>::with_capacity(palette_entry_size * index_count);
                for i in 0..index_count {
                    let index = if index_is_nibble {
                        (indices[i / 2] >> ((1 - (i % 2)) * 4)) & 0xf
                    } else {
                        indices[i]
                    } as usize;
                    let start = index * palette_entry_size;
                    let palette_entry = &palette[start..start + palette_entry_size];
                    decoded.extend_from_slice(palette_entry);
                }

                log_dbg!(
                    "GLES1Native: software-decoded paletted texture \
                     {width}x{height} (format {internalformat:#x})"
                );

                gles11::TexImage2D(
                    target,
                    level,
                    palette_entry_format as GLint,
                    width,
                    height,
                    border,
                    palette_entry_format,
                    palette_entry_type,
                    decoded.as_ptr() as *const _,
                );
                return;
            }
            // Unknown compressed format AND host driver doesn't advertise
            // PVRTC — passthrough would just produce GL_INVALID_ENUM. Log
            // once per (format) value so a misbehaving guest can't spam
            // the console.
            use std::sync::atomic::{AtomicBool, Ordering};
            static SEEN_UNKNOWN: AtomicBool = AtomicBool::new(false);
            if !SEEN_UNKNOWN.swap(true, Ordering::Relaxed) {
                log!(
                    "Warning: GLES1Native::CompressedTexImage2D: unknown \
                     compressed format {internalformat:#x} on a host that does \
                     not advertise PVRTC; passing through to driver but \
                     expecting GL_INVALID_ENUM. {width}x{height}, level {level}. \
                     [this log will only be shown once for unknown formats]"
                );
            }
        }

        // Either we're on a PVRTC-capable host (let the driver do its thing),
        // or we hit a non-PVRTC, non-paletted format on a non-PVRTC host
        // (let it fail loudly with GL_INVALID_ENUM, exactly like a real
        // device would).
        gles11::CompressedTexImage2D(
            target,
            level,
            internalformat,
            width,
            height,
            border,
            image_size,
            data,
        );
    }

    unsafe fn CompressedTexSubImage2D(
        &mut self,
        target: GLenum,
        level: GLint,
        xoffset: GLint,
        yoffset: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        image_size: GLsizei,
        data: *const GLvoid,
    ) {
        // PVRTC sub-image updates aren't allowed by the IMG spec — Apple's
        // ES 1.1 driver returns GL_INVALID_OPERATION too — but we forward
        // anyway so the guest sees the expected error. Most apps never call
        // this entry point.
        if data.is_null() && image_size > 0 {
            log!(
                "Warning: GLES1Native::CompressedTexSubImage2D: NULL data with \
                 non-zero image_size {image_size}; dropping call."
            );
            return;
        }
        gles11::CompressedTexSubImage2D(
            target, level, xoffset, yoffset, width, height, format, image_size, data,
        );
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
        gles11::CopyTexSubImage2D(target, level, xoffset, yoffset, x, y, width, height)
    }
    unsafe fn TexEnvf(&mut self, target: GLenum, pname: GLenum, param: GLfloat) {
        gles11::TexEnvf(target, pname, param)
    }
    unsafe fn TexEnvx(&mut self, target: GLenum, pname: GLenum, param: GLfixed) {
        gles11::TexEnvx(target, pname, param)
    }
    unsafe fn TexEnvi(&mut self, target: GLenum, pname: GLenum, param: GLint) {
        gles11::TexEnvi(target, pname, param)
    }

    unsafe fn TexEnvfv(&mut self, target: GLenum, pname: GLenum, params: *const GLfloat) {
        if target == gles11::TEXTURE_FILTER_CONTROL_EXT {
            assert!(pname == gles11::TEXTURE_LOD_BIAS_EXT);
            unsafe {
                if !CStr::from_ptr(gles11::GetString(gles11::EXTENSIONS) as _)
                    .to_str()
                    .unwrap()
                    .contains("EXT_texture_lod_bias")
                {
                    return;
                }
            };
        }
        gles11::TexEnvfv(target, pname, params)
    }

    unsafe fn TexEnvxv(&mut self, target: GLenum, pname: GLenum, params: *const GLfixed) {
        gles11::TexEnvxv(target, pname, params)
    }
    unsafe fn TexEnviv(&mut self, target: GLenum, pname: GLenum, params: *const GLint) {
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
        gles11::MultiTexCoord4x(target, s, t, r, q)
    }

    // Matrix stack operations
    unsafe fn MatrixMode(&mut self, mode: GLenum) {
        gles11::MatrixMode(mode)
    }
    unsafe fn LoadIdentity(&mut self) {
        gles11::LoadIdentity()
    }
    unsafe fn LoadMatrixf(&mut self, m: *const GLfloat) {
        gles11::LoadMatrixf(m)
    }
    unsafe fn LoadMatrixx(&mut self, m: *const GLfixed) {
        gles11::LoadMatrixx(m)
    }
    unsafe fn MultMatrixf(&mut self, m: *const GLfloat) {
        gles11::MultMatrixf(m)
    }
    unsafe fn MultMatrixx(&mut self, m: *const GLfixed) {
        gles11::MultMatrixx(m)
    }
    unsafe fn PushMatrix(&mut self) {
        gles11::PushMatrix()
    }
    unsafe fn PopMatrix(&mut self) {
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
        gles11::Frustumx(left, right, bottom, top, near, far)
    }
    unsafe fn Rotatef(&mut self, angle: GLfloat, x: GLfloat, y: GLfloat, z: GLfloat) {
        gles11::Rotatef(angle, x, y, z)
    }
    unsafe fn Rotatex(&mut self, angle: GLfixed, x: GLfixed, y: GLfixed, z: GLfixed) {
        gles11::Rotatex(angle, x, y, z)
    }
    unsafe fn Scalef(&mut self, x: GLfloat, y: GLfloat, z: GLfloat) {
        gles11::Scalef(x, y, z)
    }
    unsafe fn Scalex(&mut self, x: GLfixed, y: GLfixed, z: GLfixed) {
        gles11::Scalex(x, y, z)
    }
    unsafe fn Translatef(&mut self, x: GLfloat, y: GLfloat, z: GLfloat) {
        gles11::Translatef(x, y, z)
    }
    unsafe fn Translatex(&mut self, x: GLfixed, y: GLfixed, z: GLfixed) {
        gles11::Translatex(x, y, z)
    }

    // OES_framebuffer_object -> EXT_framebuffer_object
    unsafe fn GenFramebuffersOES(&mut self, n: GLsizei, framebuffers: *mut GLuint) {
        gles11::GenFramebuffersOES(n, framebuffers)
    }
    unsafe fn GenRenderbuffersOES(&mut self, n: GLsizei, renderbuffers: *mut GLuint) {
        gles11::GenRenderbuffersOES(n, renderbuffers)
    }
    unsafe fn IsFramebufferOES(&mut self, renderbuffer: GLuint) -> GLboolean {
        gles11::IsFramebufferOES(renderbuffer)
    }
    unsafe fn IsRenderbufferOES(&mut self, renderbuffer: GLuint) -> GLboolean {
        gles11::IsRenderbufferOES(renderbuffer)
    }
    unsafe fn BindFramebufferOES(&mut self, target: GLenum, framebuffer: GLuint) {
        gles11::BindFramebufferOES(target, framebuffer)
    }
    unsafe fn BindRenderbufferOES(&mut self, target: GLenum, renderbuffer: GLuint) {
        gles11::BindRenderbufferOES(target, renderbuffer)
    }
    unsafe fn RenderbufferStorageOES(
        &mut self,
        target: GLenum,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
    ) {
        gles11::RenderbufferStorageOES(target, internalformat, width, height)
    }
    unsafe fn FramebufferRenderbufferOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        renderbuffertarget: GLenum,
        renderbuffer: GLuint,
    ) {
        gles11::FramebufferRenderbufferOES(target, attachment, renderbuffertarget, renderbuffer)
    }
    unsafe fn FramebufferTexture2DOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        textarget: GLenum,
        texture: GLuint,
        level: i32,
    ) {
        gles11::FramebufferTexture2DOES(target, attachment, textarget, texture, level)
    }
    unsafe fn GetFramebufferAttachmentParameterivOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gles11::GetFramebufferAttachmentParameterivOES(target, attachment, pname, params)
    }
    unsafe fn GetRenderbufferParameterivOES(
        &mut self,
        target: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gles11::GetRenderbufferParameterivOES(target, pname, params)
    }
    unsafe fn CheckFramebufferStatusOES(&mut self, target: GLenum) -> GLenum {
        gles11::CheckFramebufferStatusOES(target)
    }
    unsafe fn DeleteFramebuffersOES(&mut self, n: GLsizei, framebuffers: *const GLuint) {
        gles11::DeleteFramebuffersOES(n, framebuffers)
    }
    unsafe fn DeleteRenderbuffersOES(&mut self, n: GLsizei, renderbuffers: *const GLuint) {
        gles11::DeleteRenderbuffersOES(n, renderbuffers)
    }
    unsafe fn GenerateMipmapOES(&mut self, target: GLenum) {
        gles11::GenerateMipmapOES(target)
    }

    // Non-OES aliases for OES_framebuffer_object functions.
    unsafe fn GenFramebuffers(&mut self, n: GLsizei, framebuffers: *mut GLuint) {
        self.GenFramebuffersOES(n, framebuffers)
    }
    unsafe fn GenRenderbuffers(&mut self, n: GLsizei, renderbuffers: *mut GLuint) {
        self.GenRenderbuffersOES(n, renderbuffers)
    }
    unsafe fn IsFramebuffer(&mut self, framebuffer: GLuint) -> GLboolean {
        self.IsFramebufferOES(framebuffer)
    }
    unsafe fn IsRenderbuffer(&mut self, renderbuffer: GLuint) -> GLboolean {
        self.IsRenderbufferOES(renderbuffer)
    }
    unsafe fn BindFramebuffer(&mut self, target: GLenum, framebuffer: GLuint) {
        self.BindFramebufferOES(target, framebuffer)
    }
    unsafe fn BindRenderbuffer(&mut self, target: GLenum, renderbuffer: GLuint) {
        self.BindRenderbufferOES(target, renderbuffer)
    }
    unsafe fn RenderbufferStorage(
        &mut self,
        target: GLenum,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
    ) {
        self.RenderbufferStorageOES(target, internalformat, width, height)
    }
    unsafe fn FramebufferRenderbuffer(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        renderbuffertarget: GLenum,
        renderbuffer: GLuint,
    ) {
        self.FramebufferRenderbufferOES(target, attachment, renderbuffertarget, renderbuffer)
    }
    unsafe fn FramebufferTexture2D(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        textarget: GLenum,
        texture: GLuint,
        level: i32,
    ) {
        self.FramebufferTexture2DOES(target, attachment, textarget, texture, level)
    }
    unsafe fn CheckFramebufferStatus(&mut self, target: GLenum) -> GLenum {
        self.CheckFramebufferStatusOES(target)
    }
    unsafe fn DeleteFramebuffers(&mut self, n: GLsizei, framebuffers: *const GLuint) {
        self.DeleteFramebuffersOES(n, framebuffers)
    }
    unsafe fn DeleteRenderbuffers(&mut self, n: GLsizei, renderbuffers: *const GLuint) {
        self.DeleteRenderbuffersOES(n, renderbuffers)
    }
    unsafe fn GenerateMipmap(&mut self, target: GLenum) {
        self.GenerateMipmapOES(target)
    }
    unsafe fn GetFramebufferAttachmentParameteriv(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        self.GetFramebufferAttachmentParameterivOES(target, attachment, pname, params)
    }
    unsafe fn GetRenderbufferParameteriv(
        &mut self,
        target: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        self.GetRenderbufferParameterivOES(target, pname, params)
    }
    unsafe fn GetBufferParameteriv(&mut self, target: GLenum, pname: GLenum, params: *mut GLint) {
        gles11::GetBufferParameteriv(target, pname, params)
    }
    unsafe fn MapBufferOES(&mut self, target: GLenum, access: GLenum) -> *mut GLvoid {
        gles11::MapBufferOES(target, access)
    }
    unsafe fn UnmapBufferOES(&mut self, target: GLenum) -> GLboolean {
        gles11::UnmapBufferOES(target)
    }

    // ===== OpenGL ES 2.0 shader-pipeline entry points =====
    //
    // None of these exist on a strict ES 1.1 driver. The ES 2.0 / ES 1.1
    // specifications agree that an implementation must report
    // `GL_INVALID_OPERATION` for "operation not supported by this profile",
    // so we route every entry point to [`record_es2_unsupported`] which:
    //   1. flags `GL_INVALID_OPERATION` in the synthetic error queue (read by
    //      our `GetError` override above), and
    //   2. logs a one-shot warning that the guest app is mixing ES 1.1 and
    //      ES 2.0 calls.
    // Methods with non-`()` return types pick the standard "failure" sentinel
    // value defined by the ES 2.0 spec: `0` for object names
    // (`glCreateShader`, `glCreateProgram`), `-1` for locations
    // (`glGetUniformLocation`, `glGetAttribLocation`), `GL_FALSE` for
    // boolean queries (`glIsShader`, `glIsProgram`).
    unsafe fn CreateShader(&mut self, _type_: GLenum) -> GLuint {
        self.record_es2_unsupported("CreateShader");
        0
    }
    unsafe fn DeleteShader(&mut self, _shader: GLuint) {
        self.record_es2_unsupported("DeleteShader");
    }
    unsafe fn ShaderSource(
        &mut self,
        _shader: GLuint,
        _count: GLsizei,
        _string: *const *const GLchar,
        _length: *const GLint,
    ) {
        self.record_es2_unsupported("ShaderSource");
    }
    unsafe fn CompileShader(&mut self, _shader: GLuint) {
        self.record_es2_unsupported("CompileShader");
    }
    unsafe fn GetShaderiv(&mut self, _shader: GLuint, _pname: GLenum, _params: *mut GLint) {
        self.record_es2_unsupported("GetShaderiv");
    }
    unsafe fn GetShaderInfoLog(
        &mut self,
        _shader: GLuint,
        _maxLength: GLsizei,
        _length: *mut GLsizei,
        _infoLog: *mut GLchar,
    ) {
        self.record_es2_unsupported("GetShaderInfoLog");
    }
    unsafe fn IsShader(&mut self, _shader: GLuint) -> GLboolean {
        self.record_es2_unsupported("IsShader");
        gles11::FALSE
    }
    unsafe fn CreateProgram(&mut self) -> GLuint {
        self.record_es2_unsupported("CreateProgram");
        0
    }
    unsafe fn DeleteProgram(&mut self, _program: GLuint) {
        self.record_es2_unsupported("DeleteProgram");
    }
    unsafe fn AttachShader(&mut self, _program: GLuint, _shader: GLuint) {
        self.record_es2_unsupported("AttachShader");
    }
    unsafe fn DetachShader(&mut self, _program: GLuint, _shader: GLuint) {
        self.record_es2_unsupported("DetachShader");
    }
    unsafe fn LinkProgram(&mut self, _program: GLuint) {
        self.record_es2_unsupported("LinkProgram");
    }
    unsafe fn UseProgram(&mut self, _program: GLuint) {
        self.record_es2_unsupported("UseProgram");
    }
    unsafe fn GetProgramiv(&mut self, _program: GLuint, _pname: GLenum, _params: *mut GLint) {
        self.record_es2_unsupported("GetProgramiv");
    }
    unsafe fn GetProgramInfoLog(
        &mut self,
        _program: GLuint,
        _maxLength: GLsizei,
        _length: *mut GLsizei,
        _infoLog: *mut GLchar,
    ) {
        self.record_es2_unsupported("GetProgramInfoLog");
    }
    unsafe fn IsProgram(&mut self, _program: GLuint) -> GLboolean {
        self.record_es2_unsupported("IsProgram");
        gles11::FALSE
    }
    unsafe fn ValidateProgram(&mut self, _program: GLuint) {
        self.record_es2_unsupported("ValidateProgram");
    }
    unsafe fn BindAttribLocation(
        &mut self,
        _program: GLuint,
        _index: GLuint,
        _name: *const GLchar,
    ) {
        self.record_es2_unsupported("BindAttribLocation");
    }
    unsafe fn GetAttribLocation(&mut self, _program: GLuint, _name: *const GLchar) -> GLint {
        self.record_es2_unsupported("GetAttribLocation");
        -1
    }
    unsafe fn GetUniformLocation(&mut self, _program: GLuint, _name: *const GLchar) -> GLint {
        self.record_es2_unsupported("GetUniformLocation");
        -1
    }
    unsafe fn GetActiveAttrib(
        &mut self,
        _program: GLuint,
        _index: GLuint,
        _bufSize: GLsizei,
        _length: *mut GLsizei,
        _size: *mut GLint,
        _type_: *mut GLenum,
        _name: *mut GLchar,
    ) {
        self.record_es2_unsupported("GetActiveAttrib");
    }
    unsafe fn GetActiveUniform(
        &mut self,
        _program: GLuint,
        _index: GLuint,
        _bufSize: GLsizei,
        _length: *mut GLsizei,
        _size: *mut GLint,
        _type_: *mut GLenum,
        _name: *mut GLchar,
    ) {
        self.record_es2_unsupported("GetActiveUniform");
    }
    unsafe fn EnableVertexAttribArray(&mut self, _index: GLuint) {
        self.record_es2_unsupported("EnableVertexAttribArray");
    }
    unsafe fn DisableVertexAttribArray(&mut self, _index: GLuint) {
        self.record_es2_unsupported("DisableVertexAttribArray");
    }
    unsafe fn VertexAttribPointer(
        &mut self,
        _index: GLuint,
        _size: GLint,
        _type_: GLenum,
        _normalized: GLboolean,
        _stride: GLsizei,
        _pointer: *const GLvoid,
    ) {
        self.record_es2_unsupported("VertexAttribPointer");
    }
    unsafe fn VertexAttrib1f(&mut self, _index: GLuint, _x: GLfloat) {
        self.record_es2_unsupported("VertexAttrib1f");
    }
    unsafe fn VertexAttrib2f(&mut self, _index: GLuint, _x: GLfloat, _y: GLfloat) {
        self.record_es2_unsupported("VertexAttrib2f");
    }
    unsafe fn VertexAttrib3f(&mut self, _index: GLuint, _x: GLfloat, _y: GLfloat, _z: GLfloat) {
        self.record_es2_unsupported("VertexAttrib3f");
    }
    unsafe fn VertexAttrib4f(
        &mut self,
        _index: GLuint,
        _x: GLfloat,
        _y: GLfloat,
        _z: GLfloat,
        _w: GLfloat,
    ) {
        self.record_es2_unsupported("VertexAttrib4f");
    }
    unsafe fn VertexAttrib1fv(&mut self, _index: GLuint, _v: *const GLfloat) {
        self.record_es2_unsupported("VertexAttrib1fv");
    }
    unsafe fn VertexAttrib2fv(&mut self, _index: GLuint, _v: *const GLfloat) {
        self.record_es2_unsupported("VertexAttrib2fv");
    }
    unsafe fn VertexAttrib3fv(&mut self, _index: GLuint, _v: *const GLfloat) {
        self.record_es2_unsupported("VertexAttrib3fv");
    }
    unsafe fn VertexAttrib4fv(&mut self, _index: GLuint, _v: *const GLfloat) {
        self.record_es2_unsupported("VertexAttrib4fv");
    }
    unsafe fn Uniform1f(&mut self, _location: GLint, _v0: GLfloat) {
        self.record_es2_unsupported("Uniform1f");
    }
    unsafe fn Uniform2f(&mut self, _location: GLint, _v0: GLfloat, _v1: GLfloat) {
        self.record_es2_unsupported("Uniform2f");
    }
    unsafe fn Uniform3f(&mut self, _location: GLint, _v0: GLfloat, _v1: GLfloat, _v2: GLfloat) {
        self.record_es2_unsupported("Uniform3f");
    }
    unsafe fn Uniform4f(
        &mut self,
        _location: GLint,
        _v0: GLfloat,
        _v1: GLfloat,
        _v2: GLfloat,
        _v3: GLfloat,
    ) {
        self.record_es2_unsupported("Uniform4f");
    }
    unsafe fn Uniform1i(&mut self, _location: GLint, _v0: GLint) {
        self.record_es2_unsupported("Uniform1i");
    }
    unsafe fn Uniform2i(&mut self, _location: GLint, _v0: GLint, _v1: GLint) {
        self.record_es2_unsupported("Uniform2i");
    }
    unsafe fn Uniform3i(&mut self, _location: GLint, _v0: GLint, _v1: GLint, _v2: GLint) {
        self.record_es2_unsupported("Uniform3i");
    }
    unsafe fn Uniform4i(
        &mut self,
        _location: GLint,
        _v0: GLint,
        _v1: GLint,
        _v2: GLint,
        _v3: GLint,
    ) {
        self.record_es2_unsupported("Uniform4i");
    }
    unsafe fn Uniform1fv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLfloat) {
        self.record_es2_unsupported("Uniform1fv");
    }
    unsafe fn Uniform2fv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLfloat) {
        self.record_es2_unsupported("Uniform2fv");
    }
    unsafe fn Uniform3fv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLfloat) {
        self.record_es2_unsupported("Uniform3fv");
    }
    unsafe fn Uniform4fv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLfloat) {
        self.record_es2_unsupported("Uniform4fv");
    }
    unsafe fn Uniform1iv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLint) {
        self.record_es2_unsupported("Uniform1iv");
    }
    unsafe fn Uniform2iv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLint) {
        self.record_es2_unsupported("Uniform2iv");
    }
    unsafe fn Uniform3iv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLint) {
        self.record_es2_unsupported("Uniform3iv");
    }
    unsafe fn Uniform4iv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLint) {
        self.record_es2_unsupported("Uniform4iv");
    }
    unsafe fn UniformMatrix2fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        self.record_es2_unsupported("UniformMatrix2fv");
    }
    unsafe fn UniformMatrix3fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        self.record_es2_unsupported("UniformMatrix3fv");
    }
    unsafe fn UniformMatrix4fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        self.record_es2_unsupported("UniformMatrix4fv");
    }
    unsafe fn BlendColor(&mut self, _r: GLclampf, _g: GLclampf, _b: GLclampf, _a: GLclampf) {
        self.record_es2_unsupported("BlendColor");
    }
    unsafe fn BlendEquation(&mut self, _mode: GLenum) {
        self.record_es2_unsupported("BlendEquation");
    }
    unsafe fn BlendEquationSeparate(&mut self, _modeRGB: GLenum, _modeAlpha: GLenum) {
        self.record_es2_unsupported("BlendEquationSeparate");
    }
    unsafe fn BlendFuncSeparate(
        &mut self,
        _srcRGB: GLenum,
        _dstRGB: GLenum,
        _srcAlpha: GLenum,
        _dstAlpha: GLenum,
    ) {
        self.record_es2_unsupported("BlendFuncSeparate");
    }
    unsafe fn StencilFuncSeparate(
        &mut self,
        _face: GLenum,
        _func: GLenum,
        _ref_: GLint,
        _mask: GLuint,
    ) {
        self.record_es2_unsupported("StencilFuncSeparate");
    }
    unsafe fn StencilOpSeparate(
        &mut self,
        _face: GLenum,
        _sfail: GLenum,
        _dpfail: GLenum,
        _dppass: GLenum,
    ) {
        self.record_es2_unsupported("StencilOpSeparate");
    }
    unsafe fn StencilMaskSeparate(&mut self, _face: GLenum, _mask: GLuint) {
        self.record_es2_unsupported("StencilMaskSeparate");
    }
    unsafe fn GetVertexAttribiv(&mut self, _index: GLuint, _pname: GLenum, _params: *mut GLint) {
        self.record_es2_unsupported("GetVertexAttribiv");
    }
    unsafe fn GetVertexAttribfv(&mut self, _index: GLuint, _pname: GLenum, _params: *mut GLfloat) {
        self.record_es2_unsupported("GetVertexAttribfv");
    }
    unsafe fn GetVertexAttribPointerv(
        &mut self,
        _index: GLuint,
        _pname: GLenum,
        _pointer: *mut *mut GLvoid,
    ) {
        self.record_es2_unsupported("GetVertexAttribPointerv");
    }
    unsafe fn GetUniformiv(&mut self, _program: GLuint, _location: GLint, _params: *mut GLint) {
        self.record_es2_unsupported("GetUniformiv");
    }
    unsafe fn GetUniformfv(&mut self, _program: GLuint, _location: GLint, _params: *mut GLfloat) {
        self.record_es2_unsupported("GetUniformfv");
    }
    unsafe fn GetAttachedShaders(
        &mut self,
        _program: GLuint,
        _maxCount: GLsizei,
        _count: *mut GLsizei,
        _shaders: *mut GLuint,
    ) {
        self.record_es2_unsupported("GetAttachedShaders");
    }
    unsafe fn GetShaderSource(
        &mut self,
        _shader: GLuint,
        _bufSize: GLsizei,
        _length: *mut GLsizei,
        _source: *mut GLchar,
    ) {
        self.record_es2_unsupported("GetShaderSource");
    }
    unsafe fn GetShaderPrecisionFormat(
        &mut self,
        _shadertype: GLenum,
        _precisiontype: GLenum,
        _range: *mut GLint,
        _precision: *mut GLint,
    ) {
        self.record_es2_unsupported("GetShaderPrecisionFormat");
    }
    unsafe fn ShaderBinary(
        &mut self,
        _count: GLsizei,
        _shaders: *const GLuint,
        _binaryformat: GLenum,
        _binary: *const GLvoid,
        _length: GLsizei,
    ) {
        self.record_es2_unsupported("ShaderBinary");
    }
    // `glReleaseShaderCompiler` is a hint, not a stateful operation; the
    // default `unsafe fn ReleaseShaderCompiler` in `gles_generic` already
    // returns `()` cleanly so we don't need to override it.
}

impl<'gl_ctx> GLES1Native<'gl_ctx> {
    /// Helper used by every `OpenGL ES 2.0` shader-pipeline override above
    /// to flag a synthetic `GL_INVALID_OPERATION` and emit a one-shot
    /// warning describing the offending call.
    fn record_es2_unsupported(&self, fn_name: &'static str) {
        // The synthetic error must be queued on *every* call, because the
        // ES 2.0 spec requires `GL_INVALID_OPERATION` to be reported each
        // time one of these entry points is hit on an ES 1.1 context.
        self.pending_synthetic_error
            .set(gles11::INVALID_OPERATION);

        // The accompanying human-readable warning, however, must only be
        // emitted once per distinct entry point. Apps such as Cut the Rope
        // and Angry Birds poll `glGetVertexAttribiv` every frame, which
        // previously flooded the log with thousands of identical lines and
        // added measurable per-frame overhead. Deduplicate via a
        // process-global set keyed by the (`'static`) function name.
        use std::collections::HashSet;
        use std::sync::Mutex;
        use std::sync::OnceLock;
        static LOGGED: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
        let logged = LOGGED.get_or_init(|| Mutex::new(HashSet::new()));
        let is_new = logged.lock().unwrap().insert(fn_name);
        if is_new {
            log!(
                "{} (OpenGL ES 2.0) called on a native ES 1.1 context; \
                 reporting GL_INVALID_OPERATION via glGetError as required \
                 by spec. [this log will only be shown once per entry point]",
                fn_name
            );
        }
    }
}
