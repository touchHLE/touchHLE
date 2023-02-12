/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
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

pub(super) struct ArrayInfo {
    /// Enum used by `glEnableClientState`, `glDisableClientState` and
    /// `glGetBoolean`.
    pub(super) name: GLenum,
    /// Buffer binding enum for `glGetInteger`.
    buffer_binding: GLenum,
    /// Size enum for `glGetInteger`.
    size: Option<GLenum>,
    /// Stride enum for `glGetInteger`.
    stride: GLenum,
    /// Pointer enum for `glGetPointer`.
    pointer: GLenum,
}

struct ArrayStateBackup {
    size: Option<GLint>,
    stride: GLsizei,
    pointer: *const GLvoid,
}

/// List of arrays shared by OpenGL ES 1.1 and OpenGL 2.1.
///
/// TODO: GL_POINT_SIZE_ARRAY_OES?
pub(super) const ARRAYS: &[ArrayInfo] = &[
    ArrayInfo {
        name: gl21::COLOR_ARRAY,
        buffer_binding: gl21::COLOR_ARRAY_BUFFER_BINDING,
        size: Some(gl21::COLOR_ARRAY_SIZE),
        stride: gl21::COLOR_ARRAY_STRIDE,
        pointer: gl21::COLOR_ARRAY_POINTER,
    },
    ArrayInfo {
        name: gl21::NORMAL_ARRAY,
        buffer_binding: gl21::NORMAL_ARRAY_BUFFER_BINDING,
        size: None,
        stride: gl21::NORMAL_ARRAY_STRIDE,
        pointer: gl21::NORMAL_ARRAY_POINTER,
    },
    ArrayInfo {
        name: gl21::TEXTURE_COORD_ARRAY,
        buffer_binding: gl21::TEXTURE_COORD_ARRAY_BUFFER_BINDING,
        size: Some(gl21::TEXTURE_COORD_ARRAY_SIZE),
        stride: gl21::TEXTURE_COORD_ARRAY_STRIDE,
        pointer: gl21::TEXTURE_COORD_ARRAY_POINTER,
    },
    ArrayInfo {
        name: gl21::VERTEX_ARRAY,
        buffer_binding: gl21::VERTEX_ARRAY_BUFFER_BINDING,
        size: Some(gl21::VERTEX_ARRAY_SIZE),
        stride: gl21::VERTEX_ARRAY_STRIDE,
        pointer: gl21::VERTEX_ARRAY_POINTER,
    },
];

/// List of `glLightfv`/`glLightxv` parameters shared by OpenGL ES 1.1 and
/// OpenGL 2.1, together with the number of float/fixed-point values they take.
pub(super) const LIGHT_PARAMS: &[(GLenum, u8)] = &[
    (gl21::AMBIENT, 4),
    (gl21::DIFFUSE, 4),
    (gl21::SPECULAR, 4),
    (gl21::POSITION, 4),
    (gl21::SPOT_CUTOFF, 1),
    (gl21::SPOT_DIRECTION, 3),
    (gl21::SPOT_EXPONENT, 1),
    (gl21::CONSTANT_ATTENUATION, 1),
    (gl21::LINEAR_ATTENUATION, 1),
    (gl21::QUADRATIC_ATTENUATION, 1),
];

/// List of `glTexEnv` parameters for the `GL_TEXTURE_ENV` target shared by
/// OpenGL ES 1.1 and OpenGL 2.1, together with a boolean indicating whether
/// they are integer/enum (true) or float/fixed-point (false), and the number of
/// values they take.
pub(super) const TEX_ENV_PARAMS: &[(GLenum, bool, u8)] = &[
    (gl21::TEXTURE_ENV_MODE, true, 1),
    (gl21::COORD_REPLACE, true, 1),
    (gl21::COMBINE_RGB, true, 1),
    (gl21::COMBINE_ALPHA, true, 1),
    (gl21::SRC0_RGB, true, 1),
    (gl21::SRC1_RGB, true, 1),
    (gl21::SRC2_RGB, true, 1),
    (gl21::SRC0_ALPHA, true, 1),
    (gl21::SRC1_ALPHA, true, 1),
    (gl21::SRC2_ALPHA, true, 1),
    (gl21::OPERAND0_RGB, true, 1),
    (gl21::OPERAND1_RGB, true, 1),
    (gl21::OPERAND2_RGB, true, 1),
    (gl21::OPERAND0_ALPHA, true, 1),
    (gl21::OPERAND1_ALPHA, true, 1),
    (gl21::OPERAND2_ALPHA, true, 1),
    (gl21::TEXTURE_ENV_COLOR, false, 4),
    (gl21::RGB_SCALE, false, 1),
    (gl21::ALPHA_SCALE, false, 1),
];

/// List of integer `glTexParameter` parameters.
const TEX_PARAMS_INT: &[GLenum] = &[
    gl21::TEXTURE_MIN_FILTER,
    gl21::TEXTURE_MAG_FILTER,
    gl21::TEXTURE_WRAP_S,
    gl21::TEXTURE_WRAP_T,
    gl21::GENERATE_MIPMAP,
];
/// List of float/fixed-point `glTexParameter` parameters.
const TEX_PARAMS_FLOAT: &[GLenum] = &[
    gl21::TEXTURE_MAX_ANISOTROPY_EXT,
    gl21::MAX_TEXTURE_MAX_ANISOTROPY_EXT,
];

pub struct GLES1OnGL2 {
    gl_ctx: GLContext,
    pointer_is_fixed_point: [bool; ARRAYS.len()],
    fixed_point_translation_buffers: [Vec<GLfloat>; ARRAYS.len()],
}
impl GLES1OnGL2 {
    /// If any arrays with fixed-point data are in use at the time of a draw
    /// call, this function will convert the data to floating-point and
    /// replace the pointers. [Self::restore_fixed_point_arrays] can be called
    /// after to restore the original state.
    unsafe fn translate_fixed_point_arrays(
        &mut self,
        first: GLint,
        count: GLsizei,
    ) -> [Option<ArrayStateBackup>; ARRAYS.len()] {
        let mut backups: [Option<ArrayStateBackup>; ARRAYS.len()] = Default::default();
        for (i, array_info) in ARRAYS.iter().enumerate() {
            // Decide whether we need to do anything for this array

            if !self.pointer_is_fixed_point[i] {
                continue;
            }

            let mut is_active = gl21::FALSE;
            gl21::GetBooleanv(array_info.name, &mut is_active);
            if is_active != gl21::TRUE {
                continue;
            }

            let mut buffer_binding = 0;
            gl21::GetIntegerv(array_info.buffer_binding, &mut buffer_binding);
            // TODO: translation for bound array buffers
            assert!(buffer_binding == 0);

            // Get and back up data

            let size = array_info.size.map(|size_enum| {
                let mut size: GLint = 0;
                gl21::GetIntegerv(size_enum, &mut size);
                size
            });
            let mut stride: GLsizei = 0;
            gl21::GetIntegerv(array_info.stride, &mut stride);
            let mut pointer: *mut GLvoid = std::ptr::null_mut();
            // The second argument to glGetPointerv must be a mutable pointer,
            // but gl_generator generates the wrong signature by mistake, see
            // https://github.com/brendanzab/gl-rs/issues/541
            #[allow(clippy::unnecessary_mut_passed)]
            gl21::GetPointerv(array_info.pointer, &mut pointer);
            let pointer = pointer.cast_const();

            backups[i] = Some(ArrayStateBackup {
                size,
                stride,
                pointer,
            });

            // Create translated array and substitute pointer

            let size = size.unwrap_or_else(|| {
                assert!(array_info.name == gl21::NORMAL_ARRAY);
                3
            });
            let stride = if stride == 0 {
                // tightly packed mode
                size * 4 // sizeof(gl::FLOAT)
            } else {
                stride
            };

            let buffer = &mut self.fixed_point_translation_buffers[i];
            buffer.clear();
            buffer.resize(((first + count) * size).try_into().unwrap(), 0.0);

            {
                assert!(first >= 0 && count >= 0 && size >= 0 && stride >= 0);
                let first = first as usize;
                let count = count as usize;
                let size = size as usize;
                let stride = stride as usize;
                for j in first..(first + count) {
                    let vector_ptr: *const GLvoid = pointer.add(j * stride);
                    let vector_ptr: *const GLfixed = vector_ptr.cast();
                    for k in 0..size {
                        buffer[j * size + k] = fixed_to_float(vector_ptr.add(k).read_unaligned());
                    }
                }
            }

            let buffer_ptr: *const GLfloat = buffer.as_ptr();
            let buffer_ptr: *const GLvoid = buffer_ptr.cast();
            match array_info.name {
                gl21::COLOR_ARRAY => gl21::ColorPointer(size, gl21::FLOAT, 0, buffer_ptr),
                gl21::NORMAL_ARRAY => {
                    assert!(size == 3);
                    gl21::NormalPointer(gl21::FLOAT, 0, buffer_ptr)
                }
                gl21::TEXTURE_COORD_ARRAY => {
                    gl21::TexCoordPointer(size, gl21::FLOAT, 0, buffer_ptr)
                }
                gl21::VERTEX_ARRAY => gl21::VertexPointer(size, gl21::FLOAT, 0, buffer_ptr),
                _ => unreachable!(),
            }
        }
        backups
    }
    unsafe fn restore_fixed_point_arrays(
        &mut self,
        from_backup: [Option<ArrayStateBackup>; ARRAYS.len()],
    ) {
        for (i, backup) in from_backup.into_iter().enumerate() {
            let array_info = &ARRAYS[i];
            let Some(ArrayStateBackup { size, stride, pointer }) = backup else {
                continue;
            };

            match array_info.name {
                gl21::COLOR_ARRAY => {
                    gl21::ColorPointer(size.unwrap(), gl21::FLOAT, stride, pointer)
                }
                gl21::NORMAL_ARRAY => {
                    assert!(size.is_none());
                    gl21::NormalPointer(gl21::FLOAT, stride, pointer)
                }
                gl21::TEXTURE_COORD_ARRAY => {
                    gl21::TexCoordPointer(size.unwrap(), gl21::FLOAT, stride, pointer)
                }
                gl21::VERTEX_ARRAY => {
                    gl21::VertexPointer(size.unwrap(), gl21::FLOAT, stride, pointer)
                }
                _ => unreachable!(),
            }
        }
    }
}
impl GLES for GLES1OnGL2 {
    fn new(window: &mut Window) -> Self {
        Self {
            gl_ctx: window.create_gl_context(GLVersion::GL21Compat),
            pointer_is_fixed_point: [false; ARRAYS.len()],
            fixed_point_translation_buffers: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
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
        assert!(ARRAYS.iter().any(|&ArrayInfo { name, .. }| name == array));
        gl21::EnableClientState(array);
    }
    unsafe fn DisableClientState(&mut self, array: GLenum) {
        assert!(ARRAYS.iter().any(|&ArrayInfo { name, .. }| name == array));
        gl21::DisableClientState(array);
    }
    unsafe fn GetIntegerv(&mut self, pname: GLenum, params: *mut GLint) {
        // This function family can return a huge number of things.
        // TODO: support more possible values.
        assert!([
            gl21::ARRAY_BUFFER_BINDING,
            gl21::ELEMENT_ARRAY_BUFFER_BINDING,
            gl21::MATRIX_MODE,
            gl21::MAX_TEXTURE_SIZE,
            gl21::TEXTURE_BINDING_2D
        ]
        .contains(&pname));
        gl21::GetIntegerv(pname, params);
    }
    unsafe fn Hint(&mut self, target: GLenum, mode: GLenum) {
        assert!([
            gl21::FOG_HINT,
            gl21::GENERATE_MIPMAP_HINT,
            gl21::LINE_SMOOTH_HINT,
            gl21::PERSPECTIVE_CORRECTION_HINT,
            gl21::POINT_SMOOTH_HINT
        ]
        .contains(&target));
        assert!([gl21::FASTEST, gl21::NICEST, gl21::DONT_CARE].contains(&mode));
        gl21::Hint(target, mode);
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
    unsafe fn CullFace(&mut self, mode: GLenum) {
        assert!([gl21::FRONT, gl21::BACK, gl21::FRONT_AND_BACK].contains(&mode));
        gl21::CullFace(mode);
    }
    unsafe fn DepthMask(&mut self, flag: GLboolean) {
        gl21::DepthMask(flag)
    }
    unsafe fn FrontFace(&mut self, mode: GLenum) {
        assert!(mode == gl21::CW || mode == gl21::CCW);
        gl21::FrontFace(mode);
    }
    unsafe fn DepthRangef(&mut self, near: GLclampf, far: GLclampf) {
        gl21::DepthRange(near.into(), far.into())
    }
    unsafe fn DepthRangex(&mut self, near: GLclampx, far: GLclampx) {
        gl21::DepthRange(fixed_to_float(near).into(), fixed_to_float(far).into())
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

    // Lighting
    unsafe fn Lightf(&mut self, light: GLenum, pname: GLenum, param: GLfloat) {
        assert!(LIGHT_PARAMS
            .iter()
            .any(|&(pname2, pcount)| pname == pname2 && pcount == 1));
        gl21::Lightf(light, pname, param);
    }
    unsafe fn Lightx(&mut self, light: GLenum, pname: GLenum, param: GLfixed) {
        self.Lightf(light, pname, fixed_to_float(param));
    }
    unsafe fn Lightfv(&mut self, light: GLenum, pname: GLenum, params: *const GLfloat) {
        assert!(LIGHT_PARAMS.iter().any(|&(pname2, _)| pname == pname2));
        gl21::Lightfv(light, pname, params);
    }
    unsafe fn Lightxv(&mut self, light: GLenum, pname: GLenum, params: *const GLfixed) {
        let mut params_float = [0.0; 4];
        let &(_, pcount) = LIGHT_PARAMS
            .iter()
            .find(|&&(pname2, _)| pname == pname2)
            .unwrap();
        #[allow(clippy::needless_range_loop)]
        for i in 0..(pcount as usize) {
            params_float[i] = fixed_to_float(params.add(i).read())
        }
        gl21::Lightfv(light, pname, params_float.as_ptr());
    }

    // Buffers
    unsafe fn GenBuffers(&mut self, n: GLsizei, buffers: *mut GLuint) {
        gl21::GenBuffers(n, buffers)
    }
    unsafe fn DeleteBuffers(&mut self, n: GLsizei, buffers: *const GLuint) {
        gl21::DeleteBuffers(n, buffers)
    }
    unsafe fn BindBuffer(&mut self, target: GLenum, buffer: GLuint) {
        assert!(target == gl21::ARRAY_BUFFER || target == gl21::ELEMENT_ARRAY_BUFFER);
        gl21::BindBuffer(target, buffer)
    }

    // Non-pointers
    unsafe fn Color4f(&mut self, red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat) {
        gl21::Color4f(red, green, blue, alpha)
    }
    unsafe fn Color4x(&mut self, red: GLfixed, green: GLfixed, blue: GLfixed, alpha: GLfixed) {
        gl21::Color4f(
            fixed_to_float(red),
            fixed_to_float(green),
            fixed_to_float(blue),
            fixed_to_float(alpha),
        )
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
        if type_ == gles11::FIXED {
            // Translation deferred until draw call
            self.pointer_is_fixed_point[0] = true;
            gl21::ColorPointer(size, gl21::FLOAT, stride, pointer)
        } else {
            assert!(type_ == gl21::UNSIGNED_BYTE || type_ == gl21::FLOAT);
            self.pointer_is_fixed_point[0] = false;
            gl21::ColorPointer(size, type_, stride, pointer)
        }
    }
    unsafe fn NormalPointer(&mut self, type_: GLenum, stride: GLsizei, pointer: *const GLvoid) {
        if type_ == gles11::FIXED {
            // Translation deferred until draw call
            self.pointer_is_fixed_point[1] = true;
            gl21::NormalPointer(gl21::FLOAT, stride, pointer)
        } else {
            assert!(type_ == gl21::BYTE || type_ == gl21::SHORT || type_ == gl21::FLOAT);
            self.pointer_is_fixed_point[1] = false;
            gl21::NormalPointer(type_, stride, pointer)
        }
    }
    unsafe fn TexCoordPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        assert!(size == 2 || size == 3 || size == 4);
        if type_ == gles11::FIXED {
            // Translation deferred until draw call
            self.pointer_is_fixed_point[2] = true;
            gl21::TexCoordPointer(size, gl21::FLOAT, stride, pointer)
        } else {
            // TODO: byte
            assert!(type_ == gl21::SHORT || type_ == gl21::FLOAT);
            self.pointer_is_fixed_point[2] = false;
            gl21::TexCoordPointer(size, type_, stride, pointer)
        }
    }
    unsafe fn VertexPointer(
        &mut self,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        assert!(size == 2 || size == 3 || size == 4);
        if type_ == gles11::FIXED {
            // Translation deferred until draw call
            self.pointer_is_fixed_point[3] = true;
            gl21::VertexPointer(size, gl21::FLOAT, stride, pointer)
        } else {
            // TODO: byte
            assert!(type_ == gl21::SHORT || type_ == gl21::FLOAT);
            self.pointer_is_fixed_point[3] = false;
            gl21::VertexPointer(size, type_, stride, pointer)
        }
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

        let state_backup = self.translate_fixed_point_arrays(first, count);

        gl21::DrawArrays(mode, first, count);

        self.restore_fixed_point_arrays(state_backup);
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

        let state_backup = if self.pointer_is_fixed_point.iter().any(|&is_fixed| is_fixed) {
            // Scan the index buffer to find the range of data that may need
            // fixed-point translation.
            // TODO: Would it be more efficient to turn this into a non-indexed
            // draw-call instead?

            let mut index_buffer_binding = 0;
            gl21::GetIntegerv(
                gl21::ELEMENT_ARRAY_BUFFER_BINDING,
                &mut index_buffer_binding,
            );
            // TODO: handling of bound index array buffers
            assert!(index_buffer_binding == 0);

            let mut first = usize::MAX;
            let mut last = usize::MIN;
            assert!(count >= 0);
            match type_ {
                gl21::UNSIGNED_BYTE => {
                    let indices_ptr: *const GLubyte = indices.cast();
                    for i in 0..(count as usize) {
                        let index = indices_ptr.add(i).read_unaligned();
                        first = first.min(index as usize);
                        last = last.max(index as usize);
                    }
                }
                gl21::UNSIGNED_SHORT => {
                    let indices_ptr: *const GLushort = indices.cast();
                    for i in 0..(count as usize) {
                        let index = indices_ptr.add(i).read_unaligned();
                        first = first.min(index as usize);
                        last = last.max(index as usize);
                    }
                }
                _ => unreachable!(),
            }

            let (first, count) = if first == usize::MAX && last == usize::MIN {
                assert!(count == 0);
                (0, 0)
            } else {
                (
                    first.try_into().unwrap(),
                    (last + 1 - first).try_into().unwrap(),
                )
            };

            Some(self.translate_fixed_point_arrays(first, count))
        } else {
            None
        };

        gl21::DrawElements(mode, count, type_, indices);

        if let Some(state_backup) = state_backup {
            self.restore_fixed_point_arrays(state_backup);
        }
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
        assert!(TEX_PARAMS_INT.contains(&pname) || TEX_PARAMS_FLOAT.contains(&pname));
        gl21::TexParameteri(target, pname, param);
    }
    unsafe fn TexParameterf(&mut self, target: GLenum, pname: GLenum, param: GLfloat) {
        assert!(target == gl21::TEXTURE_2D);
        assert!(TEX_PARAMS_INT.contains(&pname) || TEX_PARAMS_FLOAT.contains(&pname));
        gl21::TexParameterf(target, pname, param);
    }
    unsafe fn TexParameterx(&mut self, target: GLenum, pname: GLenum, param: GLfixed) {
        assert!(target == gl21::TEXTURE_2D);
        // The conversion behaviour for fixed-point to integer is special.
        if TEX_PARAMS_INT.contains(&pname) {
            gl21::TexParameteri(target, pname, param);
        } else {
            assert!(TEX_PARAMS_FLOAT.contains(&pname));
            gl21::TexParameterf(target, pname, fixed_to_float(param));
        }
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
    unsafe fn TexEnvf(&mut self, target: GLenum, pname: GLenum, param: GLfloat) {
        // TODO: GL_POINT_SPRITE_OES
        assert!(target == gl21::TEXTURE_ENV);
        assert!(TEX_ENV_PARAMS
            .iter()
            .any(|&(pname2, _, pcount)| pname == pname2 && pcount == 1));
        gl21::TexEnvf(target, pname, param);
    }
    unsafe fn TexEnvx(&mut self, target: GLenum, pname: GLenum, param: GLfixed) {
        // TODO: GL_POINT_SPRITE_OES
        assert!(target == gl21::TEXTURE_ENV);
        let &(_, is_integer, _) = TEX_ENV_PARAMS
            .iter()
            .find(|&&(pname2, _, pcount)| pname == pname2 && pcount == 1)
            .unwrap();
        // The conversion behaviour for fixed-point to integer is special.
        if is_integer {
            gl21::TexEnvi(target, pname, param);
        } else {
            gl21::TexEnvf(target, pname, fixed_to_float(param));
        }
    }
    unsafe fn TexEnvi(&mut self, target: GLenum, pname: GLenum, param: GLint) {
        // TODO: GL_POINT_SPRITE_OES
        assert!(target == gl21::TEXTURE_ENV);
        assert!(TEX_ENV_PARAMS
            .iter()
            .any(|&(pname2, _, pcount)| pname == pname2 && pcount == 1));
        gl21::TexEnvi(target, pname, param);
    }
    unsafe fn TexEnvfv(&mut self, target: GLenum, pname: GLenum, params: *const GLfloat) {
        // TODO: GL_POINT_SPRITE_OES
        assert!(target == gl21::TEXTURE_ENV);
        assert!(TEX_ENV_PARAMS.iter().any(|&(pname2, _, _)| pname == pname2));
        gl21::TexEnvfv(target, pname, params);
    }
    unsafe fn TexEnvxv(&mut self, target: GLenum, pname: GLenum, params: *const GLfixed) {
        // TODO: GL_POINT_SPRITE_OES
        assert!(target == gl21::TEXTURE_ENV);
        let &(_, is_integer, pcount) = TEX_ENV_PARAMS
            .iter()
            .find(|&&(pname2, _, _)| pname == pname2)
            .unwrap();
        // The conversion behaviour for fixed-point to integer is special.
        if is_integer {
            gl21::TexEnviv(target, pname, params.cast());
        } else {
            let mut params_float = [0.0; 4];
            #[allow(clippy::needless_range_loop)]
            for i in 0..(pcount as usize) {
                params_float[i] = fixed_to_float(params.add(i).read())
            }
            gl21::TexEnvfv(target, pname, params_float.as_ptr());
        }
    }
    unsafe fn TexEnviv(&mut self, target: GLenum, pname: GLenum, params: *const GLint) {
        // TODO: GL_POINT_SPRITE_OES
        assert!(target == gl21::TEXTURE_ENV);
        assert!(TEX_ENV_PARAMS.iter().any(|&(pname2, _, _)| pname == pname2));
        gl21::TexEnviv(target, pname, params);
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
    unsafe fn DeleteFramebuffersOES(&mut self, n: GLsizei, framebuffers: *mut GLuint) {
        gl21::DeleteFramebuffersEXT(n, framebuffers)
    }
    unsafe fn DeleteRenderbuffersOES(&mut self, n: GLsizei, renderbuffers: *mut GLuint) {
        gl21::DeleteRenderbuffersEXT(n, renderbuffers)
    }
}
