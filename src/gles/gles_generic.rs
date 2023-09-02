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

use super::gles11_raw::types::*;

/// Trait representing an OpenGL ES implementation and context.
///
/// # Safety
/// It is the caller's responsibility to make the context active before using
/// any of the `unsafe` methods of this trait.
#[allow(clippy::upper_case_acronyms)]
#[allow(clippy::too_many_arguments)] // not our fault :(
pub trait GLES {
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
    fn make_current(&self, window: &crate::window::Window);

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
    unsafe fn ColorMask(
        &mut self,
        red: GLboolean,
        green: GLboolean,
        blue: GLboolean,
        alpha: GLboolean,
    );
    unsafe fn CullFace(&mut self, mode: GLenum);
    unsafe fn DepthFunc(&mut self, func: GLenum);
    unsafe fn DepthMask(&mut self, flag: GLboolean);
    unsafe fn DepthRangef(&mut self, near: GLclampf, far: GLclampf);
    unsafe fn DepthRangex(&mut self, near: GLclampx, far: GLclampx);
    unsafe fn FrontFace(&mut self, mode: GLenum);
    unsafe fn PolygonOffset(&mut self, factor: GLfloat, units: GLfloat);
    unsafe fn PolygonOffsetx(&mut self, factor: GLfixed, units: GLfixed);
    unsafe fn ShadeModel(&mut self, mode: GLenum);
    unsafe fn Scissor(&mut self, x: GLint, y: GLint, width: GLsizei, height: GLsizei);
    unsafe fn Viewport(&mut self, x: GLint, y: GLint, width: GLsizei, height: GLsizei);
    unsafe fn LineWidth(&mut self, val: GLfloat);
    unsafe fn LineWidthx(&mut self, val: GLfixed);
    unsafe fn StencilFunc(&mut self, func: GLenum, ref_: GLint, mask: GLuint);
    unsafe fn StencilOp(&mut self, sfail: GLenum, dpfail: GLenum, dppass: GLenum);
    unsafe fn StencilMask(&mut self, mask: GLuint);

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
    unsafe fn GetBufferParameteriv(&mut self, target: GLenum, pname: GLenum, params: *mut GLint);
    unsafe fn MapBufferOES(&mut self, target: GLenum, access: GLenum) -> *mut GLvoid;
    unsafe fn UnmapBufferOES(&mut self, target: GLenum) -> GLboolean;
}
