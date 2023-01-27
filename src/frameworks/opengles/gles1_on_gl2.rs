//! Implementation of OpenGL ES 1.1 on top of OpenGL 2.1 compatibility profile.
//!
//! The standard graphics drivers on most desktop operating systems do not
//! provide OpenGL ES 1.1, so we must provide it ourselves somehow.
//!
//! OpenGL ES 1.1 is based on OpenGL 1.5. Much of its core functionality (e.g.
//! the fixed-function pipeline) is considered legacy and not available in the
//! "core profile" for modern OpenGL versions, nor is it available at all in
//! later versions of OpenGL ES. However, OpenGL also has the "compatibility
//! profile" which still offers this legacy functionality.
//!
//! OpenGL 2.1 is the latest version that has a compatibility profile available
//! on macOS. It's also a version supported on various other OSes.
//! It is therefore a convenient target for our implementation.

use super::GLES;
use crate::window::gl21compat as gl21;
use crate::window::gl21compat::types::*;
use crate::window::gles11;
use crate::window::{GLContext, GLVersion, Window};

fn fixed_to_float(fixed: gles11::types::GLfixed) -> GLfloat {
    ((fixed as f64) / ((1 << 16) as f64)) as f32
}

unsafe fn matrix_fixed_to_float(m: *const gles11::types::GLfixed) -> [GLfloat; 16] {
    let mut matrix = [0f32; 16];
    for (i, cell) in matrix.iter_mut().enumerate() {
        *cell = fixed_to_float(*m.add(i));
    }
    matrix
}

/// List of capabilities shared by OpenGL ES 1.1 and OpenGL 2.1.
///
/// Note: There can be arbitrarily many lights or clip planes, depending on
/// implementation limits. We might eventually need to check those rather than
/// just providing the minimum.
///
/// TODO: GL_POINT_SPRITE_OES?
pub(super) const CAPABILITIES: &[GLenum] = &[
    gl21::ALPHA_TEST,
    gl21::BLEND,
    gl21::COLOR_LOGIC_OP,
    gl21::CLIP_PLANE0,
    gl21::LIGHT0,
    gl21::LIGHT1,
    gl21::LIGHT2,
    gl21::LIGHT3,
    gl21::LIGHT4,
    gl21::LIGHT5,
    gl21::LIGHT6,
    gl21::LIGHT7,
    gl21::COLOR_MATERIAL,
    gl21::CULL_FACE,
    gl21::DEPTH_TEST,
    gl21::DITHER,
    gl21::FOG,
    gl21::LIGHTING,
    gl21::LINE_SMOOTH,
    gl21::MULTISAMPLE,
    gl21::NORMALIZE,
    gl21::POINT_SMOOTH,
    gl21::POLYGON_OFFSET_FILL,
    gl21::RESCALE_NORMAL,
    gl21::SAMPLE_ALPHA_TO_COVERAGE,
    gl21::SAMPLE_ALPHA_TO_ONE,
    gl21::SAMPLE_COVERAGE,
    gl21::SCISSOR_TEST,
    gl21::STENCIL_TEST,
    gl21::TEXTURE_2D,
];

/// List of client-side capabilities shared by OpenGL ES 1.1 and OpenGL 2.1.
///
/// TODO: GL_POINT_SIZE_ARRAY_OES?
pub(super) const CLIENT_CAPABILITIES: &[GLenum] = &[
    gl21::COLOR_ARRAY,
    gl21::NORMAL_ARRAY,
    gl21::TEXTURE_COORD_ARRAY,
    gl21::VERTEX_ARRAY,
];

pub struct GLES1OnGL2 {
    gl_ctx: GLContext,
}
impl GLES for GLES1OnGL2 {
    fn new(window: &mut Window) -> Self {
        Self {
            gl_ctx: window.create_gl_context(GLVersion::GL21Compat),
        }
    }

    fn make_current(&self, window: &mut Window) {
        window.make_gl_context_current(&self.gl_ctx);
    }

    // Generic state manipulation
    unsafe fn GetError(&mut self) -> GLenum {
        gl21::GetError()
    }
    unsafe fn Enable(&mut self, cap: GLenum) {
        assert!(CAPABILITIES.contains(&cap));
        gl21::Enable(cap);
    }
    unsafe fn Disable(&mut self, cap: GLenum) {
        assert!(CAPABILITIES.contains(&cap));
        gl21::Disable(cap);
    }
    unsafe fn EnableClientState(&mut self, array: GLenum) {
        assert!(CLIENT_CAPABILITIES.contains(&array));
        gl21::EnableClientState(array);
    }
    unsafe fn DisableClientState(&mut self, array: GLenum) {
        assert!(CLIENT_CAPABILITIES.contains(&array));
        gl21::DisableClientState(array);
    }
    unsafe fn GetIntegerv(&mut self, pname: GLenum, params: *mut GLint) {
        // This function family can return a huge number of things.
        // TODO: support more possible values.
        assert!([
            gl21::ARRAY_BUFFER_BINDING,
            gl21::ELEMENT_ARRAY_BUFFER_BINDING,
            gl21::MATRIX_MODE,
            gl21::TEXTURE_BINDING_2D
        ]
        .contains(&pname));
        gl21::GetIntegerv(pname, params);
    }

    // Other state manipulation
    unsafe fn AlphaFunc(&mut self, func: GLenum, ref_: GLclampf) {
        assert!([
            gl21::NEVER,
            gl21::LESS,
            gl21::EQUAL,
            gl21::LEQUAL,
            gl21::GREATER,
            gl21::NOTEQUAL,
            gl21::GEQUAL,
            gl21::ALWAYS
        ]
        .contains(&func));
        gl21::AlphaFunc(func, ref_)
    }
    unsafe fn AlphaFuncx(&mut self, func: GLenum, ref_: GLclampx) {
        self.AlphaFunc(func, fixed_to_float(ref_))
    }
    unsafe fn BlendFunc(&mut self, sfactor: GLenum, dfactor: GLenum) {
        assert!([
            gl21::ZERO,
            gl21::ONE,
            gl21::DST_COLOR,
            gl21::ONE_MINUS_DST_COLOR,
            gl21::SRC_ALPHA,
            gl21::ONE_MINUS_SRC_ALPHA,
            gl21::DST_ALPHA,
            gl21::ONE_MINUS_DST_ALPHA,
            gl21::SRC_ALPHA_SATURATE
        ]
        .contains(&sfactor));
        assert!([
            gl21::ZERO,
            gl21::ONE,
            gl21::SRC_COLOR,
            gl21::ONE_MINUS_SRC_COLOR,
            gl21::SRC_ALPHA,
            gl21::ONE_MINUS_SRC_ALPHA,
            gl21::DST_ALPHA,
            gl21::ONE_MINUS_DST_ALPHA
        ]
        .contains(&dfactor));
        gl21::BlendFunc(sfactor, dfactor);
    }
    unsafe fn ShadeModel(&mut self, mode: GLenum) {
        assert!(mode == gl21::FLAT || mode == gl21::SMOOTH);
        gl21::ShadeModel(mode);
    }
    unsafe fn Scissor(&mut self, x: GLint, y: GLint, width: GLsizei, height: GLsizei) {
        gl21::Scissor(x, y, width, height)
    }
    unsafe fn Viewport(&mut self, x: GLint, y: GLint, width: GLsizei, height: GLsizei) {
        gl21::Viewport(x, y, width, height)
    }

    // Pointers
    unsafe fn ColorPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        assert!(size == 4);
        // TODO: fixed-point
        assert!(type_ == gl21::UNSIGNED_BYTE || type_ == gl21::FLOAT);
        gl21::ColorPointer(size, type_, stride, pointer)
    }
    unsafe fn NormalPointer(&mut self, type_: GLenum, stride: GLsizei, pointer: *const GLvoid) {
        // TODO: fixed-point
        assert!(type_ == gl21::BYTE || type_ == gl21::SHORT || type_ == gl21::FLOAT);
        gl21::NormalPointer(type_, stride, pointer)
    }
    unsafe fn TexCoordPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        assert!(size == 2 || size == 3 || size == 4);
        // TODO: byte and fixed-point
        assert!(type_ == gl21::SHORT || type_ == gl21::FLOAT);
        gl21::TexCoordPointer(size, type_, stride, pointer)
    }
    unsafe fn VertexPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        assert!(size == 2 || size == 3 || size == 4);
        // TODO: byte and fixed-point
        assert!(type_ == gl21::SHORT || type_ == gl21::FLOAT);
        gl21::VertexPointer(size, type_, stride, pointer)
    }

    // Drawing
    unsafe fn DrawArrays(&mut self, mode: GLenum, first: GLint, count: GLsizei) {
        assert!([
            gl21::POINTS,
            gl21::LINE_STRIP,
            gl21::LINE_LOOP,
            gl21::LINES,
            gl21::TRIANGLE_STRIP,
            gl21::TRIANGLE_FAN,
            gl21::TRIANGLES
        ]
        .contains(&mode));
        gl21::DrawArrays(mode, first, count);
    }
    unsafe fn DrawElements(
        &mut self,
        mode: GLenum,
        count: GLsizei,
        type_: GLenum,
        indices: *const GLvoid,
    ) {
        assert!([
            gl21::POINTS,
            gl21::LINE_STRIP,
            gl21::LINE_LOOP,
            gl21::LINES,
            gl21::TRIANGLE_STRIP,
            gl21::TRIANGLE_FAN,
            gl21::TRIANGLES
        ]
        .contains(&mode));
        assert!(type_ == gl21::UNSIGNED_BYTE || type_ == gl21::UNSIGNED_SHORT);
        gl21::DrawElements(mode, count, type_, indices);
    }

    // Clearing
    unsafe fn Clear(&mut self, mask: GLbitfield) {
        assert!(
            mask & !(gl21::COLOR_BUFFER_BIT | gl21::DEPTH_BUFFER_BIT | gl21::STENCIL_BUFFER_BIT)
                == 0
        );
        gl21::Clear(mask)
    }
    unsafe fn ClearColor(
        &mut self,
        red: GLclampf,
        green: GLclampf,
        blue: GLclampf,
        alpha: GLclampf,
    ) {
        gl21::ClearColor(red, green, blue, alpha)
    }
    unsafe fn ClearColorx(
        &mut self,
        red: GLclampx,
        green: GLclampx,
        blue: GLclampx,
        alpha: GLclampx,
    ) {
        gl21::ClearColor(
            fixed_to_float(red),
            fixed_to_float(green),
            fixed_to_float(blue),
            fixed_to_float(alpha),
        )
    }
    unsafe fn ClearDepthf(&mut self, depth: GLclampf) {
        gl21::ClearDepth(depth.into())
    }
    unsafe fn ClearDepthx(&mut self, depth: GLclampx) {
        self.ClearDepthf(fixed_to_float(depth))
    }
    unsafe fn ClearStencil(&mut self, s: GLint) {
        gl21::ClearStencil(s)
    }

    // Textures
    unsafe fn GenTextures(&mut self, n: GLsizei, textures: *mut GLuint) {
        gl21::GenTextures(n, textures)
    }
    unsafe fn DeleteTextures(&mut self, n: GLsizei, textures: *const GLuint) {
        gl21::DeleteTextures(n, textures)
    }
    unsafe fn BindTexture(&mut self, target: GLenum, texture: GLuint) {
        assert!(target == gl21::TEXTURE_2D);
        gl21::BindTexture(target, texture)
    }
    unsafe fn TexParameteri(&mut self, target: GLenum, pname: GLenum, param: GLint) {
        assert!(target == gl21::TEXTURE_2D);
        assert!(
            pname == gl21::TEXTURE_MIN_FILTER
                || pname == gl21::TEXTURE_MAG_FILTER
                || pname == gl21::TEXTURE_WRAP_S
                || pname == gl21::TEXTURE_WRAP_T
                || pname == gl21::GENERATE_MIPMAP
        );
        gl21::TexParameteri(target, pname, param);
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
        assert!(target == gl21::TEXTURE_2D);
        assert!(level >= 0);
        assert!(
            internalformat as GLenum == gl21::ALPHA
                || internalformat as GLenum == gl21::RGB
                || internalformat as GLenum == gl21::RGBA
                || internalformat as GLenum == gl21::LUMINANCE
                || internalformat as GLenum == gl21::LUMINANCE_ALPHA
        );
        assert!(border == 0);
        assert!(
            format == gl21::ALPHA
                || format == gl21::RGB
                || format == gl21::RGBA
                || format == gl21::LUMINANCE
                || format == gl21::LUMINANCE_ALPHA
        );
        assert!(
            type_ == gl21::UNSIGNED_BYTE
                || type_ == gl21::UNSIGNED_SHORT_5_6_5
                || type_ == gl21::UNSIGNED_SHORT_4_4_4_4
                || type_ == gl21::UNSIGNED_SHORT_5_5_5_1
        );
        gl21::TexImage2D(
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

    // Matrix stack operations
    unsafe fn MatrixMode(&mut self, mode: GLenum) {
        assert!(mode == gl21::MODELVIEW || mode == gl21::PROJECTION || mode == gl21::TEXTURE);
        gl21::MatrixMode(mode);
    }
    unsafe fn LoadIdentity(&mut self) {
        gl21::LoadIdentity();
    }
    unsafe fn LoadMatrixf(&mut self, m: *const GLfloat) {
        gl21::LoadMatrixf(m);
    }
    unsafe fn LoadMatrixx(&mut self, m: *const GLfixed) {
        let matrix = matrix_fixed_to_float(m);
        gl21::LoadMatrixf(matrix.as_ptr());
    }
    unsafe fn MultMatrixf(&mut self, m: *const GLfloat) {
        gl21::MultMatrixf(m);
    }
    unsafe fn MultMatrixx(&mut self, m: *const GLfixed) {
        let matrix = matrix_fixed_to_float(m);
        gl21::MultMatrixf(matrix.as_ptr());
    }
    unsafe fn PushMatrix(&mut self) {
        gl21::PushMatrix();
    }
    unsafe fn PopMatrix(&mut self) {
        gl21::PopMatrix();
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
        gl21::Ortho(
            left.into(),
            right.into(),
            bottom.into(),
            top.into(),
            near.into(),
            far.into(),
        );
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
        gl21::Ortho(
            fixed_to_float(left).into(),
            fixed_to_float(right).into(),
            fixed_to_float(bottom).into(),
            fixed_to_float(top).into(),
            fixed_to_float(near).into(),
            fixed_to_float(far).into(),
        );
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
        gl21::Frustum(
            left.into(),
            right.into(),
            bottom.into(),
            top.into(),
            near.into(),
            far.into(),
        );
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
        gl21::Frustum(
            fixed_to_float(left).into(),
            fixed_to_float(right).into(),
            fixed_to_float(bottom).into(),
            fixed_to_float(top).into(),
            fixed_to_float(near).into(),
            fixed_to_float(far).into(),
        );
    }
    unsafe fn Rotatef(&mut self, angle: GLfloat, x: GLfloat, y: GLfloat, z: GLfloat) {
        gl21::Rotatef(angle, x, y, z);
    }
    unsafe fn Rotatex(&mut self, angle: GLfixed, x: GLfixed, y: GLfixed, z: GLfixed) {
        gl21::Rotatef(
            fixed_to_float(angle),
            fixed_to_float(x),
            fixed_to_float(y),
            fixed_to_float(z),
        );
    }
    unsafe fn Scalef(&mut self, x: GLfloat, y: GLfloat, z: GLfloat) {
        gl21::Scalef(x, y, z);
    }
    unsafe fn Scalex(&mut self, x: GLfixed, y: GLfixed, z: GLfixed) {
        gl21::Scalef(fixed_to_float(x), fixed_to_float(y), fixed_to_float(z));
    }
    unsafe fn Translatef(&mut self, x: GLfloat, y: GLfloat, z: GLfloat) {
        gl21::Translatef(x, y, z);
    }
    unsafe fn Translatex(&mut self, x: GLfixed, y: GLfixed, z: GLfixed) {
        gl21::Translatef(fixed_to_float(x), fixed_to_float(y), fixed_to_float(z));
    }

    // OES_framebuffer_object -> EXT_framebuffer_object
    unsafe fn GenFramebuffersOES(&mut self, n: GLsizei, framebuffers: *mut GLuint) {
        gl21::GenFramebuffersEXT(n, framebuffers)
    }
    unsafe fn GenRenderbuffersOES(&mut self, n: GLsizei, renderbuffers: *mut GLuint) {
        gl21::GenRenderbuffersEXT(n, renderbuffers)
    }
    unsafe fn BindFramebufferOES(&mut self, target: GLenum, framebuffer: GLuint) {
        gl21::BindFramebufferEXT(target, framebuffer)
    }
    unsafe fn BindRenderbufferOES(&mut self, target: GLenum, renderbuffer: GLuint) {
        gl21::BindRenderbufferEXT(target, renderbuffer)
    }
    unsafe fn RenderbufferStorageOES(
        &mut self,
        target: GLenum,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
    ) {
        gl21::RenderbufferStorageEXT(target, internalformat, width, height)
    }
    unsafe fn FramebufferRenderbufferOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        renderbuffertarget: GLenum,
        renderbuffer: GLuint,
    ) {
        gl21::FramebufferRenderbufferEXT(target, attachment, renderbuffertarget, renderbuffer)
    }
    unsafe fn GetRenderbufferParameterivOES(
        &mut self,
        target: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gl21::GetRenderbufferParameterivEXT(target, pname, params)
    }
    unsafe fn CheckFramebufferStatusOES(&mut self, target: GLenum) -> GLenum {
        gl21::CheckFramebufferStatusEXT(target)
    }
}
