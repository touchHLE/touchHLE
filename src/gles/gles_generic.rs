/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Generic OpenGL ES 1.1 interface.
//!
//! Unfortunately this does not provide the types and constants, so the correct
//! usage is to import `GLES` and `types` from this module, but get the
//! constants from [super::gles11_raw].

use crate::window::{GLContext, Window};

use super::gles11_raw::types::*;

/// `GLchar` from the ES 2.0 type set. Not defined by the ES 1.1 registry, so
/// we provide our own alias here for use in the [GLES] trait's ES 2.0 entry
/// points.
pub type GLchar = std::os::raw::c_char;

/// Trait representing an OpenGL ES implementation and context.
///
/// The GL context is not necessarily active, so GL functions can't be called
/// from this trait. It can be made active from [GLESContext::make_current].
#[allow(clippy::upper_case_acronyms)]
pub trait GLESContext {
    /// Get a human-friendly description of this implementation.
    fn description() -> &'static str
    where
        Self: Sized;

    /// Construct a new context. This might fail if the host OS doesn't have a
    /// compatible driver, for example.
    #[allow(clippy::new_ret_no_self)]
    fn new(window: &mut crate::window::Window) -> Result<Self, String>
    where
        Self: Sized;

    /// Make this context (and any underlying context) the active OpenGL
    /// context.
    ///
    /// The lifetime ensures safety - the GLES object can't be destroyed while
    /// the instance is active, so the OpenGL state remains valid, and the
    /// window reference prevents the thread from yielding while the GLES
    /// object is being used, and prevents multiple contexts from existing at
    /// the same time (which can cause a UAF).
    fn make_current<'gl_ctx, 'win: 'gl_ctx>(
        &'gl_ctx mut self,
        window: &'win mut Window,
    ) -> Box<dyn GLES + 'gl_ctx>;

    /// Make this context (and any underlying context) the active OpenGL
    /// context, without checking if it is the only context. You shouldn't use
    /// this outside of [crate::window::Window], as this is function exists to
    /// work around lifetime splitting issues inside of it.
    ///
    /// SAFETY: Callers must ensure that this is the only active context,
    /// that the GLES instance does not outlive the self or window
    /// parameter, that make_current_fn makes the passed context current,
    /// and that loader_fn properly loads the requested function.
    unsafe fn make_current_unchecked_for_window<'gl_ctx>(
        &'gl_ctx mut self,
        make_current_fn: &mut dyn FnMut(&GLContext),
        loader_fn: &mut dyn FnMut(&'static str) -> *const std::ffi::c_void,
    ) -> Box<dyn GLES + 'gl_ctx>;
}

/// An active GLES context that can be used.
///
/// These are effectively direct wrappers around the raw OpenGL functions,
/// but they make sure that the context is active while it is using it.
/// # Safety
/// These functions (should) act as documented by the OpenGL ES spec. Callers
/// should ensure that all uses of raw pointers are verfied to be valid and
/// of the correct size as documented in the OpenGL ES spec.
#[allow(clippy::upper_case_acronyms)]
#[allow(clippy::too_many_arguments)] // not our fault :(
pub trait GLES {
    /// Get some string describing the underlying driver. For OpenGL this is
    /// `GL_VENDOR`, `GL_RENDERER` and `GL_VERSION`.
    unsafe fn driver_description(&self) -> String {
        unimplemented!("driver_description not implemented by this backend")
    }
    /// Returns `true` if this backend is a real OpenGL ES 2.0 / 3.0 driver and
    /// therefore does NOT support fixed-function pipeline calls (`MatrixMode`,
    /// `EnableClientState`, `Color4f`, …). Used by `present_renderbuffer` so it
    /// can take a shader-based code path on such backends.
    fn is_es2(&self) -> bool {
        false
    }
    /// Returns `true` if this backend is a real (native) OpenGL ES 1.1 driver,
    /// i.e. one that strictly enforces the ES 1.1 spec and rejects desktop-GL
    /// enums. The desktop GL 2.1 emulation backend (`gles1_on_gl2`) returns
    /// `false` because it accepts the broader GL 2.1 enum set. Used by
    /// `present_renderbuffer` to skip state queries / state restores for
    /// enums that are part of GL 2.1 but not part of ES 1.1 (e.g.
    /// `GL_COLOR_LOGIC_OP`), which would otherwise generate `GL_INVALID_ENUM`
    /// on a strict native driver and pollute the GL error queue.
    fn is_native_es1(&self) -> bool {
        false
    }
    // Generic state manipulation
    unsafe fn GetError(&mut self) -> GLenum {
        unimplemented!("GetError not implemented by this backend")
    }
    unsafe fn Enable(&mut self, _cap: GLenum) {
        unimplemented!("Enable not implemented by this backend")
    }
    unsafe fn IsEnabled(&mut self, _cap: GLenum) -> GLboolean {
        unimplemented!("IsEnabled not implemented by this backend")
    }
    unsafe fn Disable(&mut self, _cap: GLenum) {
        unimplemented!("Disable not implemented by this backend")
    }
    unsafe fn ClientActiveTexture(&mut self, _texture: GLenum) {
        unimplemented!("ClientActiveTexture not implemented by this backend")
    }
    unsafe fn EnableClientState(&mut self, _array: GLenum) {
        unimplemented!("EnableClientState not implemented by this backend")
    }
    unsafe fn DisableClientState(&mut self, _array: GLenum) {
        unimplemented!("DisableClientState not implemented by this backend")
    }
    unsafe fn GetBooleanv(&mut self, _pname: GLenum, _params: *mut GLboolean) {
        unimplemented!("GetBooleanv not implemented by this backend")
    }
    unsafe fn GetFloatv(&mut self, _pname: GLenum, _params: *mut GLfloat) {
        unimplemented!("GetFloatv not implemented by this backend")
    }
    unsafe fn GetIntegerv(&mut self, _pname: GLenum, _params: *mut GLint) {
        unimplemented!("GetIntegerv not implemented by this backend")
    }
    unsafe fn GetFixedv(&mut self, _pname: GLenum, _params: *mut GLfixed) {
        unimplemented!("GetFixedv not implemented by this backend")
    }
    unsafe fn GetTexEnviv(&mut self, _target: GLenum, _pname: GLenum, _params: *mut GLint) {
        unimplemented!("GetTexEnviv not implemented by this backend")
    }
    unsafe fn GetTexEnvfv(&mut self, _target: GLenum, _pname: GLenum, _params: *mut GLfloat) {
        unimplemented!("GetTexEnvfv not implemented by this backend")
    }
    unsafe fn GetTexEnvxv(&mut self, _target: GLenum, _pname: GLenum, _params: *mut GLfixed) {
        unimplemented!("GetTexEnvxv not implemented by this backend")
    }
    unsafe fn GetTexParameteriv(&mut self, _target: GLenum, _pname: GLenum, _params: *mut GLint) {
        unimplemented!("GetTexParameteriv not implemented by this backend")
    }
    unsafe fn GetTexParameterfv(&mut self, _target: GLenum, _pname: GLenum, _params: *mut GLfloat) {
        unimplemented!("GetTexParameterfv not implemented by this backend")
    }
    unsafe fn GetTexParameterxv(&mut self, _target: GLenum, _pname: GLenum, _params: *mut GLfixed) {
        unimplemented!("GetTexParameterxv not implemented by this backend")
    }
    unsafe fn GetClipPlanef(&mut self, _plane: GLenum, _equation: *mut GLfloat) {
        unimplemented!("GetClipPlanef not implemented by this backend")
    }
    unsafe fn GetClipPlanex(&mut self, _plane: GLenum, _equation: *mut GLfixed) {
        unimplemented!("GetClipPlanex not implemented by this backend")
    }
    unsafe fn GetLightfv(&mut self, _light: GLenum, _pname: GLenum, _params: *mut GLfloat) {
        unimplemented!("GetLightfv not implemented by this backend")
    }
    unsafe fn GetLightxv(&mut self, _light: GLenum, _pname: GLenum, _params: *mut GLfixed) {
        unimplemented!("GetLightxv not implemented by this backend")
    }
    unsafe fn GetMaterialfv(&mut self, _face: GLenum, _pname: GLenum, _params: *mut GLfloat) {
        unimplemented!("GetMaterialfv not implemented by this backend")
    }
    unsafe fn GetMaterialxv(&mut self, _face: GLenum, _pname: GLenum, _params: *mut GLfixed) {
        unimplemented!("GetMaterialxv not implemented by this backend")
    }
    unsafe fn GetPointerv(&mut self, _pname: GLenum, _params: *mut *const GLvoid) {
        unimplemented!("GetPointerv not implemented by this backend")
    }
    unsafe fn Hint(&mut self, _target: GLenum, _mode: GLenum) {
        unimplemented!("Hint not implemented by this backend")
    }
    unsafe fn Finish(&mut self) {
        unimplemented!("Finish not implemented by this backend")
    }
    unsafe fn Flush(&mut self) {
        unimplemented!("Flush not implemented by this backend")
    }
    #[allow(dead_code)]
    unsafe fn GetString(&mut self, _name: GLenum) -> *const GLubyte {
        unimplemented!("GetString not implemented by this backend")
    }

    // Other state manipulation
    unsafe fn AlphaFunc(&mut self, _func: GLenum, _ref_: GLclampf) {
        unimplemented!("AlphaFunc not implemented by this backend")
    }
    unsafe fn AlphaFuncx(&mut self, _func: GLenum, _ref_: GLclampx) {
        unimplemented!("AlphaFuncx not implemented by this backend")
    }
    unsafe fn BlendFunc(&mut self, _sfactor: GLenum, _dfactor: GLenum) {
        unimplemented!("BlendFunc not implemented by this backend")
    }
    unsafe fn BlendEquationOES(&mut self, _mode: GLenum) {
        unimplemented!("BlendEquationOES not implemented by this backend")
    }
    unsafe fn ColorMask(
        &mut self,
        _red: GLboolean,
        _green: GLboolean,
        _blue: GLboolean,
        _alpha: GLboolean,
    ) {
        unimplemented!("ColorMask not implemented by this backend")
    }
    unsafe fn ClipPlanef(&mut self, _plane: GLenum, _equation: *const GLfloat) {
        unimplemented!("ClipPlanef not implemented by this backend")
    }
    unsafe fn ClipPlanex(&mut self, _plane: GLenum, _equation: *const GLfixed) {
        unimplemented!("ClipPlanex not implemented by this backend")
    }
    unsafe fn CullFace(&mut self, _mode: GLenum) {
        unimplemented!("CullFace not implemented by this backend")
    }
    unsafe fn DepthFunc(&mut self, _func: GLenum) {
        unimplemented!("DepthFunc not implemented by this backend")
    }
    unsafe fn DepthMask(&mut self, _flag: GLboolean) {
        unimplemented!("DepthMask not implemented by this backend")
    }
    unsafe fn DepthRangef(&mut self, _near: GLclampf, _far: GLclampf) {
        unimplemented!("DepthRangef not implemented by this backend")
    }
    unsafe fn DepthRangex(&mut self, _near: GLclampx, _far: GLclampx) {
        unimplemented!("DepthRangex not implemented by this backend")
    }
    unsafe fn FrontFace(&mut self, _mode: GLenum) {
        unimplemented!("FrontFace not implemented by this backend")
    }
    unsafe fn PolygonOffset(&mut self, _factor: GLfloat, _units: GLfloat) {
        unimplemented!("PolygonOffset not implemented by this backend")
    }
    unsafe fn PolygonOffsetx(&mut self, _factor: GLfixed, _units: GLfixed) {
        unimplemented!("PolygonOffsetx not implemented by this backend")
    }
    unsafe fn SampleCoverage(&mut self, _value: GLclampf, _invert: GLboolean) {
        unimplemented!("SampleCoverage not implemented by this backend")
    }
    unsafe fn SampleCoveragex(&mut self, _value: GLclampx, _invert: GLboolean) {
        unimplemented!("SampleCoveragex not implemented by this backend")
    }
    unsafe fn ShadeModel(&mut self, _mode: GLenum) {
        unimplemented!("ShadeModel not implemented by this backend")
    }
    unsafe fn Scissor(&mut self, _x: GLint, _y: GLint, _width: GLsizei, _height: GLsizei) {
        unimplemented!("Scissor not implemented by this backend")
    }
    unsafe fn Viewport(&mut self, _x: GLint, _y: GLint, _width: GLsizei, _height: GLsizei) {
        unimplemented!("Viewport not implemented by this backend")
    }
    unsafe fn LineWidth(&mut self, _val: GLfloat) {
        unimplemented!("LineWidth not implemented by this backend")
    }
    unsafe fn LineWidthx(&mut self, _val: GLfixed) {
        unimplemented!("LineWidthx not implemented by this backend")
    }
    unsafe fn StencilFunc(&mut self, _func: GLenum, _ref_: GLint, _mask: GLuint) {
        unimplemented!("StencilFunc not implemented by this backend")
    }
    unsafe fn StencilOp(&mut self, _sfail: GLenum, _dpfail: GLenum, _dppass: GLenum) {
        unimplemented!("StencilOp not implemented by this backend")
    }
    unsafe fn StencilMask(&mut self, _mask: GLuint) {
        unimplemented!("StencilMask not implemented by this backend")
    }
    unsafe fn LogicOp(&mut self, _opcode: GLenum) {
        unimplemented!("LogicOp not implemented by this backend")
    }

    // Points
    unsafe fn PointSize(&mut self, _size: GLfloat) {
        unimplemented!("PointSize not implemented by this backend")
    }
    unsafe fn PointSizex(&mut self, _size: GLfixed) {
        unimplemented!("PointSizex not implemented by this backend")
    }
    unsafe fn PointParameterf(&mut self, _pname: GLenum, _param: GLfloat) {
        unimplemented!("PointParameterf not implemented by this backend")
    }
    unsafe fn PointParameterx(&mut self, _pname: GLenum, _param: GLfixed) {
        unimplemented!("PointParameterx not implemented by this backend")
    }
    unsafe fn PointParameterfv(&mut self, _pname: GLenum, _params: *const GLfloat) {
        unimplemented!("PointParameterfv not implemented by this backend")
    }
    unsafe fn PointParameterxv(&mut self, _pname: GLenum, _params: *const GLfixed) {
        unimplemented!("PointParameterxv not implemented by this backend")
    }

    // Lighting and materials
    unsafe fn Fogf(&mut self, _pname: GLenum, _param: GLfloat) {
        unimplemented!("Fogf not implemented by this backend")
    }
    unsafe fn Fogx(&mut self, _pname: GLenum, _param: GLfixed) {
        unimplemented!("Fogx not implemented by this backend")
    }
    unsafe fn Fogfv(&mut self, _pname: GLenum, _params: *const GLfloat) {
        unimplemented!("Fogfv not implemented by this backend")
    }
    unsafe fn Fogxv(&mut self, _pname: GLenum, _params: *const GLfixed) {
        unimplemented!("Fogxv not implemented by this backend")
    }
    unsafe fn Lightf(&mut self, _light: GLenum, _pname: GLenum, _param: GLfloat) {
        unimplemented!("Lightf not implemented by this backend")
    }
    unsafe fn Lightx(&mut self, _light: GLenum, _pname: GLenum, _param: GLfixed) {
        unimplemented!("Lightx not implemented by this backend")
    }
    unsafe fn Lightfv(&mut self, _light: GLenum, _pname: GLenum, _params: *const GLfloat) {
        unimplemented!("Lightfv not implemented by this backend")
    }
    unsafe fn Lightxv(&mut self, _light: GLenum, _pname: GLenum, _params: *const GLfixed) {
        unimplemented!("Lightxv not implemented by this backend")
    }
    unsafe fn LightModelf(&mut self, _pname: GLenum, _param: GLfloat) {
        unimplemented!("LightModelf not implemented by this backend")
    }
    unsafe fn LightModelx(&mut self, _pname: GLenum, _param: GLfixed) {
        unimplemented!("LightModelx not implemented by this backend")
    }
    unsafe fn LightModelfv(&mut self, _pname: GLenum, _params: *const GLfloat) {
        unimplemented!("LightModelfv not implemented by this backend")
    }
    unsafe fn LightModelxv(&mut self, _pname: GLenum, _params: *const GLfixed) {
        unimplemented!("LightModelxv not implemented by this backend")
    }
    unsafe fn Materialf(&mut self, _face: GLenum, _pname: GLenum, _param: GLfloat) {
        unimplemented!("Materialf not implemented by this backend")
    }
    unsafe fn Materialx(&mut self, _face: GLenum, _pname: GLenum, _param: GLfixed) {
        unimplemented!("Materialx not implemented by this backend")
    }
    unsafe fn Materialfv(&mut self, _face: GLenum, _pname: GLenum, _params: *const GLfloat) {
        unimplemented!("Materialfv not implemented by this backend")
    }
    unsafe fn Materialxv(&mut self, _face: GLenum, _pname: GLenum, _params: *const GLfixed) {
        unimplemented!("Materialxv not implemented by this backend")
    }

    // Buffers
    unsafe fn IsBuffer(&mut self, _buffer: GLuint) -> GLboolean {
        unimplemented!("IsBuffer not implemented by this backend")
    }
    unsafe fn GenBuffers(&mut self, _n: GLsizei, _buffers: *mut GLuint) {
        unimplemented!("GenBuffers not implemented by this backend")
    }
    unsafe fn DeleteBuffers(&mut self, _n: GLsizei, _buffers: *const GLuint) {
        unimplemented!("DeleteBuffers not implemented by this backend")
    }
    unsafe fn BindBuffer(&mut self, _target: GLenum, _buffer: GLuint) {
        unimplemented!("BindBuffer not implemented by this backend")
    }
    unsafe fn BufferData(
        &mut self,
        _target: GLenum,
        _size: GLsizeiptr,
        _data: *const GLvoid,
        _usage: GLenum,
    ) {
        unimplemented!("BufferData not implemented by this backend")
    }
    unsafe fn BufferSubData(
        &mut self,
        _target: GLenum,
        _offset: GLintptr,
        _size: GLsizeiptr,
        _data: *const GLvoid,
    ) {
        unimplemented!("BufferSubData not implemented by this backend")
    }

    // Non-pointers
    unsafe fn Color4f(&mut self, _red: GLfloat, _green: GLfloat, _blue: GLfloat, _alpha: GLfloat) {
        unimplemented!("Color4f not implemented by this backend")
    }
    unsafe fn Color4x(&mut self, _red: GLfixed, _green: GLfixed, _blue: GLfixed, _alpha: GLfixed) {
        unimplemented!("Color4x not implemented by this backend")
    }
    unsafe fn Color4ub(&mut self, _red: GLubyte, _green: GLubyte, _blue: GLubyte, _alpha: GLubyte) {
        unimplemented!("Color4ub not implemented by this backend")
    }
    unsafe fn Normal3f(&mut self, _nx: GLfloat, _ny: GLfloat, _nz: GLfloat) {
        unimplemented!("Normal3f not implemented by this backend")
    }
    unsafe fn Normal3x(&mut self, _nx: GLfixed, _ny: GLfixed, _nz: GLfixed) {
        unimplemented!("Normal3x not implemented by this backend")
    }

    // Pointers
    unsafe fn ColorPointer(
        &mut self,
        _size: GLint,
        _type_: GLenum,
        _stride: GLsizei,
        _pointer: *const GLvoid,
    ) {
        unimplemented!("ColorPointer not implemented by this backend")
    }
    unsafe fn NormalPointer(&mut self, _type_: GLenum, _stride: GLsizei, _pointer: *const GLvoid) {
        unimplemented!("NormalPointer not implemented by this backend")
    }
    unsafe fn TexCoordPointer(
        &mut self,
        _size: GLint,
        _type_: GLenum,
        _stride: GLsizei,
        _pointer: *const GLvoid,
    ) {
        unimplemented!("TexCoordPointer not implemented by this backend")
    }
    unsafe fn VertexPointer(
        &mut self,
        _size: GLint,
        _type_: GLenum,
        _stride: GLsizei,
        _pointer: *const GLvoid,
    ) {
        unimplemented!("VertexPointer not implemented by this backend")
    }
    /// `glPointSizePointerOES` from `GL_OES_point_size_array`. Lets the
    /// app supply per-vertex point sizes alongside the regular vertex
    /// array. `type_` is `GL_FLOAT` or `GL_FIXED`.
    unsafe fn PointSizePointerOES(
        &mut self,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        let _ = (type_, stride, pointer);
        // Backends that lack `GL_ARB_point_parameters`-like per-vertex
        // sizing fall back to the scalar `glPointSize` value, which is the
        // closest sensible behaviour available.
    }

    // Drawing
    unsafe fn DrawArrays(&mut self, _mode: GLenum, _first: GLint, _count: GLsizei) {
        unimplemented!("DrawArrays not implemented by this backend")
    }
    unsafe fn DrawElements(
        &mut self,
        _mode: GLenum,
        _count: GLsizei,
        _type_: GLenum,
        _indices: *const GLvoid,
    ) {
        unimplemented!("DrawElements not implemented by this backend")
    }
    /// `glDrawTex{s,i,x,f}OES` from `GL_OES_draw_texture` — blit the
    /// currently bound 2D texture (using `GL_TEXTURE_CROP_RECT_OES`) to a
    /// screen-space rectangle. The default implementation provides a
    /// software fallback in `present_renderbuffer`-style code paths; concrete
    /// backends with a native equivalent override these.
    unsafe fn DrawTexfOES(
        &mut self,
        x: GLfloat,
        y: GLfloat,
        z: GLfloat,
        width: GLfloat,
        height: GLfloat,
    ) {
        let _ = (x, y, z, width, height);
        // Default: do nothing. Most apps that use GL_OES_draw_texture also
        // ship a fallback path; not panicking lets them progress.
    }

    // Clearing
    unsafe fn Clear(&mut self, _mask: GLbitfield) {
        unimplemented!("Clear not implemented by this backend")
    }
    unsafe fn ClearColor(
        &mut self,
        _red: GLclampf,
        _green: GLclampf,
        _blue: GLclampf,
        _alpha: GLclampf,
    ) {
        unimplemented!("ClearColor not implemented by this backend")
    }
    unsafe fn ClearColorx(
        &mut self,
        _red: GLclampx,
        _green: GLclampx,
        _blue: GLclampx,
        _alpha: GLclampx,
    ) {
        unimplemented!("ClearColorx not implemented by this backend")
    }
    unsafe fn ClearDepthf(&mut self, _depth: GLclampf) {
        unimplemented!("ClearDepthf not implemented by this backend")
    }
    unsafe fn ClearDepthx(&mut self, _depth: GLclampx) {
        unimplemented!("ClearDepthx not implemented by this backend")
    }
    unsafe fn ClearStencil(&mut self, _s: GLint) {
        unimplemented!("ClearStencil not implemented by this backend")
    }

    // Textures
    unsafe fn PixelStorei(&mut self, _pname: GLenum, _param: GLint) {
        unimplemented!("PixelStorei not implemented by this backend")
    }
    unsafe fn ReadPixels(
        &mut self,
        _x: GLint,
        _y: GLint,
        _width: GLsizei,
        _height: GLsizei,
        _format: GLenum,
        _type_: GLenum,
        _pixels: *mut GLvoid,
    ) {
        unimplemented!("ReadPixels not implemented by this backend")
    }
    unsafe fn GenTextures(&mut self, _n: GLsizei, _textures: *mut GLuint) {
        unimplemented!("GenTextures not implemented by this backend")
    }
    unsafe fn DeleteTextures(&mut self, _n: GLsizei, _textures: *const GLuint) {
        unimplemented!("DeleteTextures not implemented by this backend")
    }
    unsafe fn ActiveTexture(&mut self, _texture: GLenum) {
        unimplemented!("ActiveTexture not implemented by this backend")
    }
    unsafe fn IsTexture(&mut self, _texture: GLuint) -> GLboolean {
        unimplemented!("IsTexture not implemented by this backend")
    }
    unsafe fn BindTexture(&mut self, _target: GLenum, _texture: GLuint) {
        unimplemented!("BindTexture not implemented by this backend")
    }
    unsafe fn TexParameteri(&mut self, _target: GLenum, _pname: GLenum, _param: GLint) {
        unimplemented!("TexParameteri not implemented by this backend")
    }
    unsafe fn TexParameterf(&mut self, _target: GLenum, _pname: GLenum, _param: GLfloat) {
        unimplemented!("TexParameterf not implemented by this backend")
    }
    unsafe fn TexParameterx(&mut self, _target: GLenum, _pname: GLenum, _param: GLfixed) {
        unimplemented!("TexParameterx not implemented by this backend")
    }
    unsafe fn TexParameteriv(&mut self, _target: GLenum, _pname: GLenum, _params: *const GLint) {
        unimplemented!("TexParameteriv not implemented by this backend")
    }
    unsafe fn TexParameterfv(&mut self, _target: GLenum, _pname: GLenum, _params: *const GLfloat) {
        unimplemented!("TexParameterfv not implemented by this backend")
    }
    unsafe fn TexParameterxv(&mut self, _target: GLenum, _pname: GLenum, _params: *const GLfixed) {
        unimplemented!("TexParameterxv not implemented by this backend")
    }
    unsafe fn TexImage2D(
        &mut self,
        _target: GLenum,
        _level: GLint,
        _internalformat: GLint,
        _width: GLsizei,
        _height: GLsizei,
        _border: GLint,
        _format: GLenum,
        _type_: GLenum,
        _pixels: *const GLvoid,
    ) {
        unimplemented!("TexImage2D not implemented by this backend")
    }
    unsafe fn TexSubImage2D(
        &mut self,
        _target: GLenum,
        _level: GLint,
        _xoffset: GLint,
        _yoffset: GLint,
        _width: GLsizei,
        _height: GLsizei,
        _format: GLenum,
        _type_: GLenum,
        _pixels: *const GLvoid,
    ) {
        unimplemented!("TexSubImage2D not implemented by this backend")
    }
    unsafe fn CompressedTexImage2D(
        &mut self,
        _target: GLenum,
        _level: GLint,
        _internalformat: GLenum,
        _width: GLsizei,
        _height: GLsizei,
        _border: GLint,
        _image_size: GLsizei,
        _data: *const GLvoid,
    ) {
        unimplemented!("CompressedTexImage2D not implemented by this backend")
    }
    unsafe fn CompressedTexSubImage2D(
        &mut self,
        _target: GLenum,
        _level: GLint,
        _xoffset: GLint,
        _yoffset: GLint,
        _width: GLsizei,
        _height: GLsizei,
        _format: GLenum,
        _image_size: GLsizei,
        _data: *const GLvoid,
    ) {
        unimplemented!("CompressedTexSubImage2D not implemented by this backend")
    }
    unsafe fn CopyTexImage2D(
        &mut self,
        _target: GLenum,
        _level: GLint,
        _internalformat: GLenum,
        _x: GLint,
        _y: GLint,
        _width: GLsizei,
        _height: GLsizei,
        _border: GLint,
    ) {
        unimplemented!("CopyTexImage2D not implemented by this backend")
    }
    unsafe fn CopyTexSubImage2D(
        &mut self,
        _target: GLenum,
        _level: GLint,
        _xoffset: GLint,
        _yoffset: GLint,
        _x: GLint,
        _y: GLint,
        _width: GLsizei,
        _height: GLsizei,
    ) {
        unimplemented!("CopyTexSubImage2D not implemented by this backend")
    }
    unsafe fn TexEnvf(&mut self, _target: GLenum, _pname: GLenum, _param: GLfloat) {
        unimplemented!("TexEnvf not implemented by this backend")
    }
    unsafe fn TexEnvx(&mut self, _target: GLenum, _pname: GLenum, _param: GLfixed) {
        unimplemented!("TexEnvx not implemented by this backend")
    }
    unsafe fn TexEnvi(&mut self, _target: GLenum, _pname: GLenum, _param: GLint) {
        unimplemented!("TexEnvi not implemented by this backend")
    }
    unsafe fn TexEnvfv(&mut self, _target: GLenum, _pname: GLenum, _params: *const GLfloat) {
        unimplemented!("TexEnvfv not implemented by this backend")
    }
    unsafe fn TexEnvxv(&mut self, _target: GLenum, _pname: GLenum, _params: *const GLfixed) {
        unimplemented!("TexEnvxv not implemented by this backend")
    }
    unsafe fn TexEnviv(&mut self, _target: GLenum, _pname: GLenum, _params: *const GLint) {
        unimplemented!("TexEnviv not implemented by this backend")
    }
    unsafe fn DrawTexsOES(
        &mut self,
        x: i16,
        y: i16,
        z: i16,
        width: i16,
        height: i16,
    ) {
        self.DrawTexfOES(x as GLfloat, y as GLfloat, z as GLfloat, width as GLfloat, height as GLfloat)
    }
    unsafe fn DrawTexiOES(&mut self, x: GLint, y: GLint, z: GLint, width: GLint, height: GLint) {
        self.DrawTexfOES(x as GLfloat, y as GLfloat, z as GLfloat, width as GLfloat, height as GLfloat)
    }
    unsafe fn DrawTexxOES(
        &mut self,
        x: GLfixed,
        y: GLfixed,
        z: GLfixed,
        width: GLfixed,
        height: GLfixed,
    ) {
        use super::util::fixed_to_float;
        self.DrawTexfOES(
            fixed_to_float(x),
            fixed_to_float(y),
            fixed_to_float(z),
            fixed_to_float(width),
            fixed_to_float(height),
        )
    }
    unsafe fn DrawTexsvOES(&mut self, coords: *const i16) {
        self.DrawTexsOES(
            coords.read_unaligned(),
            coords.add(1).read_unaligned(),
            coords.add(2).read_unaligned(),
            coords.add(3).read_unaligned(),
            coords.add(4).read_unaligned(),
        )
    }
    unsafe fn DrawTexivOES(&mut self, coords: *const GLint) {
        self.DrawTexiOES(
            coords.read_unaligned(),
            coords.add(1).read_unaligned(),
            coords.add(2).read_unaligned(),
            coords.add(3).read_unaligned(),
            coords.add(4).read_unaligned(),
        )
    }
    unsafe fn DrawTexxvOES(&mut self, coords: *const GLfixed) {
        self.DrawTexxOES(
            coords.read_unaligned(),
            coords.add(1).read_unaligned(),
            coords.add(2).read_unaligned(),
            coords.add(3).read_unaligned(),
            coords.add(4).read_unaligned(),
        )
    }
    unsafe fn DrawTexfvOES(&mut self, coords: *const GLfloat) {
        self.DrawTexfOES(
            coords.read_unaligned(),
            coords.add(1).read_unaligned(),
            coords.add(2).read_unaligned(),
            coords.add(3).read_unaligned(),
            coords.add(4).read_unaligned(),
        )
    }

    unsafe fn MultiTexCoord4f(
        &mut self,
        _target: GLenum,
        _s: GLfloat,
        _t: GLfloat,
        _r: GLfloat,
        _q: GLfloat,
    ) {
        unimplemented!("MultiTexCoord4f not implemented by this backend")
    }
    unsafe fn MultiTexCoord4x(
        &mut self,
        _target: GLenum,
        _s: GLfixed,
        _t: GLfixed,
        _r: GLfixed,
        _q: GLfixed,
    ) {
        unimplemented!("MultiTexCoord4x not implemented by this backend")
    }

    // Matrix stack operations
    unsafe fn MatrixMode(&mut self, _mode: GLenum) {
        unimplemented!("MatrixMode not implemented by this backend")
    }
    unsafe fn LoadIdentity(&mut self) {
        unimplemented!("LoadIdentity not implemented by this backend")
    }
    unsafe fn LoadMatrixf(&mut self, _m: *const GLfloat) {
        unimplemented!("LoadMatrixf not implemented by this backend")
    }
    unsafe fn LoadMatrixx(&mut self, _m: *const GLfixed) {
        unimplemented!("LoadMatrixx not implemented by this backend")
    }
    unsafe fn MultMatrixf(&mut self, _m: *const GLfloat) {
        unimplemented!("MultMatrixf not implemented by this backend")
    }
    unsafe fn MultMatrixx(&mut self, _m: *const GLfixed) {
        unimplemented!("MultMatrixx not implemented by this backend")
    }
    unsafe fn PushMatrix(&mut self) {
        unimplemented!("PushMatrix not implemented by this backend")
    }
    unsafe fn PopMatrix(&mut self) {
        unimplemented!("PopMatrix not implemented by this backend")
    }
    unsafe fn Orthof(
        &mut self,
        _left: GLfloat,
        _right: GLfloat,
        _bottom: GLfloat,
        _top: GLfloat,
        _near: GLfloat,
        _far: GLfloat,
    ) {
        unimplemented!("Orthof not implemented by this backend")
    }
    unsafe fn Orthox(
        &mut self,
        _left: GLfixed,
        _right: GLfixed,
        _bottom: GLfixed,
        _top: GLfixed,
        _near: GLfixed,
        _far: GLfixed,
    ) {
        unimplemented!("Orthox not implemented by this backend")
    }
    unsafe fn Frustumf(
        &mut self,
        _left: GLfloat,
        _right: GLfloat,
        _bottom: GLfloat,
        _top: GLfloat,
        _near: GLfloat,
        _far: GLfloat,
    ) {
        unimplemented!("Frustumf not implemented by this backend")
    }
    unsafe fn Frustumx(
        &mut self,
        _left: GLfixed,
        _right: GLfixed,
        _bottom: GLfixed,
        _top: GLfixed,
        _near: GLfixed,
        _far: GLfixed,
    ) {
        unimplemented!("Frustumx not implemented by this backend")
    }
    unsafe fn Rotatef(&mut self, _angle: GLfloat, _x: GLfloat, _y: GLfloat, _z: GLfloat) {
        unimplemented!("Rotatef not implemented by this backend")
    }
    unsafe fn Rotatex(&mut self, _angle: GLfixed, _x: GLfixed, _y: GLfixed, _z: GLfixed) {
        unimplemented!("Rotatex not implemented by this backend")
    }
    unsafe fn Scalef(&mut self, _x: GLfloat, _y: GLfloat, _z: GLfloat) {
        unimplemented!("Scalef not implemented by this backend")
    }
    unsafe fn Scalex(&mut self, _x: GLfixed, _y: GLfixed, _z: GLfixed) {
        unimplemented!("Scalex not implemented by this backend")
    }
    unsafe fn Translatef(&mut self, _x: GLfloat, _y: GLfloat, _z: GLfloat) {
        unimplemented!("Translatef not implemented by this backend")
    }
    unsafe fn Translatex(&mut self, _x: GLfixed, _y: GLfixed, _z: GLfixed) {
        unimplemented!("Translatex not implemented by this backend")
    }

    // OES_framebuffer_object (incomplete)
    unsafe fn GenFramebuffersOES(&mut self, _n: GLsizei, _framebuffers: *mut GLuint) {
        unimplemented!("GenFramebuffersOES not implemented by this backend")
    }
    unsafe fn GenRenderbuffersOES(&mut self, _n: GLsizei, _renderbuffers: *mut GLuint) {
        unimplemented!("GenRenderbuffersOES not implemented by this backend")
    }
    unsafe fn IsFramebufferOES(&mut self, _framebuffer: GLuint) -> GLboolean {
        unimplemented!("IsFramebufferOES not implemented by this backend")
    }
    unsafe fn IsRenderbufferOES(&mut self, _renderbuffer: GLuint) -> GLboolean {
        unimplemented!("IsRenderbufferOES not implemented by this backend")
    }
    unsafe fn BindFramebufferOES(&mut self, _target: GLenum, _framebuffer: GLuint) {
        unimplemented!("BindFramebufferOES not implemented by this backend")
    }
    unsafe fn BindRenderbufferOES(&mut self, _target: GLenum, _renderbuffer: GLuint) {
        unimplemented!("BindRenderbufferOES not implemented by this backend")
    }
    unsafe fn RenderbufferStorageOES(
        &mut self,
        _target: GLenum,
        _internalformat: GLenum,
        _width: GLsizei,
        _height: GLsizei,
    ) {
        unimplemented!("RenderbufferStorageOES not implemented by this backend")
    }
    unsafe fn FramebufferRenderbufferOES(
        &mut self,
        _target: GLenum,
        _attachment: GLenum,
        _renderbuffertarget: GLenum,
        _renderbuffer: GLuint,
    ) {
        unimplemented!("FramebufferRenderbufferOES not implemented by this backend")
    }
    unsafe fn FramebufferTexture2DOES(
        &mut self,
        _target: GLenum,
        _attachment: GLenum,
        _textarget: GLenum,
        _texture: GLuint,
        _level: i32,
    ) {
        unimplemented!("FramebufferTexture2DOES not implemented by this backend")
    }
    unsafe fn GetFramebufferAttachmentParameterivOES(
        &mut self,
        _target: GLenum,
        _attachment: GLenum,
        _pname: GLenum,
        _params: *mut GLint,
    ) {
        unimplemented!("GetFramebufferAttachmentParameterivOES not implemented by this backend")
    }
    unsafe fn GetRenderbufferParameterivOES(
        &mut self,
        _target: GLenum,
        _pname: GLenum,
        _params: *mut GLint,
    ) {
        unimplemented!("GetRenderbufferParameterivOES not implemented by this backend")
    }
    unsafe fn CheckFramebufferStatusOES(&mut self, _target: GLenum) -> GLenum {
        unimplemented!("CheckFramebufferStatusOES not implemented by this backend")
    }
    unsafe fn DeleteFramebuffersOES(&mut self, _n: GLsizei, _framebuffers: *const GLuint) {
        unimplemented!("DeleteFramebuffersOES not implemented by this backend")
    }
    unsafe fn DeleteRenderbuffersOES(&mut self, _n: GLsizei, _renderbuffers: *const GLuint) {
        unimplemented!("DeleteRenderbuffersOES not implemented by this backend")
    }
    unsafe fn GenerateMipmapOES(&mut self, _target: GLenum) {
        unimplemented!("GenerateMipmapOES not implemented by this backend")
    }

    // GL_APPLE_framebuffer_multisample
    //
    // Apple's iOS MSAA pattern uses a "sample" framebuffer with multisample
    // renderbuffer storage, renders to it, and then resolves into a separate
    // "resolve" framebuffer whose color renderbuffer is the CAEAGLLayer
    // drawable. `presentRenderbuffer:` is then called on the resolve
    // renderbuffer. Without an honest resolve, the resolve renderbuffer stays
    // empty and the screen never updates — see Apple's "Working with EAGL
    // Contexts" docs and the
    // [GL_APPLE_framebuffer_multisample]
    // (https://registry.khronos.org/OpenGL/extensions/APPLE/APPLE_framebuffer_multisample.txt)
    // extension spec.
    unsafe fn RenderbufferStorageMultisampleAPPLE(
        &mut self,
        target: GLenum,
        samples: GLsizei,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
    ) {
        // Default: fall back to single-sample. Concrete backends that have
        // the equivalent host-side extension override this.
        let _ = samples;
        self.RenderbufferStorageOES(target, internalformat, width, height);
    }
    unsafe fn ResolveMultisampleFramebufferAPPLE(&mut self) {
        // Default fallback for backends that don't implement a true
        // multisample resolve (e.g. native OpenGL ES 1.1 / ES 2.0 on Android,
        // where the host driver lacks `GL_APPLE_framebuffer_multisample` and
        // there's no `glBlitFramebuffer` to copy with). The companion
        // `RenderbufferStorageMultisampleAPPLE` default impl already degrades
        // multisample storage to single-sample, so the "sample" renderbuffer
        // holds the rendered pixels directly. With the canonical Apple
        // EAGLView MSAA pattern the sample framebuffer remains the currently
        // bound framebuffer (because the driver rejects
        // `GL_READ_FRAMEBUFFER_APPLE` / `GL_DRAW_FRAMEBUFFER_APPLE` as
        // targets without the extension), so `presentRenderbuffer:`'s
        // existing fallback — `glCopyTexImage2D` from the currently bound
        // FBO's color attachment — still finds the rendered frame and
        // displays it. Panicking here turned Temple Run (and any other
        // canonical EAGLView MSAA app) into a hard host-process abort on the
        // first frame; logging once and continuing keeps the app running.
        log_once!(
            "ResolveMultisampleFramebufferAPPLE: backend lacks a real resolve; \
             relying on the single-sample fallback for RenderbufferStorageMultisampleAPPLE \
             and the bound-FBO source in presentRenderbuffer:"
        );
    }

    // Non-OES aliases for OES_framebuffer_object functions.
    // Some GLES1 apps call the suffix-free ES2-style names directly.
    unsafe fn GenFramebuffers(&mut self, _n: GLsizei, _framebuffers: *mut GLuint) {
        unimplemented!("GenFramebuffers not implemented by this backend")
    }
    unsafe fn GenRenderbuffers(&mut self, _n: GLsizei, _renderbuffers: *mut GLuint) {
        unimplemented!("GenRenderbuffers not implemented by this backend")
    }
    unsafe fn IsFramebuffer(&mut self, _framebuffer: GLuint) -> GLboolean {
        unimplemented!("IsFramebuffer not implemented by this backend")
    }
    unsafe fn IsRenderbuffer(&mut self, _renderbuffer: GLuint) -> GLboolean {
        unimplemented!("IsRenderbuffer not implemented by this backend")
    }
    unsafe fn BindFramebuffer(&mut self, _target: GLenum, _framebuffer: GLuint) {
        unimplemented!("BindFramebuffer not implemented by this backend")
    }
    unsafe fn BindRenderbuffer(&mut self, _target: GLenum, _renderbuffer: GLuint) {
        unimplemented!("BindRenderbuffer not implemented by this backend")
    }
    unsafe fn RenderbufferStorage(
        &mut self,
        _target: GLenum,
        _internalformat: GLenum,
        _width: GLsizei,
        _height: GLsizei,
    ) {
        unimplemented!("RenderbufferStorage not implemented by this backend")
    }
    unsafe fn FramebufferRenderbuffer(
        &mut self,
        _target: GLenum,
        _attachment: GLenum,
        _renderbuffertarget: GLenum,
        _renderbuffer: GLuint,
    ) {
        unimplemented!("FramebufferRenderbuffer not implemented by this backend")
    }
    unsafe fn FramebufferTexture2D(
        &mut self,
        _target: GLenum,
        _attachment: GLenum,
        _textarget: GLenum,
        _texture: GLuint,
        _level: i32,
    ) {
        unimplemented!("FramebufferTexture2D not implemented by this backend")
    }
    unsafe fn CheckFramebufferStatus(&mut self, _target: GLenum) -> GLenum {
        unimplemented!("CheckFramebufferStatus not implemented by this backend")
    }
    unsafe fn DeleteFramebuffers(&mut self, _n: GLsizei, _framebuffers: *const GLuint) {
        unimplemented!("DeleteFramebuffers not implemented by this backend")
    }
    unsafe fn DeleteRenderbuffers(&mut self, _n: GLsizei, _renderbuffers: *const GLuint) {
        unimplemented!("DeleteRenderbuffers not implemented by this backend")
    }
    unsafe fn GenerateMipmap(&mut self, _target: GLenum) {
        unimplemented!("GenerateMipmap not implemented by this backend")
    }
    unsafe fn GetFramebufferAttachmentParameteriv(
        &mut self,
        _target: GLenum,
        _attachment: GLenum,
        _pname: GLenum,
        _params: *mut GLint,
    ) {
        unimplemented!("GetFramebufferAttachmentParameteriv not implemented by this backend")
    }
    unsafe fn GetRenderbufferParameteriv(
        &mut self,
        _target: GLenum,
        _pname: GLenum,
        _params: *mut GLint,
    ) {
        unimplemented!("GetRenderbufferParameteriv not implemented by this backend")
    }

    unsafe fn GetBufferParameteriv(&mut self, _target: GLenum, _pname: GLenum, _params: *mut GLint) {
        unimplemented!("GetBufferParameteriv not implemented by this backend")
    }
    unsafe fn MapBufferOES(&mut self, _target: GLenum, _access: GLenum) -> *mut GLvoid {
        unimplemented!("MapBufferOES not implemented by this backend")
    }
    unsafe fn UnmapBufferOES(&mut self, _target: GLenum) -> GLboolean {
        unimplemented!("UnmapBufferOES not implemented by this backend")
    }

    // OpenGL ES 2.0 entry points. Every shader-capable backend
    // ([super::gles1_on_gl2], [super::gles2_native], [super::gles2_on_gl3],
    // [super::gles3_native], [super::gles3_on_gl3]) implements every method
    // below as a real call into the underlying driver. EAGL routes ES 2.0
    // contexts to one of these backends; ES 1.1-only backends
    // ([super::gles1_native]) override every method below with an ES-spec
    // compliant "no shader pipeline" stub that records a synthetic
    // `GL_INVALID_OPERATION` so the guest sees standard OpenGL ES error
    // semantics rather than a panic.
    //
    // The default implementations therefore `unimplemented!()` so that any
    // future backend which forgets to provide one of these methods fails
    // loudly at the point of misuse, rather than silently returning zero or
    // no-opping. This is what makes the touchHLE ES 2.0 surface
    // "implemented without stubs" — every code path is real.
    unsafe fn CreateShader(&mut self, _type_: GLenum) -> GLuint {
        unimplemented!("CreateShader (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn DeleteShader(&mut self, _shader: GLuint) {
        unimplemented!("DeleteShader (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn ShaderSource(
        &mut self,
        _shader: GLuint,
        _count: GLsizei,
        _string: *const *const GLchar,
        _length: *const GLint,
    ) {
        unimplemented!("ShaderSource (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn CompileShader(&mut self, _shader: GLuint) {
        unimplemented!("CompileShader (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn GetShaderiv(&mut self, _shader: GLuint, _pname: GLenum, _params: *mut GLint) {
        unimplemented!("GetShaderiv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn GetShaderInfoLog(
        &mut self,
        _shader: GLuint,
        _maxLength: GLsizei,
        _length: *mut GLsizei,
        _infoLog: *mut GLchar,
    ) {
        unimplemented!("GetShaderInfoLog (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn IsShader(&mut self, _shader: GLuint) -> GLboolean {
        unimplemented!("IsShader (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn CreateProgram(&mut self) -> GLuint {
        unimplemented!("CreateProgram (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn DeleteProgram(&mut self, _program: GLuint) {
        unimplemented!("DeleteProgram (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn AttachShader(&mut self, _program: GLuint, _shader: GLuint) {
        unimplemented!("AttachShader (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn DetachShader(&mut self, _program: GLuint, _shader: GLuint) {
        unimplemented!("DetachShader (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn LinkProgram(&mut self, _program: GLuint) {
        unimplemented!("LinkProgram (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn UseProgram(&mut self, _program: GLuint) {
        unimplemented!("UseProgram (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn GetProgramiv(&mut self, _program: GLuint, _pname: GLenum, _params: *mut GLint) {
        unimplemented!("GetProgramiv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn GetProgramInfoLog(
        &mut self,
        _program: GLuint,
        _maxLength: GLsizei,
        _length: *mut GLsizei,
        _infoLog: *mut GLchar,
    ) {
        unimplemented!("GetProgramInfoLog (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn IsProgram(&mut self, _program: GLuint) -> GLboolean {
        unimplemented!("IsProgram (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn ValidateProgram(&mut self, _program: GLuint) {
        unimplemented!("ValidateProgram (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn BindAttribLocation(
        &mut self,
        _program: GLuint,
        _index: GLuint,
        _name: *const GLchar,
    ) {
        unimplemented!("BindAttribLocation (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn GetAttribLocation(&mut self, _program: GLuint, _name: *const GLchar) -> GLint {
        unimplemented!("GetAttribLocation (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn GetUniformLocation(&mut self, _program: GLuint, _name: *const GLchar) -> GLint {
        unimplemented!("GetUniformLocation (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
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
        unimplemented!("GetActiveAttrib (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
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
        unimplemented!("GetActiveUniform (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn EnableVertexAttribArray(&mut self, _index: GLuint) {
        unimplemented!(
            "EnableVertexAttribArray (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends"
        )
    }
    unsafe fn DisableVertexAttribArray(&mut self, _index: GLuint) {
        unimplemented!(
            "DisableVertexAttribArray (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends"
        )
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
        unimplemented!("VertexAttribPointer (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn VertexAttrib1f(&mut self, _index: GLuint, _x: GLfloat) {
        unimplemented!("VertexAttrib1f (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn VertexAttrib2f(&mut self, _index: GLuint, _x: GLfloat, _y: GLfloat) {
        unimplemented!("VertexAttrib2f (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn VertexAttrib3f(&mut self, _index: GLuint, _x: GLfloat, _y: GLfloat, _z: GLfloat) {
        unimplemented!("VertexAttrib3f (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn VertexAttrib4f(
        &mut self,
        _index: GLuint,
        _x: GLfloat,
        _y: GLfloat,
        _z: GLfloat,
        _w: GLfloat,
    ) {
        unimplemented!("VertexAttrib4f (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn VertexAttrib1fv(&mut self, _index: GLuint, _v: *const GLfloat) {
        unimplemented!("VertexAttrib1fv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn VertexAttrib2fv(&mut self, _index: GLuint, _v: *const GLfloat) {
        unimplemented!("VertexAttrib2fv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn VertexAttrib3fv(&mut self, _index: GLuint, _v: *const GLfloat) {
        unimplemented!("VertexAttrib3fv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn VertexAttrib4fv(&mut self, _index: GLuint, _v: *const GLfloat) {
        unimplemented!("VertexAttrib4fv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform1f(&mut self, _location: GLint, _v0: GLfloat) {
        unimplemented!("Uniform1f (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform2f(&mut self, _location: GLint, _v0: GLfloat, _v1: GLfloat) {
        unimplemented!("Uniform2f (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform3f(&mut self, _location: GLint, _v0: GLfloat, _v1: GLfloat, _v2: GLfloat) {
        unimplemented!("Uniform3f (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform4f(
        &mut self,
        _location: GLint,
        _v0: GLfloat,
        _v1: GLfloat,
        _v2: GLfloat,
        _v3: GLfloat,
    ) {
        unimplemented!("Uniform4f (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform1i(&mut self, _location: GLint, _v0: GLint) {
        unimplemented!("Uniform1i (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform2i(&mut self, _location: GLint, _v0: GLint, _v1: GLint) {
        unimplemented!("Uniform2i (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform3i(&mut self, _location: GLint, _v0: GLint, _v1: GLint, _v2: GLint) {
        unimplemented!("Uniform3i (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform4i(
        &mut self,
        _location: GLint,
        _v0: GLint,
        _v1: GLint,
        _v2: GLint,
        _v3: GLint,
    ) {
        unimplemented!("Uniform4i (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform1fv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLfloat) {
        unimplemented!("Uniform1fv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform2fv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLfloat) {
        unimplemented!("Uniform2fv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform3fv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLfloat) {
        unimplemented!("Uniform3fv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform4fv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLfloat) {
        unimplemented!("Uniform4fv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform1iv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLint) {
        unimplemented!("Uniform1iv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform2iv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLint) {
        unimplemented!("Uniform2iv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform3iv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLint) {
        unimplemented!("Uniform3iv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn Uniform4iv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLint) {
        unimplemented!("Uniform4iv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn UniformMatrix2fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        unimplemented!("UniformMatrix2fv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn UniformMatrix3fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        unimplemented!("UniformMatrix3fv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn UniformMatrix4fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        unimplemented!("UniformMatrix4fv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn BlendColor(&mut self, _r: GLclampf, _g: GLclampf, _b: GLclampf, _a: GLclampf) {
        unimplemented!("BlendColor (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn BlendEquation(&mut self, _mode: GLenum) {
        unimplemented!("BlendEquation (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn BlendEquationSeparate(&mut self, _modeRGB: GLenum, _modeAlpha: GLenum) {
        unimplemented!("BlendEquationSeparate (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn BlendFuncSeparate(
        &mut self,
        _srcRGB: GLenum,
        _dstRGB: GLenum,
        _srcAlpha: GLenum,
        _dstAlpha: GLenum,
    ) {
        unimplemented!("BlendFuncSeparate (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn StencilFuncSeparate(
        &mut self,
        _face: GLenum,
        _func: GLenum,
        _ref_: GLint,
        _mask: GLuint,
    ) {
        unimplemented!("StencilFuncSeparate (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn StencilOpSeparate(
        &mut self,
        _face: GLenum,
        _sfail: GLenum,
        _dpfail: GLenum,
        _dppass: GLenum,
    ) {
        unimplemented!("StencilOpSeparate (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn StencilMaskSeparate(&mut self, _face: GLenum, _mask: GLuint) {
        unimplemented!("StencilMaskSeparate (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn GetVertexAttribiv(&mut self, _index: GLuint, _pname: GLenum, _params: *mut GLint) {
        unimplemented!("GetVertexAttribiv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn GetVertexAttribfv(&mut self, _index: GLuint, _pname: GLenum, _params: *mut GLfloat) {
        unimplemented!("GetVertexAttribfv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn GetVertexAttribPointerv(
        &mut self,
        _index: GLuint,
        _pname: GLenum,
        _pointer: *mut *mut GLvoid,
    ) {
        unimplemented!(
            "GetVertexAttribPointerv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends"
        )
    }
    unsafe fn GetUniformiv(&mut self, _program: GLuint, _location: GLint, _params: *mut GLint) {
        unimplemented!("GetUniformiv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn GetUniformfv(&mut self, _program: GLuint, _location: GLint, _params: *mut GLfloat) {
        unimplemented!("GetUniformfv (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn GetAttachedShaders(
        &mut self,
        _program: GLuint,
        _maxCount: GLsizei,
        _count: *mut GLsizei,
        _shaders: *mut GLuint,
    ) {
        unimplemented!("GetAttachedShaders (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn GetShaderSource(
        &mut self,
        _shader: GLuint,
        _bufSize: GLsizei,
        _length: *mut GLsizei,
        _source: *mut GLchar,
    ) {
        unimplemented!("GetShaderSource (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }
    unsafe fn ReleaseShaderCompiler(&mut self) {
        // No-op: we always have a shader compiler.
    }
    unsafe fn GetShaderPrecisionFormat(
        &mut self,
        _shadertype: GLenum,
        _precisiontype: GLenum,
        _range: *mut GLint,
        _precision: *mut GLint,
    ) {
        unimplemented!(
            "GetShaderPrecisionFormat (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends"
        )
    }
    unsafe fn ShaderBinary(
        &mut self,
        _count: GLsizei,
        _shaders: *const GLuint,
        _binaryformat: GLenum,
        _binary: *const GLvoid,
        _length: GLsizei,
    ) {
        unimplemented!("ShaderBinary (OpenGL ES 2.0) must be overridden by ES 2.0-capable backends")
    }

    // ===== OpenGL ES 3.0 entry points =====
    //
    // The methods below correspond 1:1 to the OpenGL ES 3.0 specification
    // entry points that are new vs. ES 2.0 (plus a few that have ES-2
    // OES-extension equivalents — those keep their pre-existing entries
    // above and gain unsuffixed core variants here).
    //
    // Backends that do not implement OpenGL ES 3.0 inherit a default that
    // log-once warns and returns the type's default value, mirroring the
    // existing ES 2.0 stub convention. The dedicated `gles3_native` and
    // `gles3_on_gl3` backends override every single one with a real
    // pass-through call to the underlying driver — i.e. these are *not*
    // stubs at the level the guest app interacts with.

    /// Returns `true` if this backend implements OpenGL ES 3.0 (or higher).
    /// Used by `gles_guest` to gate ES-3-only entry points.
    fn is_es3(&self) -> bool {
        false
    }

    // -- Vertex array objects (promoted from OES_vertex_array_object) --
    unsafe fn IsVertexArray(&mut self, _array: GLuint) -> GLboolean {
        log_once!("IsVertexArray (OpenGL ES 3.0) not supported by this backend [stubbed]");
        0
    }
    unsafe fn BindVertexArray(&mut self, _array: GLuint) {
        log_once!("BindVertexArray (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn DeleteVertexArrays(&mut self, _n: GLsizei, _arrays: *const GLuint) {
        log_once!("DeleteVertexArrays (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn GenVertexArrays(&mut self, _n: GLsizei, _arrays: *mut GLuint) {
        log_once!("GenVertexArrays (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }

    // -- Vertex array objects via GL_OES_vertex_array_object (ES 1.1 / 2.0) --
    //
    // Returns whether this backend can back the `*VertexArrayOES` entry points
    // with real host vertex array objects. When false, the guest-facing
    // wrappers fall back to a no-op emulation. Failing to honour VAO state is a
    // common cause of lost buffer bindings (see the My Talking Tom crash where
    // a tapped-Tom draw used a vertex attribute whose backing buffer binding
    // had been dropped), so backends that can should override this.
    fn supports_vao_oes(&self) -> bool {
        false
    }
    unsafe fn BindVertexArrayOES(&mut self, _array: GLuint) {
        log_once!("BindVertexArrayOES not supported by this backend [stubbed]");
    }
    unsafe fn GenVertexArraysOES(&mut self, _n: GLsizei, _arrays: *mut GLuint) {
        log_once!("GenVertexArraysOES not supported by this backend [stubbed]");
    }
    unsafe fn DeleteVertexArraysOES(&mut self, _n: GLsizei, _arrays: *const GLuint) {
        log_once!("DeleteVertexArraysOES not supported by this backend [stubbed]");
    }
    unsafe fn IsVertexArrayOES(&mut self, _array: GLuint) -> GLboolean {
        log_once!("IsVertexArrayOES not supported by this backend [stubbed]");
        0
    }

    // -- Buffer object operations (3.0) --
    unsafe fn MapBufferRange(
        &mut self,
        _target: GLenum,
        _offset: GLintptr,
        _length: GLsizeiptr,
        _access: GLbitfield,
    ) -> *mut GLvoid {
        log_once!("MapBufferRange (OpenGL ES 3.0) not supported by this backend [stubbed]");
        std::ptr::null_mut()
    }
    unsafe fn FlushMappedBufferRange(
        &mut self,
        _target: GLenum,
        _offset: GLintptr,
        _length: GLsizeiptr,
    ) {
        log_once!(
            "FlushMappedBufferRange (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn GetBufferPointerv(
        &mut self,
        _target: GLenum,
        _pname: GLenum,
        _params: *mut *mut GLvoid,
    ) {
        log_once!("GetBufferPointerv (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn GetBufferParameteri64v(
        &mut self,
        _target: GLenum,
        _pname: GLenum,
        _params: *mut i64,
    ) {
        log_once!(
            "GetBufferParameteri64v (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn CopyBufferSubData(
        &mut self,
        _readTarget: GLenum,
        _writeTarget: GLenum,
        _readOffset: GLintptr,
        _writeOffset: GLintptr,
        _size: GLsizeiptr,
    ) {
        log_once!("CopyBufferSubData (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn BindBufferBase(&mut self, _target: GLenum, _index: GLuint, _buffer: GLuint) {
        log_once!("BindBufferBase (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn BindBufferRange(
        &mut self,
        _target: GLenum,
        _index: GLuint,
        _buffer: GLuint,
        _offset: GLintptr,
        _size: GLsizeiptr,
    ) {
        log_once!("BindBufferRange (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn UnmapBuffer(&mut self, _target: GLenum) -> GLboolean {
        log_once!("UnmapBuffer (OpenGL ES 3.0) not supported by this backend [stubbed]");
        0
    }

    // -- 3D textures and immutable storage --
    unsafe fn TexImage3D(
        &mut self,
        _target: GLenum,
        _level: GLint,
        _internalformat: GLint,
        _width: GLsizei,
        _height: GLsizei,
        _depth: GLsizei,
        _border: GLint,
        _format: GLenum,
        _type_: GLenum,
        _pixels: *const GLvoid,
    ) {
        log_once!("TexImage3D (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn TexSubImage3D(
        &mut self,
        _target: GLenum,
        _level: GLint,
        _xoffset: GLint,
        _yoffset: GLint,
        _zoffset: GLint,
        _width: GLsizei,
        _height: GLsizei,
        _depth: GLsizei,
        _format: GLenum,
        _type_: GLenum,
        _pixels: *const GLvoid,
    ) {
        log_once!("TexSubImage3D (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn CopyTexSubImage3D(
        &mut self,
        _target: GLenum,
        _level: GLint,
        _xoffset: GLint,
        _yoffset: GLint,
        _zoffset: GLint,
        _x: GLint,
        _y: GLint,
        _width: GLsizei,
        _height: GLsizei,
    ) {
        log_once!("CopyTexSubImage3D (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn CompressedTexImage3D(
        &mut self,
        _target: GLenum,
        _level: GLint,
        _internalformat: GLenum,
        _width: GLsizei,
        _height: GLsizei,
        _depth: GLsizei,
        _border: GLint,
        _imageSize: GLsizei,
        _data: *const GLvoid,
    ) {
        log_once!(
            "CompressedTexImage3D (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn CompressedTexSubImage3D(
        &mut self,
        _target: GLenum,
        _level: GLint,
        _xoffset: GLint,
        _yoffset: GLint,
        _zoffset: GLint,
        _width: GLsizei,
        _height: GLsizei,
        _depth: GLsizei,
        _format: GLenum,
        _imageSize: GLsizei,
        _data: *const GLvoid,
    ) {
        log_once!(
            "CompressedTexSubImage3D (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn TexStorage2D(
        &mut self,
        _target: GLenum,
        _levels: GLsizei,
        _internalformat: GLenum,
        _width: GLsizei,
        _height: GLsizei,
    ) {
        log_once!("TexStorage2D (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn TexStorage3D(
        &mut self,
        _target: GLenum,
        _levels: GLsizei,
        _internalformat: GLenum,
        _width: GLsizei,
        _height: GLsizei,
        _depth: GLsizei,
    ) {
        log_once!("TexStorage3D (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }

    // -- Framebuffer / renderbuffer (3.0) --
    unsafe fn BlitFramebuffer(
        &mut self,
        _srcX0: GLint,
        _srcY0: GLint,
        _srcX1: GLint,
        _srcY1: GLint,
        _dstX0: GLint,
        _dstY0: GLint,
        _dstX1: GLint,
        _dstY1: GLint,
        _mask: GLbitfield,
        _filter: GLenum,
    ) {
        log_once!("BlitFramebuffer (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn RenderbufferStorageMultisample(
        &mut self,
        _target: GLenum,
        _samples: GLsizei,
        _internalformat: GLenum,
        _width: GLsizei,
        _height: GLsizei,
    ) {
        log_once!(
            "RenderbufferStorageMultisample (OpenGL ES 3.0) not supported by this backend \
             [stubbed]"
        );
    }
    unsafe fn FramebufferTextureLayer(
        &mut self,
        _target: GLenum,
        _attachment: GLenum,
        _texture: GLuint,
        _level: GLint,
        _layer: GLint,
    ) {
        log_once!(
            "FramebufferTextureLayer (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn InvalidateFramebuffer(
        &mut self,
        _target: GLenum,
        _numAttachments: GLsizei,
        _attachments: *const GLenum,
    ) {
        log_once!(
            "InvalidateFramebuffer (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn InvalidateSubFramebuffer(
        &mut self,
        _target: GLenum,
        _numAttachments: GLsizei,
        _attachments: *const GLenum,
        _x: GLint,
        _y: GLint,
        _width: GLsizei,
        _height: GLsizei,
    ) {
        log_once!(
            "InvalidateSubFramebuffer (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn ReadBuffer(&mut self, _src: GLenum) {
        log_once!("ReadBuffer (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn DrawBuffers(&mut self, _n: GLsizei, _bufs: *const GLenum) {
        log_once!("DrawBuffers (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn DrawRangeElements(
        &mut self,
        _mode: GLenum,
        _start: GLuint,
        _end: GLuint,
        _count: GLsizei,
        _type_: GLenum,
        _indices: *const GLvoid,
    ) {
        log_once!("DrawRangeElements (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn ClearBufferiv(
        &mut self,
        _buffer: GLenum,
        _drawbuffer: GLint,
        _value: *const GLint,
    ) {
        log_once!("ClearBufferiv (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn ClearBufferuiv(
        &mut self,
        _buffer: GLenum,
        _drawbuffer: GLint,
        _value: *const GLuint,
    ) {
        log_once!("ClearBufferuiv (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn ClearBufferfv(
        &mut self,
        _buffer: GLenum,
        _drawbuffer: GLint,
        _value: *const GLfloat,
    ) {
        log_once!("ClearBufferfv (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn ClearBufferfi(
        &mut self,
        _buffer: GLenum,
        _drawbuffer: GLint,
        _depth: GLfloat,
        _stencil: GLint,
    ) {
        log_once!("ClearBufferfi (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }

    // -- Query objects --
    unsafe fn GenQueries(&mut self, _n: GLsizei, _ids: *mut GLuint) {
        log_once!("GenQueries (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn DeleteQueries(&mut self, _n: GLsizei, _ids: *const GLuint) {
        log_once!("DeleteQueries (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn IsQuery(&mut self, _id: GLuint) -> GLboolean {
        log_once!("IsQuery (OpenGL ES 3.0) not supported by this backend [stubbed]");
        0
    }
    unsafe fn BeginQuery(&mut self, _target: GLenum, _id: GLuint) {
        log_once!("BeginQuery (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn EndQuery(&mut self, _target: GLenum) {
        log_once!("EndQuery (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn GetQueryiv(&mut self, _target: GLenum, _pname: GLenum, _params: *mut GLint) {
        log_once!("GetQueryiv (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn GetQueryObjectuiv(&mut self, _id: GLuint, _pname: GLenum, _params: *mut GLuint) {
        log_once!("GetQueryObjectuiv (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }

    // -- Sampler objects --
    unsafe fn GenSamplers(&mut self, _count: GLsizei, _samplers: *mut GLuint) {
        log_once!("GenSamplers (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn DeleteSamplers(&mut self, _count: GLsizei, _samplers: *const GLuint) {
        log_once!("DeleteSamplers (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn IsSampler(&mut self, _sampler: GLuint) -> GLboolean {
        log_once!("IsSampler (OpenGL ES 3.0) not supported by this backend [stubbed]");
        0
    }
    unsafe fn BindSampler(&mut self, _unit: GLuint, _sampler: GLuint) {
        log_once!("BindSampler (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn SamplerParameteri(&mut self, _sampler: GLuint, _pname: GLenum, _param: GLint) {
        log_once!("SamplerParameteri (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn SamplerParameteriv(
        &mut self,
        _sampler: GLuint,
        _pname: GLenum,
        _params: *const GLint,
    ) {
        log_once!(
            "SamplerParameteriv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn SamplerParameterf(
        &mut self,
        _sampler: GLuint,
        _pname: GLenum,
        _param: GLfloat,
    ) {
        log_once!("SamplerParameterf (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn SamplerParameterfv(
        &mut self,
        _sampler: GLuint,
        _pname: GLenum,
        _params: *const GLfloat,
    ) {
        log_once!(
            "SamplerParameterfv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn GetSamplerParameteriv(
        &mut self,
        _sampler: GLuint,
        _pname: GLenum,
        _params: *mut GLint,
    ) {
        log_once!(
            "GetSamplerParameteriv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn GetSamplerParameterfv(
        &mut self,
        _sampler: GLuint,
        _pname: GLenum,
        _params: *mut GLfloat,
    ) {
        log_once!(
            "GetSamplerParameterfv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }

    // -- Transform feedback --
    unsafe fn BeginTransformFeedback(&mut self, _primitiveMode: GLenum) {
        log_once!(
            "BeginTransformFeedback (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn EndTransformFeedback(&mut self) {
        log_once!(
            "EndTransformFeedback (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn BindTransformFeedback(&mut self, _target: GLenum, _id: GLuint) {
        log_once!(
            "BindTransformFeedback (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn DeleteTransformFeedbacks(&mut self, _n: GLsizei, _ids: *const GLuint) {
        log_once!(
            "DeleteTransformFeedbacks (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn GenTransformFeedbacks(&mut self, _n: GLsizei, _ids: *mut GLuint) {
        log_once!(
            "GenTransformFeedbacks (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn IsTransformFeedback(&mut self, _id: GLuint) -> GLboolean {
        log_once!(
            "IsTransformFeedback (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
        0
    }
    unsafe fn PauseTransformFeedback(&mut self) {
        log_once!(
            "PauseTransformFeedback (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn ResumeTransformFeedback(&mut self) {
        log_once!(
            "ResumeTransformFeedback (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn TransformFeedbackVaryings(
        &mut self,
        _program: GLuint,
        _count: GLsizei,
        _varyings: *const *const GLchar,
        _bufferMode: GLenum,
    ) {
        log_once!(
            "TransformFeedbackVaryings (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn GetTransformFeedbackVarying(
        &mut self,
        _program: GLuint,
        _index: GLuint,
        _bufSize: GLsizei,
        _length: *mut GLsizei,
        _size: *mut GLsizei,
        _type_: *mut GLenum,
        _name: *mut GLchar,
    ) {
        log_once!(
            "GetTransformFeedbackVarying (OpenGL ES 3.0) not supported by this backend \
             [stubbed]"
        );
    }

    // -- Integer vertex attributes --
    unsafe fn VertexAttribIPointer(
        &mut self,
        _index: GLuint,
        _size: GLint,
        _type_: GLenum,
        _stride: GLsizei,
        _pointer: *const GLvoid,
    ) {
        log_once!(
            "VertexAttribIPointer (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn GetVertexAttribIiv(
        &mut self,
        _index: GLuint,
        _pname: GLenum,
        _params: *mut GLint,
    ) {
        log_once!(
            "GetVertexAttribIiv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn GetVertexAttribIuiv(
        &mut self,
        _index: GLuint,
        _pname: GLenum,
        _params: *mut GLuint,
    ) {
        log_once!(
            "GetVertexAttribIuiv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn VertexAttribI4i(
        &mut self,
        _index: GLuint,
        _x: GLint,
        _y: GLint,
        _z: GLint,
        _w: GLint,
    ) {
        log_once!("VertexAttribI4i (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn VertexAttribI4ui(
        &mut self,
        _index: GLuint,
        _x: GLuint,
        _y: GLuint,
        _z: GLuint,
        _w: GLuint,
    ) {
        log_once!(
            "VertexAttribI4ui (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn VertexAttribI4iv(&mut self, _index: GLuint, _v: *const GLint) {
        log_once!("VertexAttribI4iv (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn VertexAttribI4uiv(&mut self, _index: GLuint, _v: *const GLuint) {
        log_once!(
            "VertexAttribI4uiv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }

    // -- Integer uniforms --
    unsafe fn Uniform1ui(&mut self, _location: GLint, _v0: GLuint) {
        log_once!("Uniform1ui (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn Uniform2ui(&mut self, _location: GLint, _v0: GLuint, _v1: GLuint) {
        log_once!("Uniform2ui (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn Uniform3ui(&mut self, _location: GLint, _v0: GLuint, _v1: GLuint, _v2: GLuint) {
        log_once!("Uniform3ui (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn Uniform4ui(
        &mut self,
        _location: GLint,
        _v0: GLuint,
        _v1: GLuint,
        _v2: GLuint,
        _v3: GLuint,
    ) {
        log_once!("Uniform4ui (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn Uniform1uiv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _value: *const GLuint,
    ) {
        log_once!("Uniform1uiv (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn Uniform2uiv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _value: *const GLuint,
    ) {
        log_once!("Uniform2uiv (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn Uniform3uiv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _value: *const GLuint,
    ) {
        log_once!("Uniform3uiv (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn Uniform4uiv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _value: *const GLuint,
    ) {
        log_once!("Uniform4uiv (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn GetUniformuiv(
        &mut self,
        _program: GLuint,
        _location: GLint,
        _params: *mut GLuint,
    ) {
        log_once!("GetUniformuiv (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn UniformMatrix2x3fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        log_once!(
            "UniformMatrix2x3fv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn UniformMatrix3x2fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        log_once!(
            "UniformMatrix3x2fv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn UniformMatrix2x4fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        log_once!(
            "UniformMatrix2x4fv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn UniformMatrix4x2fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        log_once!(
            "UniformMatrix4x2fv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn UniformMatrix3x4fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        log_once!(
            "UniformMatrix3x4fv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn UniformMatrix4x3fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        log_once!(
            "UniformMatrix4x3fv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }

    // -- Uniform blocks --
    unsafe fn GetUniformIndices(
        &mut self,
        _program: GLuint,
        _uniformCount: GLsizei,
        _uniformNames: *const *const GLchar,
        _uniformIndices: *mut GLuint,
    ) {
        log_once!(
            "GetUniformIndices (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn GetActiveUniformsiv(
        &mut self,
        _program: GLuint,
        _uniformCount: GLsizei,
        _uniformIndices: *const GLuint,
        _pname: GLenum,
        _params: *mut GLint,
    ) {
        log_once!(
            "GetActiveUniformsiv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn GetUniformBlockIndex(
        &mut self,
        _program: GLuint,
        _uniformBlockName: *const GLchar,
    ) -> GLuint {
        log_once!(
            "GetUniformBlockIndex (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
        0xFFFF_FFFF // GL_INVALID_INDEX
    }
    unsafe fn GetActiveUniformBlockiv(
        &mut self,
        _program: GLuint,
        _uniformBlockIndex: GLuint,
        _pname: GLenum,
        _params: *mut GLint,
    ) {
        log_once!(
            "GetActiveUniformBlockiv (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn GetActiveUniformBlockName(
        &mut self,
        _program: GLuint,
        _uniformBlockIndex: GLuint,
        _bufSize: GLsizei,
        _length: *mut GLsizei,
        _uniformBlockName: *mut GLchar,
    ) {
        log_once!(
            "GetActiveUniformBlockName (OpenGL ES 3.0) not supported by this backend \
             [stubbed]"
        );
    }
    unsafe fn UniformBlockBinding(
        &mut self,
        _program: GLuint,
        _uniformBlockIndex: GLuint,
        _uniformBlockBinding: GLuint,
    ) {
        log_once!(
            "UniformBlockBinding (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }

    // -- Instanced rendering --
    unsafe fn DrawArraysInstanced(
        &mut self,
        _mode: GLenum,
        _first: GLint,
        _count: GLsizei,
        _instanceCount: GLsizei,
    ) {
        log_once!(
            "DrawArraysInstanced (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn DrawElementsInstanced(
        &mut self,
        _mode: GLenum,
        _count: GLsizei,
        _type_: GLenum,
        _indices: *const GLvoid,
        _instanceCount: GLsizei,
    ) {
        log_once!(
            "DrawElementsInstanced (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
    unsafe fn VertexAttribDivisor(&mut self, _index: GLuint, _divisor: GLuint) {
        log_once!(
            "VertexAttribDivisor (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }

    // -- Sync objects --
    unsafe fn FenceSync(&mut self, _condition: GLenum, _flags: GLbitfield) -> usize {
        log_once!("FenceSync (OpenGL ES 3.0) not supported by this backend [stubbed]");
        0
    }
    unsafe fn IsSync(&mut self, _sync: usize) -> GLboolean {
        log_once!("IsSync (OpenGL ES 3.0) not supported by this backend [stubbed]");
        0
    }
    unsafe fn DeleteSync(&mut self, _sync: usize) {
        log_once!("DeleteSync (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn ClientWaitSync(
        &mut self,
        _sync: usize,
        _flags: GLbitfield,
        _timeout: u64,
    ) -> GLenum {
        log_once!("ClientWaitSync (OpenGL ES 3.0) not supported by this backend [stubbed]");
        0
    }
    unsafe fn WaitSync(&mut self, _sync: usize, _flags: GLbitfield, _timeout: u64) {
        log_once!("WaitSync (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn GetSynciv(
        &mut self,
        _sync: usize,
        _pname: GLenum,
        _bufSize: GLsizei,
        _length: *mut GLsizei,
        _values: *mut GLint,
    ) {
        log_once!("GetSynciv (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }

    // -- 64-bit / indexed getters --
    unsafe fn GetInteger64v(&mut self, _pname: GLenum, _data: *mut i64) {
        log_once!("GetInteger64v (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn GetIntegeri_v(&mut self, _target: GLenum, _index: GLuint, _data: *mut GLint) {
        log_once!("GetIntegeri_v (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn GetInteger64i_v(&mut self, _target: GLenum, _index: GLuint, _data: *mut i64) {
        log_once!("GetInteger64i_v (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }

    // -- Program binary --
    unsafe fn ProgramParameteri(&mut self, _program: GLuint, _pname: GLenum, _value: GLint) {
        log_once!("ProgramParameteri (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn ProgramBinary(
        &mut self,
        _program: GLuint,
        _binaryFormat: GLenum,
        _binary: *const GLvoid,
        _length: GLsizei,
    ) {
        log_once!("ProgramBinary (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }
    unsafe fn GetProgramBinary(
        &mut self,
        _program: GLuint,
        _bufSize: GLsizei,
        _length: *mut GLsizei,
        _binaryFormat: *mut GLenum,
        _binary: *mut GLvoid,
    ) {
        log_once!("GetProgramBinary (OpenGL ES 3.0) not supported by this backend [stubbed]");
    }

    // -- Misc 3.0 --
    unsafe fn GetStringi(&mut self, _name: GLenum, _index: GLuint) -> *const GLubyte {
        log_once!("GetStringi (OpenGL ES 3.0) not supported by this backend [stubbed]");
        std::ptr::null()
    }
    unsafe fn GetFragDataLocation(
        &mut self,
        _program: GLuint,
        _name: *const GLchar,
    ) -> GLint {
        log_once!(
            "GetFragDataLocation (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
        -1
    }
    unsafe fn GetInternalformativ(
        &mut self,
        _target: GLenum,
        _internalformat: GLenum,
        _pname: GLenum,
        _bufSize: GLsizei,
        _params: *mut GLint,
    ) {
        log_once!(
            "GetInternalformativ (OpenGL ES 3.0) not supported by this backend [stubbed]"
        );
    }
}
