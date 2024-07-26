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

use super::gl21compat_raw as gl21;
use super::gl21compat_raw::types::*;
use super::gles11_raw as gles11; // constants only
use super::util::{
    fixed_to_float, matrix_fixed_to_float, try_decode_pvrtc, PalettedTextureFormat, ParamTable,
    ParamType,
};
use super::GLES;
use crate::window::{GLContext, GLVersion, Window};
use std::collections::HashSet;
use std::ffi::CStr;

/// List of capabilities shared by OpenGL ES 1.1 and OpenGL 2.1.
///
/// Note: There can be arbitrarily many lights or clip planes, depending on
/// implementation limits. We might eventually need to check those rather than
/// just providing the minimum.
pub const CAPABILITIES: &[GLenum] = &[
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
    // Same as POINT_SPRITE_OES from the GLES extension
    gl21::POINT_SPRITE,
];

pub struct ArrayInfo {
    /// Enum used by `glEnableClientState`, `glDisableClientState` and
    /// `glGetBoolean`.
    pub name: GLenum,
    /// Buffer binding enum for `glGetInteger`.
    pub buffer_binding: GLenum,
    /// Size enum for `glGetInteger`.
    size: Option<GLenum>,
    /// Stride enum for `glGetInteger`.
    stride: GLenum,
    /// Pointer enum for `glGetPointer`.
    pub pointer: GLenum,
}

struct ArrayStateBackup {
    size: Option<GLint>,
    stride: GLsizei,
    pointer: *const GLvoid,
}

/// List of arrays shared by OpenGL ES 1.1 and OpenGL 2.1.
///
/// TODO: GL_POINT_SIZE_ARRAY_OES?
pub const ARRAYS: &[ArrayInfo] = &[
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

/// Table of `glGet` parameters shared by OpenGL ES 1.1 and OpenGL 2.1.
const GET_PARAMS: ParamTable = ParamTable(&[
    (gl21::ACTIVE_TEXTURE, ParamType::Int, 1),
    (gl21::ALIASED_POINT_SIZE_RANGE, ParamType::Float, 2),
    (gl21::ALIASED_LINE_WIDTH_RANGE, ParamType::Float, 2),
    (gl21::ALPHA_BITS, ParamType::Int, 1),
    (gl21::ALPHA_TEST, ParamType::Boolean, 1),
    (gl21::ALPHA_TEST_FUNC, ParamType::Int, 1),
    // TODO: ALPHA_TEST_REF (has special type conversion behavior)
    (gl21::ARRAY_BUFFER_BINDING, ParamType::Int, 1),
    (gl21::BLEND, ParamType::Boolean, 1),
    (gl21::BLEND_DST, ParamType::Int, 1),
    (gl21::BLEND_SRC, ParamType::Int, 1),
    (gl21::BLUE_BITS, ParamType::Int, 1),
    (gl21::CLIENT_ACTIVE_TEXTURE, ParamType::Int, 1),
    // TODO: arbitrary number of clip planes?
    (gl21::CLIP_PLANE0, ParamType::Boolean, 1),
    (gl21::COLOR_ARRAY, ParamType::Boolean, 1),
    (gl21::COLOR_ARRAY_BUFFER_BINDING, ParamType::Int, 1),
    (gl21::COLOR_ARRAY_SIZE, ParamType::Int, 1),
    (gl21::COLOR_ARRAY_STRIDE, ParamType::Int, 1),
    (gl21::COLOR_ARRAY_TYPE, ParamType::Int, 1),
    (gl21::COLOR_CLEAR_VALUE, ParamType::FloatSpecial, 4), // TODO correct type
    (gl21::COLOR_LOGIC_OP, ParamType::Boolean, 1),
    (gl21::COLOR_MATERIAL, ParamType::Boolean, 1),
    (gl21::COLOR_WRITEMASK, ParamType::Boolean, 4),
    // TODO: COMPRESSED_TEXTURE_FORMATS (need to support PVRTC etc)
    (gl21::CULL_FACE, ParamType::Boolean, 1),
    (gl21::CULL_FACE_MODE, ParamType::Int, 1),
    (gl21::CURRENT_COLOR, ParamType::FloatSpecial, 4), // TODO correct type
    // TODO: CURRENT_NORMAL (has special type conversion behavior)
    (gl21::CURRENT_TEXTURE_COORDS, ParamType::Float, 4),
    (gl21::DEPTH_BITS, ParamType::Int, 1),
    // TODO: DEPTH_CLEAR_VALUE (has special type conversion behavior)
    (gl21::DEPTH_FUNC, ParamType::Int, 1),
    // TODO: DEPTH_RANGE (has special type conversion behavior)
    (gl21::DEPTH_TEST, ParamType::Boolean, 1),
    (gl21::DEPTH_WRITEMASK, ParamType::Boolean, 1),
    (gl21::DITHER, ParamType::Boolean, 1),
    (gl21::ELEMENT_ARRAY_BUFFER_BINDING, ParamType::Int, 1),
    (gl21::FOG, ParamType::Boolean, 1),
    // TODO: FOG_COLOR (has special type conversion behavior)
    (gl21::FOG_HINT, ParamType::Int, 1),
    (gl21::FOG_MODE, ParamType::Int, 1),
    (gl21::FOG_DENSITY, ParamType::Float, 1),
    (gl21::FOG_START, ParamType::Float, 1),
    (gl21::FOG_END, ParamType::Float, 1),
    (gl21::FRONT_FACE, ParamType::Int, 1),
    (gl21::GREEN_BITS, ParamType::Int, 1),
    // TODO: IMPLEMENTATION_COLOR_READ_FORMAT_OES? (not shared)
    // TODO: IMPLEMENTATION_COLOR_READ_TYPE_OES? (not shared)
    // TODO: LIGHT_MODEL_AMBIENT (has special type conversion behavior)
    (gl21::LIGHT_MODEL_TWO_SIDE, ParamType::Boolean, 1),
    // TODO: arbitrary number of lights?
    (gl21::LIGHT0, ParamType::Boolean, 1),
    (gl21::LIGHT1, ParamType::Boolean, 1),
    (gl21::LIGHT2, ParamType::Boolean, 1),
    (gl21::LIGHT3, ParamType::Boolean, 1),
    (gl21::LIGHT4, ParamType::Boolean, 1),
    (gl21::LIGHT5, ParamType::Boolean, 1),
    (gl21::LIGHT6, ParamType::Boolean, 1),
    (gl21::LIGHT7, ParamType::Boolean, 1),
    (gl21::LIGHTING, ParamType::Boolean, 1),
    (gl21::LINE_SMOOTH, ParamType::Boolean, 1),
    (gl21::LINE_SMOOTH_HINT, ParamType::Int, 1),
    (gl21::LINE_WIDTH, ParamType::Float, 1),
    (gl21::LOGIC_OP_MODE, ParamType::Int, 1),
    (gl21::MATRIX_MODE, ParamType::Int, 1),
    (gl21::MAX_CLIP_PLANES, ParamType::Int, 1),
    (gl21::MAX_LIGHTS, ParamType::Int, 1),
    (gl21::MAX_MODELVIEW_STACK_DEPTH, ParamType::Int, 1),
    (gl21::MAX_PROJECTION_STACK_DEPTH, ParamType::Int, 1),
    (gl21::MAX_TEXTURE_MAX_ANISOTROPY_EXT, ParamType::Float, 1),
    (gl21::MAX_TEXTURE_SIZE, ParamType::Int, 1),
    (gl21::MAX_TEXTURE_STACK_DEPTH, ParamType::Int, 1),
    (gl21::MAX_TEXTURE_UNITS, ParamType::Int, 1),
    (gl21::MAX_VIEWPORT_DIMS, ParamType::Int, 1),
    (gl21::MODELVIEW_MATRIX, ParamType::Float, 16),
    (gl21::MODELVIEW_STACK_DEPTH, ParamType::Int, 1),
    (gl21::MULTISAMPLE, ParamType::Boolean, 1),
    (gl21::NORMAL_ARRAY, ParamType::Boolean, 1),
    (gl21::NORMAL_ARRAY_BUFFER_BINDING, ParamType::Int, 1),
    (gl21::NORMAL_ARRAY_STRIDE, ParamType::Int, 1),
    (gl21::NORMAL_ARRAY_TYPE, ParamType::Int, 1),
    (gl21::NORMALIZE, ParamType::Boolean, 1),
    // TODO: NUM_COMPRESSED_TEXTURE_FORMATS (need to support PVRTC etc)
    (gl21::PACK_ALIGNMENT, ParamType::Int, 1),
    (gl21::PERSPECTIVE_CORRECTION_HINT, ParamType::Int, 1),
    (gl21::POINT_DISTANCE_ATTENUATION, ParamType::Float, 3),
    (gl21::POINT_FADE_THRESHOLD_SIZE, ParamType::Float, 1),
    (gl21::POINT_SIZE, ParamType::Float, 1),
    // TODO: POINT_SIZE_ARRAY_OES etc? (not shared)
    (gl21::POINT_SIZE_MAX, ParamType::Float, 1),
    (gl21::POINT_SIZE_MIN, ParamType::Float, 1),
    (gl21::POINT_SIZE_RANGE, ParamType::Float, 2),
    (gl21::POINT_SMOOTH, ParamType::Boolean, 2),
    (gl21::POINT_SMOOTH_HINT, ParamType::Int, 2),
    (gl21::POINT_SPRITE, ParamType::Boolean, 1),
    (gl21::POLYGON_OFFSET_FACTOR, ParamType::Float, 1),
    (gl21::POLYGON_OFFSET_FILL, ParamType::Boolean, 1),
    (gl21::POLYGON_OFFSET_UNITS, ParamType::Float, 1),
    (gl21::PROJECTION_MATRIX, ParamType::Float, 16),
    (gl21::PROJECTION_STACK_DEPTH, ParamType::Int, 1),
    (gl21::RED_BITS, ParamType::Int, 1),
    (gl21::RESCALE_NORMAL, ParamType::Boolean, 1),
    (gl21::SAMPLE_ALPHA_TO_COVERAGE, ParamType::Boolean, 1),
    (gl21::SAMPLE_ALPHA_TO_ONE, ParamType::Boolean, 1),
    (gl21::SAMPLE_BUFFERS, ParamType::Int, 1),
    (gl21::SAMPLE_COVERAGE, ParamType::Boolean, 1),
    (gl21::SAMPLE_COVERAGE_INVERT, ParamType::Boolean, 1),
    (gl21::SAMPLE_COVERAGE_VALUE, ParamType::Float, 1),
    (gl21::SAMPLES, ParamType::Int, 1),
    (gl21::SCISSOR_BOX, ParamType::Int, 4),
    (gl21::SCISSOR_TEST, ParamType::Boolean, 1),
    (gl21::SHADE_MODEL, ParamType::Int, 1),
    (gl21::SMOOTH_LINE_WIDTH_RANGE, ParamType::Float, 2),
    (gl21::SMOOTH_POINT_SIZE_RANGE, ParamType::Float, 2),
    (gl21::STENCIL_BITS, ParamType::Int, 1),
    (gl21::STENCIL_CLEAR_VALUE, ParamType::Int, 1),
    (gl21::STENCIL_FAIL, ParamType::Int, 1),
    (gl21::STENCIL_FUNC, ParamType::Int, 1),
    (gl21::STENCIL_PASS_DEPTH_FAIL, ParamType::Int, 1),
    (gl21::STENCIL_PASS_DEPTH_PASS, ParamType::Int, 1),
    (gl21::STENCIL_REF, ParamType::Int, 1),
    (gl21::STENCIL_TEST, ParamType::Boolean, 1),
    (gl21::STENCIL_VALUE_MASK, ParamType::Int, 1),
    (gl21::STENCIL_WRITEMASK, ParamType::Int, 1),
    (gl21::SUBPIXEL_BITS, ParamType::Int, 1),
    (gl21::TEXTURE_2D, ParamType::Boolean, 1),
    (gl21::TEXTURE_BINDING_2D, ParamType::Int, 1),
    (gl21::TEXTURE_COORD_ARRAY, ParamType::Boolean, 1),
    (gl21::TEXTURE_COORD_ARRAY_BUFFER_BINDING, ParamType::Int, 1),
    (gl21::TEXTURE_COORD_ARRAY_SIZE, ParamType::Int, 1),
    (gl21::TEXTURE_COORD_ARRAY_STRIDE, ParamType::Int, 1),
    (gl21::TEXTURE_COORD_ARRAY_TYPE, ParamType::Int, 1),
    (gl21::TEXTURE_MATRIX, ParamType::Float, 16),
    (gl21::TEXTURE_STACK_DEPTH, ParamType::Int, 1),
    (gl21::UNPACK_ALIGNMENT, ParamType::Int, 1),
    (gl21::VIEWPORT, ParamType::Int, 4),
    (gl21::VERTEX_ARRAY, ParamType::Boolean, 1),
    (gl21::VERTEX_ARRAY_BUFFER_BINDING, ParamType::Int, 1),
    (gl21::VERTEX_ARRAY_SIZE, ParamType::Int, 1),
    (gl21::VERTEX_ARRAY_STRIDE, ParamType::Int, 1),
    (gl21::VERTEX_ARRAY_TYPE, ParamType::Int, 1),
    // OES_framebuffer_object -> EXT_framebuffer_object
    (gl21::FRAMEBUFFER_BINDING_EXT, ParamType::Int, 1),
    (gl21::RENDERBUFFER_BINDING_EXT, ParamType::Int, 1),
    // EXT_texture_lod_bias
    (gl21::MAX_TEXTURE_LOD_BIAS_EXT, ParamType::Float, 1),
    // OES_matrix_palette -> ARB_matrix_palette
    (gl21::MAX_PALETTE_MATRICES_ARB, ParamType::Int, 1),
    // OES_matrix_palette -> ARB_vertex_blend
    (gl21::MAX_VERTEX_UNITS_ARB, ParamType::Int, 1),
]);

const POINT_PARAMS: ParamTable = ParamTable(&[
    (gl21::POINT_SIZE_MIN, ParamType::Float, 1),
    (gl21::POINT_SIZE_MAX, ParamType::Float, 1),
    (gl21::POINT_DISTANCE_ATTENUATION, ParamType::Float, 3),
    (gl21::POINT_FADE_THRESHOLD_SIZE, ParamType::Float, 1),
    (gl21::POINT_SMOOTH, ParamType::Boolean, 1),
]);

/// Table of `glFog` parameters shared by OpenGL ES 1.1 and OpenGL 2.1.
const FOG_PARAMS: ParamTable = ParamTable(&[
    // Despite only having f, fv, x and xv setters in OpenGL ES 1.1, this is
    // an integer! (You're meant to use the x/xv setter.)
    (gl21::FOG_MODE, ParamType::Int, 1),
    (gl21::FOG_DENSITY, ParamType::Float, 1),
    (gl21::FOG_START, ParamType::Float, 1),
    (gl21::FOG_END, ParamType::Float, 1),
    (gl21::FOG_COLOR, ParamType::FloatSpecial, 4), // TODO correct type
]);

/// Table of `glLight` parameters shared by OpenGL ES 1.1 and OpenGL 2.1.
const LIGHT_PARAMS: ParamTable = ParamTable(&[
    (gl21::AMBIENT, ParamType::Float, 4),
    (gl21::DIFFUSE, ParamType::Float, 4),
    (gl21::SPECULAR, ParamType::Float, 4),
    (gl21::POSITION, ParamType::Float, 4),
    (gl21::SPOT_CUTOFF, ParamType::Float, 1),
    (gl21::SPOT_DIRECTION, ParamType::Float, 3),
    (gl21::SPOT_EXPONENT, ParamType::Float, 1),
    (gl21::CONSTANT_ATTENUATION, ParamType::Float, 1),
    (gl21::LINEAR_ATTENUATION, ParamType::Float, 1),
    (gl21::QUADRATIC_ATTENUATION, ParamType::Float, 1),
]);

/// Table of `glMaterial` parameters shared by OpenGL ES 1.1 and OpenGL 2.1.
const MATERIAL_PARAMS: ParamTable = ParamTable(&[
    (gl21::AMBIENT, ParamType::Float, 4),
    (gl21::DIFFUSE, ParamType::Float, 4),
    (gl21::SPECULAR, ParamType::Float, 4),
    (gl21::EMISSION, ParamType::Float, 4),
    (gl21::SHININESS, ParamType::Float, 1),
    // Not a true parameter: it's equivalent to calling glMaterial twice, once
    // for GL_AMBIENT and once for GL_DIFFUSE.
    (gl21::AMBIENT_AND_DIFFUSE, ParamType::Float, 4),
]);

/// Table of `glTexEnv` parameters for the `GL_TEXTURE_ENV` target shared by
/// OpenGL ES 1.1 and OpenGL 2.1.
const TEX_ENV_PARAMS: ParamTable = ParamTable(&[
    (gl21::TEXTURE_ENV_MODE, ParamType::Int, 1),
    (gl21::COORD_REPLACE, ParamType::Int, 1),
    (gl21::COMBINE_RGB, ParamType::Int, 1),
    (gl21::COMBINE_ALPHA, ParamType::Int, 1),
    (gl21::SRC0_RGB, ParamType::Int, 1),
    (gl21::SRC1_RGB, ParamType::Int, 1),
    (gl21::SRC2_RGB, ParamType::Int, 1),
    (gl21::SRC0_ALPHA, ParamType::Int, 1),
    (gl21::SRC1_ALPHA, ParamType::Int, 1),
    (gl21::SRC2_ALPHA, ParamType::Int, 1),
    (gl21::OPERAND0_RGB, ParamType::Int, 1),
    (gl21::OPERAND1_RGB, ParamType::Int, 1),
    (gl21::OPERAND2_RGB, ParamType::Int, 1),
    (gl21::OPERAND0_ALPHA, ParamType::Int, 1),
    (gl21::OPERAND1_ALPHA, ParamType::Int, 1),
    (gl21::OPERAND2_ALPHA, ParamType::Int, 1),
    (gl21::TEXTURE_ENV_COLOR, ParamType::Float, 4),
    (gl21::RGB_SCALE, ParamType::Float, 1),
    (gl21::ALPHA_SCALE, ParamType::Float, 1),
]);

/// Table of `glTexParameter` parameters.
const TEX_PARAMS: ParamTable = ParamTable(&[
    (gl21::TEXTURE_MIN_FILTER, ParamType::Int, 1),
    (gl21::TEXTURE_MAG_FILTER, ParamType::Int, 1),
    (gl21::TEXTURE_WRAP_S, ParamType::Int, 1),
    (gl21::TEXTURE_WRAP_T, ParamType::Int, 1),
    (gl21::GENERATE_MIPMAP, ParamType::Int, 1),
    (gl21::TEXTURE_MAX_ANISOTROPY_EXT, ParamType::Float, 1),
    (gl21::MAX_TEXTURE_MAX_ANISOTROPY_EXT, ParamType::Float, 1),
]);

pub struct GLES1OnGL2 {
    gl_ctx: GLContext,
    pointer_is_fixed_point: [bool; ARRAYS.len()],
    fixed_point_texture_units: HashSet<GLenum>,
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

            // There is one texture co-ordinates pointer per texture unit.
            let old_client_active_texture = if array_info.name == gl21::TEXTURE_COORD_ARRAY {
                // Is the texture unit involved in this draw call fixed-point?
                // If not, we don't need to do anything.
                let mut active_texture: GLenum = 0;
                gl21::GetIntegerv(
                    gl21::ACTIVE_TEXTURE,
                    &mut active_texture as *mut _ as *mut _,
                );
                if !self.fixed_point_texture_units.contains(&active_texture) {
                    continue;
                }

                // Make sure our glTexCoordPointer call will affect that unit.
                let mut old_client_active_texture: GLenum = 0;
                gl21::GetIntegerv(
                    gl21::CLIENT_ACTIVE_TEXTURE,
                    &mut old_client_active_texture as *mut _ as *mut _,
                );
                gl21::ClientActiveTexture(active_texture);
                Some(old_client_active_texture)
            } else {
                None
            };

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

            if let Some(old_client_active_texture) = old_client_active_texture {
                gl21::ClientActiveTexture(old_client_active_texture);
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
            let Some(ArrayStateBackup {
                size,
                stride,
                pointer,
            }) = backup
            else {
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
                    let mut active_texture: GLenum = 0;
                    gl21::GetIntegerv(
                        gl21::ACTIVE_TEXTURE,
                        &mut active_texture as *mut _ as *mut _,
                    );
                    assert!(self.fixed_point_texture_units.contains(&active_texture));
                    let mut old_client_active_texture: GLenum = 0;
                    gl21::GetIntegerv(
                        gl21::CLIENT_ACTIVE_TEXTURE,
                        &mut old_client_active_texture as *mut _ as *mut _,
                    );
                    gl21::ClientActiveTexture(active_texture);
                    gl21::TexCoordPointer(size.unwrap(), gl21::FLOAT, stride, pointer);
                    gl21::ClientActiveTexture(old_client_active_texture)
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
    fn description() -> &'static str {
        "OpenGL ES 1.1 via touchHLE GLES1-on-GL2 layer"
    }

    fn new(window: &mut Window) -> Result<Self, String> {
        Ok(Self {
            gl_ctx: window.create_gl_context(GLVersion::GL21Compat)?,
            pointer_is_fixed_point: [false; ARRAYS.len()],
            fixed_point_texture_units: HashSet::new(),
            fixed_point_translation_buffers: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
        })
    }

    fn make_current(&self, window: &Window) {
        unsafe { window.make_gl_context_current(&self.gl_ctx) };
        gl21::load_with(|s| window.gl_get_proc_address(s))
    }

    unsafe fn driver_description(&self) -> String {
        let version = CStr::from_ptr(gl21::GetString(gl21::VERSION) as *const _);
        let vendor = CStr::from_ptr(gl21::GetString(gl21::VENDOR) as *const _);
        let renderer = CStr::from_ptr(gl21::GetString(gl21::RENDERER) as *const _);
        // OpenGL's version string is just a number, so let's contextualize it.
        format!(
            "OpenGL {} / {} / {}",
            version.to_string_lossy(),
            vendor.to_string_lossy(),
            renderer.to_string_lossy()
        )
    }

    // Generic state manipulation
    unsafe fn GetError(&mut self) -> GLenum {
        gl21::GetError()
    }
    unsafe fn Enable(&mut self, cap: GLenum) {
        if ARRAYS.iter().any(|&ArrayInfo { name, .. }| name == cap) {
            log_dbg!("Tolerating glEnable({:#x}) of client state", cap);
        } else {
            assert!(CAPABILITIES.contains(&cap));
        }
        gl21::Enable(cap);
    }
    unsafe fn IsEnabled(&mut self, cap: GLenum) -> GLboolean {
        assert!(
            CAPABILITIES.contains(&cap) || ARRAYS.iter().any(|&ArrayInfo { name, .. }| name == cap)
        );
        gl21::IsEnabled(cap)
    }
    unsafe fn Disable(&mut self, cap: GLenum) {
        if ARRAYS.iter().any(|&ArrayInfo { name, .. }| name == cap) {
            log_dbg!("Tolerating glDisable({:#x}) of client state", cap);
        } else {
            assert!(CAPABILITIES.contains(&cap));
        }
        gl21::Disable(cap);
    }
    unsafe fn ClientActiveTexture(&mut self, texture: GLenum) {
        gl21::ClientActiveTexture(texture);
    }
    unsafe fn EnableClientState(&mut self, array: GLenum) {
        assert!(ARRAYS.iter().any(|&ArrayInfo { name, .. }| name == array));
        gl21::EnableClientState(array);
    }
    unsafe fn DisableClientState(&mut self, array: GLenum) {
        assert!(ARRAYS.iter().any(|&ArrayInfo { name, .. }| name == array));
        gl21::DisableClientState(array);
    }
    unsafe fn GetBooleanv(&mut self, pname: GLenum, params: *mut GLboolean) {
        let (type_, _count) = GET_PARAMS.get_type_info(pname);
        // TODO: type conversion
        assert!(type_ == ParamType::Boolean);
        gl21::GetBooleanv(pname, params);
    }
    // TODO: GetFixedv
    unsafe fn GetFloatv(&mut self, pname: GLenum, params: *mut GLfloat) {
        let (type_, _count) = GET_PARAMS.get_type_info(pname);
        // TODO: type conversion
        assert!(type_ == ParamType::Float || type_ == ParamType::FloatSpecial);
        gl21::GetFloatv(pname, params);
    }
    unsafe fn GetIntegerv(&mut self, pname: GLenum, params: *mut GLint) {
        let (type_, _count) = GET_PARAMS.get_type_info(pname);
        // TODO: type conversion
        assert!(type_ == ParamType::Int);
        gl21::GetIntegerv(pname, params);
    }
    unsafe fn GetTexEnviv(&mut self, target: GLenum, pname: GLenum, params: *mut GLint) {
        let (type_, _count) = TEX_ENV_PARAMS.get_type_info(pname);
        assert!(type_ == ParamType::Int);
        assert_eq!(target, gl21::TEXTURE_ENV);
        gl21::GetTexEnviv(target, pname, params);
    }
    unsafe fn GetTexEnvfv(&mut self, target: GLenum, pname: GLenum, params: *mut GLfloat) {
        let (type_, _count) = TEX_ENV_PARAMS.get_type_info(pname);
        assert!(type_ == ParamType::Float);
        assert_eq!(target, gl21::TEXTURE_ENV);
        gl21::GetTexEnvfv(target, pname, params);
    }
    unsafe fn GetPointerv(&mut self, pname: GLenum, params: *mut *const GLvoid) {
        assert!(ARRAYS
            .iter()
            .any(|&ArrayInfo { pointer, .. }| pname == pointer));
        // The second argument to glGetPointerv must be a mutable pointer,
        // but gl_generator generates the wrong signature by mistake, see
        // https://github.com/brendanzab/gl-rs/issues/541
        gl21::GetPointerv(pname, params as *mut _ as *const _);
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
    unsafe fn Finish(&mut self) {
        gl21::Finish();
    }
    unsafe fn Flush(&mut self) {
        gl21::Flush();
    }
    unsafe fn GetString(&mut self, name: GLenum) -> *const GLubyte {
        gl21::GetString(name)
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
        let common_factors = [
            gl21::ZERO,
            gl21::ONE,
            gl21::SRC_ALPHA,
            gl21::ONE_MINUS_SRC_ALPHA,
            gl21::DST_ALPHA,
            gl21::ONE_MINUS_DST_ALPHA,
        ];
        let sfactors = [
            gl21::DST_COLOR,
            gl21::ONE_MINUS_DST_COLOR,
            gl21::SRC_ALPHA_SATURATE,
        ];
        let dfactors = [gl21::SRC_COLOR, gl21::ONE_MINUS_SRC_COLOR];
        assert!(
            common_factors.contains(&sfactor)
                || sfactors.contains(&sfactor)
                || dfactors.contains(&sfactor)
        );
        assert!(
            common_factors.contains(&dfactor)
                || sfactors.contains(&dfactor)
                || dfactors.contains(&dfactor)
        );
        if sfactors.contains(&dfactor) {
            log_dbg!("Tolerating sfactor {:#x} in dfactor argument", dfactor);
        }
        if dfactors.contains(&sfactor) {
            log_dbg!("Tolerating dfactor {:#x} in sfactor argument", sfactor);
        }
        gl21::BlendFunc(sfactor, dfactor);
    }
    unsafe fn ColorMask(
        &mut self,
        red: GLboolean,
        green: GLboolean,
        blue: GLboolean,
        alpha: GLboolean,
    ) {
        gl21::ColorMask(red, green, blue, alpha)
    }
    unsafe fn CullFace(&mut self, mode: GLenum) {
        assert!([gl21::FRONT, gl21::BACK, gl21::FRONT_AND_BACK].contains(&mode));
        gl21::CullFace(mode);
    }
    unsafe fn DepthFunc(&mut self, func: GLenum) {
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
        gl21::DepthFunc(func)
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
    unsafe fn PolygonOffset(&mut self, factor: GLfloat, units: GLfloat) {
        gl21::PolygonOffset(factor, units)
    }
    unsafe fn PolygonOffsetx(&mut self, factor: GLfixed, units: GLfixed) {
        gl21::PolygonOffset(fixed_to_float(factor), fixed_to_float(units))
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
    unsafe fn LineWidth(&mut self, val: GLfloat) {
        gl21::LineWidth(val)
    }
    unsafe fn LineWidthx(&mut self, val: GLfixed) {
        gl21::LineWidth(fixed_to_float(val))
    }
    unsafe fn StencilFunc(&mut self, func: GLenum, ref_: GLint, mask: GLuint) {
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
        gl21::StencilFunc(func, ref_, mask);
    }
    unsafe fn StencilOp(&mut self, sfail: GLenum, dpfail: GLenum, dppass: GLenum) {
        for enum_ in [sfail, dpfail, dppass].iter() {
            assert!([
                gl21::KEEP,
                gl21::ZERO,
                gl21::REPLACE,
                gl21::INCR,
                gl21::DECR,
                gl21::INVERT,
            ]
            .contains(enum_));
        }
        gl21::StencilOp(sfail, dpfail, dppass);
    }
    unsafe fn StencilMask(&mut self, mask: GLuint) {
        gl21::StencilMask(mask);
    }

    // Points
    unsafe fn PointSize(&mut self, size: GLfloat) {
        gl21::PointSize(size)
    }
    unsafe fn PointSizex(&mut self, size: GLfixed) {
        gl21::PointSize(fixed_to_float(size))
    }
    unsafe fn PointParameterf(&mut self, pname: GLenum, param: GLfloat) {
        gl21::PointParameterf(pname, param)
    }
    unsafe fn PointParameterx(&mut self, pname: GLenum, param: GLfixed) {
        POINT_PARAMS.setx(
            |param| gl21::PointParameterf(pname, param),
            |_| unreachable!(), // no integer parameters exist
            pname,
            param,
        );
    }
    unsafe fn PointParameterfv(&mut self, pname: GLenum, params: *const GLfloat) {
        gl21::PointParameterfv(pname, params)
    }
    unsafe fn PointParameterxv(&mut self, pname: GLenum, params: *const GLfixed) {
        POINT_PARAMS.setxv(
            |params| gl21::PointParameterfv(pname, params),
            |_| unreachable!(), // no integer parameters exist
            pname,
            params,
        );
    }

    // Lighting and materials
    unsafe fn Fogf(&mut self, pname: GLenum, param: GLfloat) {
        FOG_PARAMS.assert_component_count(pname, 1);
        gl21::Fogf(pname, param);
    }
    unsafe fn Fogx(&mut self, pname: GLenum, param: GLfixed) {
        FOG_PARAMS.setx(
            |param| gl21::Fogf(pname, param),
            |param| gl21::Fogi(pname, param),
            pname,
            param,
        )
    }
    unsafe fn Fogfv(&mut self, pname: GLenum, params: *const GLfloat) {
        FOG_PARAMS.assert_known_param(pname);
        gl21::Fogfv(pname, params);
    }
    unsafe fn Fogxv(&mut self, pname: GLenum, params: *const GLfixed) {
        FOG_PARAMS.setxv(
            |params| gl21::Fogfv(pname, params),
            |params| gl21::Fogiv(pname, params),
            pname,
            params,
        )
    }
    unsafe fn Lightf(&mut self, light: GLenum, pname: GLenum, param: GLfloat) {
        LIGHT_PARAMS.assert_component_count(pname, 1);
        gl21::Lightf(light, pname, param);
    }
    unsafe fn Lightx(&mut self, light: GLenum, pname: GLenum, param: GLfixed) {
        LIGHT_PARAMS.setx(
            |param| gl21::Lightf(light, pname, param),
            |param| gl21::Lighti(light, pname, param),
            pname,
            param,
        )
    }
    unsafe fn Lightfv(&mut self, light: GLenum, pname: GLenum, params: *const GLfloat) {
        LIGHT_PARAMS.assert_known_param(pname);
        gl21::Lightfv(light, pname, params);
    }
    unsafe fn Lightxv(&mut self, light: GLenum, pname: GLenum, params: *const GLfixed) {
        LIGHT_PARAMS.setxv(
            |params| gl21::Lightfv(light, pname, params),
            |params| gl21::Lightiv(light, pname, params),
            pname,
            params,
        )
    }
    unsafe fn LightModelf(&mut self, pname: GLenum, param: GLfloat) {
        gl21::LightModelf(pname, param)
    }
    unsafe fn LightModelfv(&mut self, pname: GLenum, params: *const GLfloat) {
        gl21::LightModelfv(pname, params)
    }
    unsafe fn Materialf(&mut self, face: GLenum, pname: GLenum, param: GLfloat) {
        assert!(face == gl21::FRONT_AND_BACK);
        MATERIAL_PARAMS.assert_component_count(pname, 1);
        gl21::Materialf(face, pname, param);
    }
    unsafe fn Materialx(&mut self, face: GLenum, pname: GLenum, param: GLfixed) {
        assert!(face == gl21::FRONT_AND_BACK);
        MATERIAL_PARAMS.setx(
            |param| gl21::Materialf(face, pname, param),
            |_| unreachable!(), // no integer parameters exist
            pname,
            param,
        )
    }
    unsafe fn Materialfv(&mut self, face: GLenum, pname: GLenum, params: *const GLfloat) {
        assert!(face == gl21::FRONT_AND_BACK);
        MATERIAL_PARAMS.assert_known_param(pname);
        gl21::Materialfv(face, pname, params);
    }
    unsafe fn Materialxv(&mut self, face: GLenum, pname: GLenum, params: *const GLfixed) {
        assert!(face == gl21::FRONT_AND_BACK);
        MATERIAL_PARAMS.setxv(
            |params| gl21::Materialfv(face, pname, params),
            |_| unreachable!(), // no integer parameters exist
            pname,
            params,
        )
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
    unsafe fn BufferData(
        &mut self,
        target: GLenum,
        size: GLsizeiptr,
        data: *const GLvoid,
        usage: GLenum,
    ) {
        assert!(target == gl21::ARRAY_BUFFER || target == gl21::ELEMENT_ARRAY_BUFFER);
        gl21::BufferData(target, size, data, usage)
    }

    unsafe fn BufferSubData(
        &mut self,
        target: GLenum,
        offset: GLintptr,
        size: GLsizeiptr,
        data: *const GLvoid,
    ) {
        assert!(target == gl21::ARRAY_BUFFER || target == gl21::ELEMENT_ARRAY_BUFFER);
        gl21::BufferSubData(target, offset, size, data)
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
    unsafe fn Color4ub(&mut self, red: GLubyte, green: GLubyte, blue: GLubyte, alpha: GLubyte) {
        gl21::Color4ub(red, green, blue, alpha)
    }
    unsafe fn Normal3f(&mut self, nx: GLfloat, ny: GLfloat, nz: GLfloat) {
        gl21::Normal3f(nx, ny, nz)
    }
    unsafe fn Normal3x(&mut self, nx: GLfixed, ny: GLfixed, nz: GLfixed) {
        gl21::Normal3f(fixed_to_float(nx), fixed_to_float(ny), fixed_to_float(nz))
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
        let mut active_texture: GLenum = 0;
        gl21::GetIntegerv(
            gl21::CLIENT_ACTIVE_TEXTURE,
            &mut active_texture as *mut _ as *mut _,
        );
        if type_ == gles11::FIXED {
            // Translation deferred until draw call.
            // There is one texture co-ordinates pointer per texture unit.
            self.fixed_point_texture_units.insert(active_texture);
            self.pointer_is_fixed_point[2] = true;
            gl21::TexCoordPointer(size, gl21::FLOAT, stride, pointer)
        } else {
            // TODO: byte
            assert!(type_ == gl21::SHORT || type_ == gl21::FLOAT);
            self.fixed_point_texture_units.remove(&active_texture);
            if self.fixed_point_texture_units.is_empty() {
                self.pointer_is_fixed_point[2] = false;
            }
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

        let fixed_point_arrays_state_backup = self.translate_fixed_point_arrays(first, count);

        gl21::DrawArrays(mode, first, count);

        self.restore_fixed_point_arrays(fixed_point_arrays_state_backup);
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

        let fixed_point_arrays_state_backup =
            if self.pointer_is_fixed_point.iter().any(|&is_fixed| is_fixed) {
                // Scan the index buffer to find the range of data that may need
                // fixed-point translation.
                // TODO: Would it be more efficient to turn this into a
                // non-indexed draw-call instead?

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

        if let Some(fixed_point_arrays_state_backup) = fixed_point_arrays_state_backup {
            self.restore_fixed_point_arrays(fixed_point_arrays_state_backup);
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
    unsafe fn PixelStorei(&mut self, pname: GLenum, param: GLint) {
        assert!(pname == gl21::PACK_ALIGNMENT || pname == gl21::UNPACK_ALIGNMENT);
        assert!(param == 1 || param == 2 || param == 4 || param == 8);
        gl21::PixelStorei(pname, param)
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
        gl21::ReadPixels(x, y, width, height, format, type_, pixels)
    }
    unsafe fn GenTextures(&mut self, n: GLsizei, textures: *mut GLuint) {
        gl21::GenTextures(n, textures)
    }
    unsafe fn DeleteTextures(&mut self, n: GLsizei, textures: *const GLuint) {
        gl21::DeleteTextures(n, textures)
    }
    unsafe fn ActiveTexture(&mut self, texture: GLenum) {
        gl21::ActiveTexture(texture)
    }
    unsafe fn IsTexture(&mut self, texture: GLuint) -> GLboolean {
        gl21::IsTexture(texture)
    }
    unsafe fn BindTexture(&mut self, target: GLenum, texture: GLuint) {
        assert!(target == gl21::TEXTURE_2D);
        gl21::BindTexture(target, texture)
    }
    unsafe fn TexParameteri(&mut self, target: GLenum, pname: GLenum, param: GLint) {
        assert!(target == gl21::TEXTURE_2D);
        TEX_PARAMS.assert_known_param(pname);
        gl21::TexParameteri(target, pname, param);
    }
    unsafe fn TexParameterf(&mut self, target: GLenum, pname: GLenum, param: GLfloat) {
        assert!(target == gl21::TEXTURE_2D);
        TEX_PARAMS.assert_known_param(pname);
        gl21::TexParameterf(target, pname, param);
    }
    unsafe fn TexParameterx(&mut self, target: GLenum, pname: GLenum, param: GLfixed) {
        assert!(target == gl21::TEXTURE_2D);
        TEX_PARAMS.setx(
            |param| gl21::TexParameterf(target, pname, param),
            |param| gl21::TexParameteri(target, pname, param),
            pname,
            param,
        )
    }
    unsafe fn TexParameteriv(&mut self, target: GLenum, pname: GLenum, params: *const GLint) {
        assert!(target == gl21::TEXTURE_2D);
        TEX_PARAMS.assert_known_param(pname);
        gl21::TexParameteriv(target, pname, params);
    }
    unsafe fn TexParameterfv(&mut self, target: GLenum, pname: GLenum, params: *const GLfloat) {
        assert!(target == gl21::TEXTURE_2D);
        TEX_PARAMS.assert_known_param(pname);
        gl21::TexParameterfv(target, pname, params);
    }
    unsafe fn TexParameterxv(&mut self, target: GLenum, pname: GLenum, params: *const GLfixed) {
        assert!(target == gl21::TEXTURE_2D);
        TEX_PARAMS.setxv(
            |params| gl21::TexParameterfv(target, pname, params),
            |params| gl21::TexParameteriv(target, pname, params),
            pname,
            params,
        )
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
                || format == gl21::BGRA
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
        assert!(target == gl21::TEXTURE_2D);
        assert!(level >= 0);
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
        gl21::TexSubImage2D(
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
        // OES_compressed_paletted_texture is only in OpenGL ES, so we'll need
        // to decompress those formats.
        } else if let Some(PalettedTextureFormat {
            index_is_nibble,
            palette_entry_format,
            palette_entry_type,
        }) = PalettedTextureFormat::get_info(internalformat)
        {
            // This should be invalid use? (TODO)
            assert!(border == 0);

            let palette_entry_size = match palette_entry_type {
                gl21::UNSIGNED_BYTE => match palette_entry_format {
                    gl21::RGB => 3,
                    gl21::RGBA => 4,
                    _ => unreachable!(),
                },
                gl21::UNSIGNED_SHORT_5_6_5
                | gl21::UNSIGNED_SHORT_4_4_4_4
                | gl21::UNSIGNED_SHORT_5_5_5_1 => 2,
                _ => unreachable!(),
            };
            let palette_entry_count = match index_is_nibble {
                true => 16,
                false => 256,
            };
            let palette_size = palette_entry_size * palette_entry_count;

            let index_count = width as usize * height as usize;
            let (index_word_size, index_word_count) = match index_is_nibble {
                true => (1, (index_count + 1) / 2),
                false => (4, (index_count + 3) / 4),
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
            gl21::TexImage2D(
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
        gl21::CopyTexImage2D(target, level, internalformat, x, y, width, height, border)
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
        assert!(target == gl21::TEXTURE_2D);
        assert!(level >= 0);
        gl21::CopyTexSubImage2D(target, level, xoffset, yoffset, x, y, width, height)
    }
    unsafe fn TexEnvf(&mut self, target: GLenum, pname: GLenum, param: GLfloat) {
        match target {
            gl21::TEXTURE_ENV => {
                TEX_ENV_PARAMS.assert_component_count(pname, 1);
                gl21::TexEnvf(target, pname, param)
            }
            gl21::TEXTURE_FILTER_CONTROL_EXT => {
                assert!(pname == gl21::TEXTURE_LOD_BIAS_EXT);
                gl21::TexEnvf(target, pname, param)
            }
            gl21::POINT_SPRITE => {
                assert!(pname == gl21::COORD_REPLACE);
                gl21::TexEnvf(target, pname, param)
            }
            _ => unimplemented!("TexEnvf target {}", target.to_string()),
        }
    }
    unsafe fn TexEnvx(&mut self, target: GLenum, pname: GLenum, param: GLfixed) {
        match target {
            gl21::TEXTURE_ENV => TEX_ENV_PARAMS.setx(
                |param| gl21::TexEnvf(target, pname, param),
                |param| gl21::TexEnvi(target, pname, param),
                pname,
                param,
            ),
            gl21::TEXTURE_FILTER_CONTROL_EXT => {
                assert!(pname == gl21::TEXTURE_LOD_BIAS_EXT);
                gl21::TexEnvf(target, pname, fixed_to_float(param))
            }
            gl21::POINT_SPRITE => {
                assert!(pname == gl21::COORD_REPLACE);
                gl21::TexEnvf(target, pname, fixed_to_float(param))
            }
            _ => unimplemented!(),
        }
    }
    unsafe fn TexEnvi(&mut self, target: GLenum, pname: GLenum, param: GLint) {
        match target {
            gl21::TEXTURE_ENV => {
                TEX_ENV_PARAMS.assert_component_count(pname, 1);
                gl21::TexEnvi(target, pname, param)
            }
            gl21::TEXTURE_FILTER_CONTROL_EXT => {
                assert!(pname == gl21::TEXTURE_LOD_BIAS_EXT);
                gl21::TexEnvi(target, pname, param)
            }
            gl21::POINT_SPRITE => {
                assert!(pname == gl21::COORD_REPLACE);
                gl21::TexEnvi(target, pname, param)
            }
            gl21::TEXTURE_2D => {
                // This is not a valid TexEnvi target, but we a tolerating it
                // for a Rayman 2 case.
                assert!(pname == gl21::TEXTURE_ENV_MODE);
                log_dbg!(
                    "Tolerating glTexEnvi(GL_TEXTURE_2D, TEXTURE_ENV_MODE, {})",
                    param
                );
                gl21::TexEnvi(target, pname, param)
            }
            _ => unimplemented!("target 0x{:X}, pname 0x{:X}", target, pname),
        }
    }
    unsafe fn TexEnvfv(&mut self, target: GLenum, pname: GLenum, params: *const GLfloat) {
        match target {
            gl21::TEXTURE_ENV => {
                TEX_ENV_PARAMS.assert_known_param(pname);
                gl21::TexEnvfv(target, pname, params)
            }
            gl21::TEXTURE_FILTER_CONTROL_EXT => {
                assert!(pname == gl21::TEXTURE_LOD_BIAS_EXT);
                gl21::TexEnvfv(target, pname, params)
            }
            gl21::POINT_SPRITE => {
                assert!(pname == gl21::COORD_REPLACE);
                gl21::TexEnvfv(target, pname, params)
            }
            _ => unimplemented!(),
        }
    }
    unsafe fn TexEnvxv(&mut self, target: GLenum, pname: GLenum, params: *const GLfixed) {
        match target {
            gl21::TEXTURE_ENV => TEX_ENV_PARAMS.setxv(
                |params| gl21::TexEnvfv(target, pname, params),
                |params| gl21::TexEnviv(target, pname, params),
                pname,
                params,
            ),
            gl21::TEXTURE_FILTER_CONTROL_EXT => {
                assert!(pname == gl21::TEXTURE_LOD_BIAS_EXT);
                let param = fixed_to_float(params.read());
                gl21::TexEnvfv(target, pname, &param)
            }
            gl21::POINT_SPRITE => {
                assert!(pname == gl21::COORD_REPLACE);
                let param = fixed_to_float(params.read());
                gl21::TexEnvfv(target, pname, &param)
            }
            _ => unimplemented!(),
        }
    }
    unsafe fn TexEnviv(&mut self, target: GLenum, pname: GLenum, params: *const GLint) {
        match target {
            gl21::TEXTURE_ENV => {
                TEX_ENV_PARAMS.assert_known_param(pname);
                gl21::TexEnviv(target, pname, params)
            }
            gl21::TEXTURE_FILTER_CONTROL_EXT => {
                assert!(pname == gl21::TEXTURE_LOD_BIAS_EXT);
                gl21::TexEnviv(target, pname, params)
            }
            gl21::POINT_SPRITE => {
                assert!(pname == gl21::COORD_REPLACE);
                gl21::TexEnviv(target, pname, params)
            }
            _ => unimplemented!(),
        }
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
    unsafe fn FramebufferTexture2DOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        textarget: GLenum,
        texture: GLuint,
        level: i32,
    ) {
        gl21::FramebufferTexture2DEXT(target, attachment, textarget, texture, level)
    }
    unsafe fn GetFramebufferAttachmentParameterivOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gl21::GetFramebufferAttachmentParameterivEXT(target, attachment, pname, params)
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
    unsafe fn DeleteFramebuffersOES(&mut self, n: GLsizei, framebuffers: *const GLuint) {
        gl21::DeleteFramebuffersEXT(n, framebuffers)
    }
    unsafe fn DeleteRenderbuffersOES(&mut self, n: GLsizei, renderbuffers: *const GLuint) {
        gl21::DeleteRenderbuffersEXT(n, renderbuffers)
    }
    unsafe fn GenerateMipmapOES(&mut self, target: GLenum) {
        gl21::GenerateMipmapEXT(target)
    }
}
