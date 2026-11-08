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
    unsafe fn driver_description(&self) -> String;
    // Generic state manipulation
    unsafe fn GetError(&mut self) -> GLenum;
    unsafe fn Enable(&mut self, cap: GLenum);
    unsafe fn IsEnabled(&mut self, cap: GLenum) -> GLboolean;
    unsafe fn Disable(&mut self, cap: GLenum);
    unsafe fn ClientActiveTexture(&mut self, texture: GLenum);
    unsafe fn EnableClientState(&mut self, array: GLenum);
    unsafe fn DisableClientState(&mut self, array: GLenum);
    unsafe fn GetBooleanv(&mut self, pname: GLenum, params: *mut GLboolean);
    unsafe fn GetFloatv(&mut self, pname: GLenum, params: *mut GLfloat);
    unsafe fn GetIntegerv(&mut self, pname: GLenum, params: *mut GLint);
    unsafe fn GetTexEnviv(&mut self, target: GLenum, pname: GLenum, params: *mut GLint);
    unsafe fn GetTexEnvfv(&mut self, target: GLenum, pname: GLenum, params: *mut GLfloat);
    unsafe fn GetPointerv(&mut self, pname: GLenum, params: *mut *const GLvoid);
    unsafe fn Hint(&mut self, target: GLenum, mode: GLenum);
    unsafe fn Finish(&mut self);
    unsafe fn Flush(&mut self);
    #[allow(dead_code)]
    unsafe fn GetString(&mut self, name: GLenum) -> *const GLubyte;

    // Other state manipulation
    unsafe fn AlphaFunc(&mut self, func: GLenum, ref_: GLclampf);
    unsafe fn AlphaFuncx(&mut self, func: GLenum, ref_: GLclampx);
    unsafe fn BlendFunc(&mut self, sfactor: GLenum, dfactor: GLenum);
    unsafe fn BlendEquationOES(&mut self, mode: GLenum);
    unsafe fn ColorMask(
        &mut self,
        red: GLboolean,
        green: GLboolean,
        blue: GLboolean,
        alpha: GLboolean,
    );
    unsafe fn ClipPlanef(&mut self, plane: GLenum, equation: *const GLfloat);
    unsafe fn ClipPlanex(&mut self, plane: GLenum, equation: *const GLfixed);
    unsafe fn CullFace(&mut self, mode: GLenum);
    unsafe fn DepthFunc(&mut self, func: GLenum);
    unsafe fn DepthMask(&mut self, flag: GLboolean);
    unsafe fn DepthRangef(&mut self, near: GLclampf, far: GLclampf);
    unsafe fn DepthRangex(&mut self, near: GLclampx, far: GLclampx);
    unsafe fn FrontFace(&mut self, mode: GLenum);
    unsafe fn PolygonOffset(&mut self, factor: GLfloat, units: GLfloat);
    unsafe fn PolygonOffsetx(&mut self, factor: GLfixed, units: GLfixed);
    unsafe fn SampleCoverage(&mut self, value: GLclampf, invert: GLboolean);
    unsafe fn SampleCoveragex(&mut self, value: GLclampx, invert: GLboolean);
    unsafe fn ShadeModel(&mut self, mode: GLenum);
    unsafe fn Scissor(&mut self, x: GLint, y: GLint, width: GLsizei, height: GLsizei);
    unsafe fn Viewport(&mut self, x: GLint, y: GLint, width: GLsizei, height: GLsizei);
    unsafe fn LineWidth(&mut self, val: GLfloat);
    unsafe fn LineWidthx(&mut self, val: GLfixed);
    unsafe fn StencilFunc(&mut self, func: GLenum, ref_: GLint, mask: GLuint);
    unsafe fn StencilOp(&mut self, sfail: GLenum, dpfail: GLenum, dppass: GLenum);
    unsafe fn StencilMask(&mut self, mask: GLuint);
    unsafe fn LogicOp(&mut self, opcode: GLenum);

    // Points
    unsafe fn PointSize(&mut self, size: GLfloat);
    unsafe fn PointSizex(&mut self, size: GLfixed);
    unsafe fn PointParameterf(&mut self, pname: GLenum, param: GLfloat);
    unsafe fn PointParameterx(&mut self, pname: GLenum, param: GLfixed);
    unsafe fn PointParameterfv(&mut self, pname: GLenum, params: *const GLfloat);
    unsafe fn PointParameterxv(&mut self, pname: GLenum, params: *const GLfixed);

    // Lighting and materials
    unsafe fn Fogf(&mut self, pname: GLenum, param: GLfloat);
    unsafe fn Fogx(&mut self, pname: GLenum, param: GLfixed);
    unsafe fn Fogfv(&mut self, pname: GLenum, params: *const GLfloat);
    unsafe fn Fogxv(&mut self, pname: GLenum, params: *const GLfixed);
    unsafe fn Lightf(&mut self, light: GLenum, pname: GLenum, param: GLfloat);
    unsafe fn Lightx(&mut self, light: GLenum, pname: GLenum, param: GLfixed);
    unsafe fn Lightfv(&mut self, light: GLenum, pname: GLenum, params: *const GLfloat);
    unsafe fn Lightxv(&mut self, light: GLenum, pname: GLenum, params: *const GLfixed);
    unsafe fn LightModelf(&mut self, pname: GLenum, param: GLfloat);
    unsafe fn LightModelx(&mut self, pname: GLenum, param: GLfixed);
    unsafe fn LightModelfv(&mut self, pname: GLenum, params: *const GLfloat);
    unsafe fn LightModelxv(&mut self, pname: GLenum, params: *const GLfixed);
    unsafe fn Materialf(&mut self, face: GLenum, pname: GLenum, param: GLfloat);
    unsafe fn Materialx(&mut self, face: GLenum, pname: GLenum, param: GLfixed);
    unsafe fn Materialfv(&mut self, face: GLenum, pname: GLenum, params: *const GLfloat);
    unsafe fn Materialxv(&mut self, face: GLenum, pname: GLenum, params: *const GLfixed);

    // Buffers
    unsafe fn IsBuffer(&mut self, buffer: GLuint) -> GLboolean;
    unsafe fn GenBuffers(&mut self, n: GLsizei, buffers: *mut GLuint);
    unsafe fn DeleteBuffers(&mut self, n: GLsizei, buffers: *const GLuint);
    unsafe fn BindBuffer(&mut self, target: GLenum, buffer: GLuint);
    unsafe fn BufferData(
        &mut self,
        target: GLenum,
        size: GLsizeiptr,
        data: *const GLvoid,
        usage: GLenum,
    );
    unsafe fn BufferSubData(
        &mut self,
        target: GLenum,
        offset: GLintptr,
        size: GLsizeiptr,
        data: *const GLvoid,
    );

    // Non-pointers
    unsafe fn Color4f(&mut self, red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat);
    unsafe fn Color4x(&mut self, red: GLfixed, green: GLfixed, blue: GLfixed, alpha: GLfixed);
    unsafe fn Color4ub(&mut self, red: GLubyte, green: GLubyte, blue: GLubyte, alpha: GLubyte);
    unsafe fn Normal3f(&mut self, nx: GLfloat, ny: GLfloat, nz: GLfloat);
    unsafe fn Normal3x(&mut self, nx: GLfixed, ny: GLfixed, nz: GLfixed);

    // Pointers
    unsafe fn ColorPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    );
    unsafe fn NormalPointer(&mut self, type_: GLenum, stride: GLsizei, pointer: *const GLvoid);
    unsafe fn TexCoordPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    );
    unsafe fn VertexPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    );

    // Drawing
    unsafe fn DrawArrays(&mut self, mode: GLenum, first: GLint, count: GLsizei);
    unsafe fn DrawElements(
        &mut self,
        mode: GLenum,
        count: GLsizei,
        type_: GLenum,
        indices: *const GLvoid,
    );

    // Clearing
    unsafe fn Clear(&mut self, mask: GLbitfield);
    unsafe fn ClearColor(
        &mut self,
        red: GLclampf,
        green: GLclampf,
        blue: GLclampf,
        alpha: GLclampf,
    );
    unsafe fn ClearColorx(
        &mut self,
        red: GLclampx,
        green: GLclampx,
        blue: GLclampx,
        alpha: GLclampx,
    );
    unsafe fn ClearDepthf(&mut self, depth: GLclampf);
    unsafe fn ClearDepthx(&mut self, depth: GLclampx);
    unsafe fn ClearStencil(&mut self, s: GLint);

    // Textures
    unsafe fn PixelStorei(&mut self, pname: GLenum, param: GLint);
    unsafe fn ReadPixels(
        &mut self,
        x: GLint,
        y: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        type_: GLenum,
        pixels: *mut GLvoid,
    );
    unsafe fn GenTextures(&mut self, n: GLsizei, textures: *mut GLuint);
    unsafe fn DeleteTextures(&mut self, n: GLsizei, textures: *const GLuint);
    unsafe fn ActiveTexture(&mut self, texture: GLenum);
    unsafe fn IsTexture(&mut self, texture: GLuint) -> GLboolean;
    unsafe fn BindTexture(&mut self, target: GLenum, texture: GLuint);
    unsafe fn TexParameteri(&mut self, target: GLenum, pname: GLenum, param: GLint);
    unsafe fn TexParameterf(&mut self, target: GLenum, pname: GLenum, param: GLfloat);
    unsafe fn TexParameterx(&mut self, target: GLenum, pname: GLenum, param: GLfixed);
    unsafe fn TexParameteriv(&mut self, target: GLenum, pname: GLenum, params: *const GLint);
    unsafe fn TexParameterfv(&mut self, target: GLenum, pname: GLenum, params: *const GLfloat);
    unsafe fn TexParameterxv(&mut self, target: GLenum, pname: GLenum, params: *const GLfixed);
    unsafe fn TexImage2D(
        &mut self,
        target: GLenum,
        level: GLint,
        internalformat: GLint,
        width: GLsizei,
        height: GLsizei,
        border: GLint,
        format: GLenum,
        type_: GLenum,
        pixels: *const GLvoid,
    );
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
    );
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
    );
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
    );
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
    );
    unsafe fn TexEnvf(&mut self, target: GLenum, pname: GLenum, param: GLfloat);
    unsafe fn TexEnvx(&mut self, target: GLenum, pname: GLenum, param: GLfixed);
    unsafe fn TexEnvi(&mut self, target: GLenum, pname: GLenum, param: GLint);
    unsafe fn TexEnvfv(&mut self, target: GLenum, pname: GLenum, params: *const GLfloat);
    unsafe fn TexEnvxv(&mut self, target: GLenum, pname: GLenum, params: *const GLfixed);
    unsafe fn TexEnviv(&mut self, target: GLenum, pname: GLenum, params: *const GLint);

    unsafe fn MultiTexCoord4f(
        &mut self,
        target: GLenum,
        s: GLfloat,
        t: GLfloat,
        r: GLfloat,
        q: GLfloat,
    );
    unsafe fn MultiTexCoord4x(
        &mut self,
        target: GLenum,
        s: GLfixed,
        t: GLfixed,
        r: GLfixed,
        q: GLfixed,
    );

    // Matrix stack operations
    unsafe fn MatrixMode(&mut self, mode: GLenum);
    unsafe fn LoadIdentity(&mut self);
    unsafe fn LoadMatrixf(&mut self, m: *const GLfloat);
    unsafe fn LoadMatrixx(&mut self, m: *const GLfixed);
    unsafe fn MultMatrixf(&mut self, m: *const GLfloat);
    unsafe fn MultMatrixx(&mut self, m: *const GLfixed);
    unsafe fn PushMatrix(&mut self);
    unsafe fn PopMatrix(&mut self);
    unsafe fn Orthof(
        &mut self,
        left: GLfloat,
        right: GLfloat,
        bottom: GLfloat,
        top: GLfloat,
        near: GLfloat,
        far: GLfloat,
    );
    unsafe fn Orthox(
        &mut self,
        left: GLfixed,
        right: GLfixed,
        bottom: GLfixed,
        top: GLfixed,
        near: GLfixed,
        far: GLfixed,
    );
    unsafe fn Frustumf(
        &mut self,
        left: GLfloat,
        right: GLfloat,
        bottom: GLfloat,
        top: GLfloat,
        near: GLfloat,
        far: GLfloat,
    );
    unsafe fn Frustumx(
        &mut self,
        left: GLfixed,
        right: GLfixed,
        bottom: GLfixed,
        top: GLfixed,
        near: GLfixed,
        far: GLfixed,
    );
    unsafe fn Rotatef(&mut self, angle: GLfloat, x: GLfloat, y: GLfloat, z: GLfloat);
    unsafe fn Rotatex(&mut self, angle: GLfixed, x: GLfixed, y: GLfixed, z: GLfixed);
    unsafe fn Scalef(&mut self, x: GLfloat, y: GLfloat, z: GLfloat);
    unsafe fn Scalex(&mut self, x: GLfixed, y: GLfixed, z: GLfixed);
    unsafe fn Translatef(&mut self, x: GLfloat, y: GLfloat, z: GLfloat);
    unsafe fn Translatex(&mut self, x: GLfixed, y: GLfixed, z: GLfixed);

    // OES_framebuffer_object (incomplete)
    unsafe fn GenFramebuffersOES(&mut self, n: GLsizei, framebuffers: *mut GLuint);
    unsafe fn GenRenderbuffersOES(&mut self, n: GLsizei, renderbuffers: *mut GLuint);
    unsafe fn IsFramebufferOES(&mut self, framebuffer: GLuint) -> GLboolean;
    unsafe fn IsRenderbufferOES(&mut self, renderbuffer: GLuint) -> GLboolean;
    unsafe fn BindFramebufferOES(&mut self, target: GLenum, framebuffer: GLuint);
    unsafe fn BindRenderbufferOES(&mut self, target: GLenum, renderbuffer: GLuint);
    unsafe fn RenderbufferStorageOES(
        &mut self,
        target: GLenum,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
    );
    unsafe fn FramebufferRenderbufferOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        renderbuffertarget: GLenum,
        renderbuffer: GLuint,
    );
    unsafe fn FramebufferTexture2DOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        textarget: GLenum,
        texture: GLuint,
        level: i32,
    );
    unsafe fn GetFramebufferAttachmentParameterivOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        pname: GLenum,
        params: *mut GLint,
    );
    unsafe fn GetRenderbufferParameterivOES(
        &mut self,
        target: GLenum,
        pname: GLenum,
        params: *mut GLint,
    );
    unsafe fn CheckFramebufferStatusOES(&mut self, target: GLenum) -> GLenum;
    unsafe fn DeleteFramebuffersOES(&mut self, n: GLsizei, framebuffers: *const GLuint);
    unsafe fn DeleteRenderbuffersOES(&mut self, n: GLsizei, renderbuffers: *const GLuint);
    unsafe fn GenerateMipmapOES(&mut self, target: GLenum);

    // Non-OES aliases for OES_framebuffer_object functions. OpenGL ES 2.0
    // (and some ES 1.1 apps) call the suffix-free names directly.
    unsafe fn GenFramebuffers(&mut self, n: GLsizei, framebuffers: *mut GLuint);
    unsafe fn GenRenderbuffers(&mut self, n: GLsizei, renderbuffers: *mut GLuint);
    unsafe fn IsFramebuffer(&mut self, framebuffer: GLuint) -> GLboolean;
    unsafe fn IsRenderbuffer(&mut self, renderbuffer: GLuint) -> GLboolean;
    unsafe fn BindFramebuffer(&mut self, target: GLenum, framebuffer: GLuint);
    unsafe fn BindRenderbuffer(&mut self, target: GLenum, renderbuffer: GLuint);
    unsafe fn RenderbufferStorage(
        &mut self,
        target: GLenum,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
    );
    unsafe fn FramebufferRenderbuffer(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        renderbuffertarget: GLenum,
        renderbuffer: GLuint,
    );
    unsafe fn FramebufferTexture2D(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        textarget: GLenum,
        texture: GLuint,
        level: i32,
    );
    unsafe fn CheckFramebufferStatus(&mut self, target: GLenum) -> GLenum;
    unsafe fn DeleteFramebuffers(&mut self, n: GLsizei, framebuffers: *const GLuint);
    unsafe fn DeleteRenderbuffers(&mut self, n: GLsizei, renderbuffers: *const GLuint);
    unsafe fn GenerateMipmap(&mut self, target: GLenum);
    unsafe fn GetFramebufferAttachmentParameteriv(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        pname: GLenum,
        params: *mut GLint,
    );
    unsafe fn GetRenderbufferParameteriv(
        &mut self,
        target: GLenum,
        pname: GLenum,
        params: *mut GLint,
    );

    unsafe fn GetBufferParameteriv(&mut self, target: GLenum, pname: GLenum, params: *mut GLint);
    unsafe fn MapBufferOES(&mut self, target: GLenum, access: GLenum) -> *mut GLvoid;
    unsafe fn UnmapBufferOES(&mut self, target: GLenum) -> GLboolean;

    // OpenGL ES 2.0 entry points. Default implementations panic — only
    // backends that actually support shaders (currently [super::gles1_on_gl2])
    // implement these. EAGL routes ES 2.0 contexts to such a backend.
    unsafe fn CreateShader(&mut self, _type_: GLenum) -> GLuint {
        unimplemented!("CreateShader (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn DeleteShader(&mut self, _shader: GLuint) {
        unimplemented!("DeleteShader (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn ShaderSource(
        &mut self,
        _shader: GLuint,
        _count: GLsizei,
        _string: *const *const GLchar,
        _length: *const GLint,
    ) {
        unimplemented!("ShaderSource (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn CompileShader(&mut self, _shader: GLuint) {
        unimplemented!("CompileShader (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn GetShaderiv(&mut self, _shader: GLuint, _pname: GLenum, _params: *mut GLint) {
        unimplemented!("GetShaderiv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn GetShaderInfoLog(
        &mut self,
        _shader: GLuint,
        _maxLength: GLsizei,
        _length: *mut GLsizei,
        _infoLog: *mut GLchar,
    ) {
        unimplemented!("GetShaderInfoLog (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn IsShader(&mut self, _shader: GLuint) -> GLboolean {
        unimplemented!("IsShader (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn CreateProgram(&mut self) -> GLuint {
        unimplemented!("CreateProgram (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn DeleteProgram(&mut self, _program: GLuint) {
        unimplemented!("DeleteProgram (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn AttachShader(&mut self, _program: GLuint, _shader: GLuint) {
        unimplemented!("AttachShader (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn DetachShader(&mut self, _program: GLuint, _shader: GLuint) {
        unimplemented!("DetachShader (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn LinkProgram(&mut self, _program: GLuint) {
        unimplemented!("LinkProgram (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn UseProgram(&mut self, _program: GLuint) {
        unimplemented!("UseProgram (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn GetProgramiv(&mut self, _program: GLuint, _pname: GLenum, _params: *mut GLint) {
        unimplemented!("GetProgramiv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn GetProgramInfoLog(
        &mut self,
        _program: GLuint,
        _maxLength: GLsizei,
        _length: *mut GLsizei,
        _infoLog: *mut GLchar,
    ) {
        unimplemented!("GetProgramInfoLog (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn IsProgram(&mut self, _program: GLuint) -> GLboolean {
        unimplemented!("IsProgram (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn ValidateProgram(&mut self, _program: GLuint) {
        unimplemented!("ValidateProgram (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn BindAttribLocation(
        &mut self,
        _program: GLuint,
        _index: GLuint,
        _name: *const GLchar,
    ) {
        unimplemented!("BindAttribLocation (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn GetAttribLocation(&mut self, _program: GLuint, _name: *const GLchar) -> GLint {
        unimplemented!("GetAttribLocation (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn GetUniformLocation(&mut self, _program: GLuint, _name: *const GLchar) -> GLint {
        unimplemented!("GetUniformLocation (OpenGL ES 2.0) not supported by this backend")
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
        unimplemented!("GetActiveAttrib (OpenGL ES 2.0) not supported by this backend")
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
        unimplemented!("GetActiveUniform (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn EnableVertexAttribArray(&mut self, _index: GLuint) {
        unimplemented!("EnableVertexAttribArray (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn DisableVertexAttribArray(&mut self, _index: GLuint) {
        unimplemented!("DisableVertexAttribArray (OpenGL ES 2.0) not supported by this backend")
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
        unimplemented!("VertexAttribPointer (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn VertexAttrib1f(&mut self, _index: GLuint, _x: GLfloat) {
        unimplemented!("VertexAttrib1f (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn VertexAttrib2f(&mut self, _index: GLuint, _x: GLfloat, _y: GLfloat) {
        unimplemented!("VertexAttrib2f (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn VertexAttrib3f(&mut self, _index: GLuint, _x: GLfloat, _y: GLfloat, _z: GLfloat) {
        unimplemented!("VertexAttrib3f (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn VertexAttrib4f(
        &mut self,
        _index: GLuint,
        _x: GLfloat,
        _y: GLfloat,
        _z: GLfloat,
        _w: GLfloat,
    ) {
        unimplemented!("VertexAttrib4f (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn VertexAttrib1fv(&mut self, _index: GLuint, _v: *const GLfloat) {
        unimplemented!("VertexAttrib1fv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn VertexAttrib2fv(&mut self, _index: GLuint, _v: *const GLfloat) {
        unimplemented!("VertexAttrib2fv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn VertexAttrib3fv(&mut self, _index: GLuint, _v: *const GLfloat) {
        unimplemented!("VertexAttrib3fv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn VertexAttrib4fv(&mut self, _index: GLuint, _v: *const GLfloat) {
        unimplemented!("VertexAttrib4fv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform1f(&mut self, _location: GLint, _v0: GLfloat) {
        unimplemented!("Uniform1f (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform2f(&mut self, _location: GLint, _v0: GLfloat, _v1: GLfloat) {
        unimplemented!("Uniform2f (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform3f(&mut self, _location: GLint, _v0: GLfloat, _v1: GLfloat, _v2: GLfloat) {
        unimplemented!("Uniform3f (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform4f(
        &mut self,
        _location: GLint,
        _v0: GLfloat,
        _v1: GLfloat,
        _v2: GLfloat,
        _v3: GLfloat,
    ) {
        unimplemented!("Uniform4f (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform1i(&mut self, _location: GLint, _v0: GLint) {
        unimplemented!("Uniform1i (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform2i(&mut self, _location: GLint, _v0: GLint, _v1: GLint) {
        unimplemented!("Uniform2i (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform3i(&mut self, _location: GLint, _v0: GLint, _v1: GLint, _v2: GLint) {
        unimplemented!("Uniform3i (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform4i(
        &mut self,
        _location: GLint,
        _v0: GLint,
        _v1: GLint,
        _v2: GLint,
        _v3: GLint,
    ) {
        unimplemented!("Uniform4i (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform1fv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLfloat) {
        unimplemented!("Uniform1fv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform2fv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLfloat) {
        unimplemented!("Uniform2fv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform3fv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLfloat) {
        unimplemented!("Uniform3fv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform4fv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLfloat) {
        unimplemented!("Uniform4fv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform1iv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLint) {
        unimplemented!("Uniform1iv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform2iv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLint) {
        unimplemented!("Uniform2iv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform3iv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLint) {
        unimplemented!("Uniform3iv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn Uniform4iv(&mut self, _location: GLint, _count: GLsizei, _value: *const GLint) {
        unimplemented!("Uniform4iv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn UniformMatrix2fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        unimplemented!("UniformMatrix2fv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn UniformMatrix3fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        unimplemented!("UniformMatrix3fv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn UniformMatrix4fv(
        &mut self,
        _location: GLint,
        _count: GLsizei,
        _transpose: GLboolean,
        _value: *const GLfloat,
    ) {
        unimplemented!("UniformMatrix4fv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn BlendColor(&mut self, _r: GLclampf, _g: GLclampf, _b: GLclampf, _a: GLclampf) {
        unimplemented!("BlendColor (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn BlendEquation(&mut self, _mode: GLenum) {
        unimplemented!("BlendEquation (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn BlendEquationSeparate(&mut self, _modeRGB: GLenum, _modeAlpha: GLenum) {
        unimplemented!("BlendEquationSeparate (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn BlendFuncSeparate(
        &mut self,
        _srcRGB: GLenum,
        _dstRGB: GLenum,
        _srcAlpha: GLenum,
        _dstAlpha: GLenum,
    ) {
        unimplemented!("BlendFuncSeparate (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn StencilFuncSeparate(
        &mut self,
        _face: GLenum,
        _func: GLenum,
        _ref_: GLint,
        _mask: GLuint,
    ) {
        unimplemented!("StencilFuncSeparate (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn StencilOpSeparate(
        &mut self,
        _face: GLenum,
        _sfail: GLenum,
        _dpfail: GLenum,
        _dppass: GLenum,
    ) {
        unimplemented!("StencilOpSeparate (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn StencilMaskSeparate(&mut self, _face: GLenum, _mask: GLuint) {
        unimplemented!("StencilMaskSeparate (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn GetVertexAttribiv(&mut self, _index: GLuint, _pname: GLenum, _params: *mut GLint) {
        unimplemented!("GetVertexAttribiv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn GetVertexAttribfv(&mut self, _index: GLuint, _pname: GLenum, _params: *mut GLfloat) {
        unimplemented!("GetVertexAttribfv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn GetVertexAttribPointerv(
        &mut self,
        _index: GLuint,
        _pname: GLenum,
        _pointer: *mut *mut GLvoid,
    ) {
        unimplemented!("GetVertexAttribPointerv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn GetUniformiv(&mut self, _program: GLuint, _location: GLint, _params: *mut GLint) {
        unimplemented!("GetUniformiv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn GetUniformfv(&mut self, _program: GLuint, _location: GLint, _params: *mut GLfloat) {
        unimplemented!("GetUniformfv (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn GetAttachedShaders(
        &mut self,
        _program: GLuint,
        _maxCount: GLsizei,
        _count: *mut GLsizei,
        _shaders: *mut GLuint,
    ) {
        unimplemented!("GetAttachedShaders (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn GetShaderSource(
        &mut self,
        _shader: GLuint,
        _bufSize: GLsizei,
        _length: *mut GLsizei,
        _source: *mut GLchar,
    ) {
        unimplemented!("GetShaderSource (OpenGL ES 2.0) not supported by this backend")
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
        unimplemented!("GetShaderPrecisionFormat (OpenGL ES 2.0) not supported by this backend")
    }
    unsafe fn ShaderBinary(
        &mut self,
        _count: GLsizei,
        _shaders: *const GLuint,
        _binaryformat: GLenum,
        _binary: *const GLvoid,
        _length: GLsizei,
    ) {
        unimplemented!("ShaderBinary (OpenGL ES 2.0) not supported by this backend")
    }
}
