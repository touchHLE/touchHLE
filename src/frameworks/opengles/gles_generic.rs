//! Generic OpenGL ES 1.1 interface.
//!
//! Unfortunately this does not provide the types and constants, so the correct
//! usage is to import `GLES` and `types` from this module, but get the
//! constants from [crate::window::gles11].

use crate::window::gles11::types::*;

/// Trait representing an OpenGL ES implementation and context.
#[allow(clippy::upper_case_acronyms)]
pub trait GLES {
    fn new(window: &mut crate::window::Window) -> Self
    where
        Self: Sized;
    fn make_current(&self, window: &mut crate::window::Window);

    // Generic state manipulation
    unsafe fn GetError(&mut self) -> GLenum;
    unsafe fn GetIntegerv(&mut self, pname: GLenum, params: *mut GLint);

    // Textures
    unsafe fn GenTextures(&mut self, n: GLsizei, textures: *mut GLuint);
    unsafe fn BindTexture(&mut self, target: GLenum, texture: GLuint);
    unsafe fn TexParameteri(&mut self, target: GLenum, pname: GLenum, param: GLint);
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
    unsafe fn GetRenderbufferParameterivOES(
        &mut self,
        target: GLenum,
        pname: GLenum,
        params: *mut GLint,
    );
    unsafe fn CheckFramebufferStatusOES(&mut self, target: GLenum) -> GLenum;
}
