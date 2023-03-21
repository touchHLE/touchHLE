/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! OpenGL ES 1.1 proxy implementation for devices natively supporting it.
//! Currently, it is used for Android build.

use super::GLES;
use crate::window::gles11 as gl11;
use crate::window::gles11::types::*;
use crate::window::{GLContext, GLVersion, Window};

pub struct GLES1Native {
    gl_ctx: GLContext,
}
impl GLES for GLES1Native {
    fn new(window: &mut Window) -> Self {
        Self {
            gl_ctx: window.create_gl_context(GLVersion::GLES11),
        }
    }

    fn make_current(&self, window: &mut Window) {
        window.make_gl_context_current(&self.gl_ctx);
    }

    // Generic state manipulation
    unsafe fn GetError(&mut self) -> GLenum {
        gl11::GetError()
    }
    unsafe fn Enable(&mut self, cap: GLenum) {
        gl11::Enable(cap);
    }
    unsafe fn Disable(&mut self, cap: GLenum) {
        gl11::Disable(cap);
    }
    unsafe fn EnableClientState(&mut self, array: GLenum) {
        gl11::EnableClientState(array);
    }
    unsafe fn DisableClientState(&mut self, array: GLenum) {
        gl11::DisableClientState(array);
    }
    unsafe fn GetIntegerv(&mut self, pname: GLenum, params: *mut GLint) {
        gl11::GetIntegerv(pname, params);
    }
    unsafe fn Hint(&mut self, target: GLenum, mode: GLenum) {
        gl11::Hint(target, mode);
    }

    // Other state manipulation
    unsafe fn AlphaFunc(&mut self, func: GLenum, ref_: GLclampf) {
        gl11::AlphaFunc(func, ref_)
    }
    unsafe fn AlphaFuncx(&mut self, func: GLenum, ref_: GLclampx) {
        gl11::AlphaFuncx(func, ref_)
    }
    unsafe fn BlendFunc(&mut self, sfactor: GLenum, dfactor: GLenum) {
        gl11::BlendFunc(sfactor, dfactor)
    }
    unsafe fn CullFace(&mut self, mode: GLenum) {
        gl11::CullFace(mode)
    }
    unsafe fn DepthMask(&mut self, flag: GLboolean) {
        gl11::DepthMask(flag)
    }
    unsafe fn FrontFace(&mut self, mode: GLenum) {
        gl11::FrontFace(mode)
    }
    unsafe fn DepthRangef(&mut self, near: GLclampf, far: GLclampf) {
        gl11::DepthRangef(near.into(), far.into())
    }
    unsafe fn DepthRangex(&mut self, near: GLclampx, far: GLclampx) {
        gl11::DepthRangex(near.into(), far.into())
    }
    unsafe fn ShadeModel(&mut self, mode: GLenum) {
        gl11::ShadeModel(mode);
    }
    unsafe fn Scissor(&mut self, x: GLint, y: GLint, width: GLsizei, height: GLsizei) {
        gl11::Scissor(x, y, width, height)
    }
    unsafe fn Viewport(&mut self, x: GLint, y: GLint, width: GLsizei, height: GLsizei) {
        gl11::Viewport(x, y, width, height)
    }

    // Lighting
    unsafe fn Lightf(&mut self, light: GLenum, pname: GLenum, param: GLfloat) {
        gl11::Lightf(light, pname, param);
    }
    unsafe fn Lightx(&mut self, light: GLenum, pname: GLenum, param: GLfixed) {
        gl11::Lightx(light, pname, param);
    }
    unsafe fn Lightfv(&mut self, light: GLenum, pname: GLenum, params: *const GLfloat) {
        gl11::Lightfv(light, pname, params);
    }
    unsafe fn Lightxv(&mut self, light: GLenum, pname: GLenum, params: *const GLfixed) {
        gl11::Lightxv(light, pname, params);
    }

    // Buffers
    unsafe fn GenBuffers(&mut self, n: GLsizei, buffers: *mut GLuint) {
        gl11::GenBuffers(n, buffers)
    }
    unsafe fn DeleteBuffers(&mut self, n: GLsizei, buffers: *const GLuint) {
        gl11::DeleteBuffers(n, buffers)
    }
    unsafe fn BindBuffer(&mut self, target: GLenum, buffer: GLuint) {
        gl11::BindBuffer(target, buffer)
    }

    // Non-pointers
    unsafe fn Color4f(&mut self, red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat) {
        gl11::Color4f(red, green, blue, alpha)
    }
    unsafe fn Color4x(&mut self, red: GLfixed, green: GLfixed, blue: GLfixed, alpha: GLfixed) {
        gl11::Color4x(red, green, blue, alpha)
    }

    // Pointers
    unsafe fn ColorPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        gl11::ColorPointer(size, type_, stride, pointer)
    }
    unsafe fn NormalPointer(&mut self, type_: GLenum, stride: GLsizei, pointer: *const GLvoid) {
        gl11::NormalPointer(type_, stride, pointer)
    }
    unsafe fn TexCoordPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        gl11::TexCoordPointer(size, type_, stride, pointer)
    }
    unsafe fn VertexPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        gl11::VertexPointer(size, type_, stride, pointer)
    }
    unsafe fn DrawArrays(&mut self, mode: GLenum, first: GLint, count: GLsizei) {
        gl11::DrawArrays(mode, first, count)
    }
    unsafe fn DrawElements(
        &mut self,
        mode: GLenum,
        count: GLsizei,
        type_: GLenum,
        indices: *const GLvoid,
    ) {
        gl11::DrawElements(mode, count, type_, indices)
    }
    unsafe fn Clear(&mut self, mask: GLbitfield) {
        gl11::Clear(mask)
    }
    unsafe fn ClearColor(
        &mut self,
        red: GLclampf,
        green: GLclampf,
        blue: GLclampf,
        alpha: GLclampf,
    ) {
        gl11::ClearColor(red, green, blue, alpha)
    }
    unsafe fn ClearColorx(
        &mut self,
        red: GLclampx,
        green: GLclampx,
        blue: GLclampx,
        alpha: GLclampx,
    ) {
        gl11::ClearColorx(red, green, blue, alpha)
    }
    unsafe fn ClearDepthf(&mut self, depth: GLclampf) {
        gl11::ClearDepthf(depth)
    }
    unsafe fn ClearDepthx(&mut self, depth: GLclampx) {
        gl11::ClearDepthx(depth)
    }
    unsafe fn ClearStencil(&mut self, s: GLint) {
        gl11::ClearStencil(s)
    }

    // Textures
    unsafe fn GenTextures(&mut self, n: GLsizei, textures: *mut GLuint) {
        gl11::GenTextures(n, textures)
    }
    unsafe fn DeleteTextures(&mut self, n: GLsizei, textures: *const GLuint) {
        gl11::DeleteTextures(n, textures)
    }
    unsafe fn BindTexture(&mut self, target: GLenum, texture: GLuint) {
        gl11::BindTexture(target, texture)
    }
    unsafe fn TexParameteri(&mut self, target: GLenum, pname: GLenum, param: GLint) {
        gl11::TexParameteri(target, pname, param);
    }
    unsafe fn TexParameterf(&mut self, target: GLenum, pname: GLenum, param: GLfloat) {
        gl11::TexParameterf(target, pname, param);
    }
    unsafe fn TexParameterx(&mut self, target: GLenum, pname: GLenum, param: GLfixed) {
        gl11::TexParameterx(target, pname, param);
    }
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
    ) {
        gl11::TexImage2D(
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
    unsafe fn TexEnvf(&mut self, target: GLenum, pname: GLenum, param: GLfloat) {
        gl11::TexEnvf(target, pname, param)
    }
    unsafe fn TexEnvx(&mut self, target: GLenum, pname: GLenum, param: GLfixed) {
        gl11::TexEnvx(target, pname, param)
    }
    unsafe fn TexEnvi(&mut self, target: GLenum, pname: GLenum, param: GLint) {
        gl11::TexEnvi(target, pname, param)
    }
    unsafe fn TexEnvfv(&mut self, target: GLenum, pname: GLenum, params: *const GLfloat) {
        gl11::TexEnvfv(target, pname, params)
    }
    unsafe fn TexEnvxv(&mut self, target: GLenum, pname: GLenum, params: *const GLfixed) {
        gl11::TexEnvxv(target, pname, params)
    }
    unsafe fn TexEnviv(&mut self, target: GLenum, pname: GLenum, params: *const GLint) {
        gl11::TexEnviv(target, pname, params)
    }

    // Matrix stack operations
    unsafe fn MatrixMode(&mut self, mode: GLenum) {
        gl11::MatrixMode(mode)
    }
    unsafe fn LoadIdentity(&mut self) {
        gl11::LoadIdentity()
    }
    unsafe fn LoadMatrixf(&mut self, m: *const GLfloat) {
        gl11::LoadMatrixf(m)
    }
    unsafe fn LoadMatrixx(&mut self, m: *const GLfixed) {
        gl11::LoadMatrixx(m)
    }
    unsafe fn MultMatrixf(&mut self, m: *const GLfloat) {
        gl11::MultMatrixf(m)
    }
    unsafe fn MultMatrixx(&mut self, m: *const GLfixed) {
        gl11::MultMatrixx(m)
    }
    unsafe fn PushMatrix(&mut self) {
        gl11::PushMatrix()
    }
    unsafe fn PopMatrix(&mut self) {
        gl11::PopMatrix()
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
        gl11::Orthof(
            left.into(),
            right.into(),
            bottom.into(),
            top.into(),
            near.into(),
            far.into(),
        )
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
        gl11::Orthox(
            left.into(),
            right.into(),
            bottom.into(),
            top.into(),
            near.into(),
            far.into(),
        )
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
        gl11::Frustumf(
            left.into(),
            right.into(),
            bottom.into(),
            top.into(),
            near.into(),
            far.into(),
        )
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
        gl11::Frustumx(
            left.into(),
            right.into(),
            bottom.into(),
            top.into(),
            near.into(),
            far.into(),
        )
    }
    unsafe fn Rotatef(&mut self, angle: GLfloat, x: GLfloat, y: GLfloat, z: GLfloat) {
        gl11::Rotatef(angle, x, y, z)
    }
    unsafe fn Rotatex(&mut self, angle: GLfixed, x: GLfixed, y: GLfixed, z: GLfixed) {
        gl11::Rotatex(angle, x, y, z)
    }
    unsafe fn Scalef(&mut self, x: GLfloat, y: GLfloat, z: GLfloat) {
        gl11::Scalef(x, y, z)
    }
    unsafe fn Scalex(&mut self, x: GLfixed, y: GLfixed, z: GLfixed) {
        gl11::Scalex(x, y, z)
    }
    unsafe fn Translatef(&mut self, x: GLfloat, y: GLfloat, z: GLfloat) {
        gl11::Translatef(x, y, z)
    }
    unsafe fn Translatex(&mut self, x: GLfixed, y: GLfixed, z: GLfixed) {
        gl11::Translatex(x, y, z)
    }

    // OES_framebuffer_object
    unsafe fn GenFramebuffersOES(&mut self, n: GLsizei, framebuffers: *mut GLuint) {
        gl11::GenFramebuffersOES(n, framebuffers)
    }
    unsafe fn GenRenderbuffersOES(&mut self, n: GLsizei, renderbuffers: *mut GLuint) {
        gl11::GenRenderbuffersOES(n, renderbuffers)
    }
    unsafe fn BindFramebufferOES(&mut self, target: GLenum, framebuffer: GLuint) {
        gl11::BindFramebufferOES(target, framebuffer)
    }
    unsafe fn BindRenderbufferOES(&mut self, target: GLenum, renderbuffer: GLuint) {
        gl11::BindRenderbufferOES(target, renderbuffer)
    }
    unsafe fn RenderbufferStorageOES(
        &mut self,
        target: GLenum,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
    ) {
        gl11::RenderbufferStorageOES(target, internalformat, width, height)
    }
    unsafe fn FramebufferRenderbufferOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        renderbuffertarget: GLenum,
        renderbuffer: GLuint,
    ) {
        gl11::FramebufferRenderbufferOES(target, attachment, renderbuffertarget, renderbuffer)
    }
    unsafe fn GetRenderbufferParameterivOES(
        &mut self,
        target: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gl11::GetRenderbufferParameterivOES(target, pname, params)
    }
    unsafe fn CheckFramebufferStatusOES(&mut self, target: GLenum) -> GLenum {
        gl11::CheckFramebufferStatusOES(target)
    }
    unsafe fn DeleteFramebuffersOES(&mut self, n: GLsizei, framebuffers: *mut GLuint) {
        gl11::DeleteFramebuffersOES(n, framebuffers)
    }
    unsafe fn DeleteRenderbuffersOES(&mut self, n: GLsizei, renderbuffers: *mut GLuint) {
        gl11::DeleteRenderbuffersOES(n, renderbuffers)
    }
}
