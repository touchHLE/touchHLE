/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Shared utilities.

use super::gles11_raw as gles11; // constants only
use super::gles11_raw::types::{GLenum, GLfixed, GLfloat, GLint, GLsizei};
use super::GLES;

/// Convert a fixed-point scalar to a floating-point scalar.
///
/// Beware: Rust's type checker won't complain if you mix up [GLfixed] with
/// [GLint], but they have very different meanings.
pub fn fixed_to_float(fixed: GLfixed) -> GLfloat {
    ((fixed as f64) / ((1 << 16) as f64)) as f32
}

/// Convert a floating-point scalar to a fixed-point (16.16) scalar, saturating
/// on overflow. Used to implement `glGetFixedv` on top of a backend that only
/// exposes floating-point state.
pub fn float_to_fixed(float: GLfloat) -> GLfixed {
    let scaled = (float as f64) * ((1 << 16) as f64);
    if scaled >= GLfixed::MAX as f64 {
        GLfixed::MAX
    } else if scaled <= GLfixed::MIN as f64 {
        GLfixed::MIN
    } else if scaled.is_nan() {
        0
    } else {
        scaled.round() as GLfixed
    }
}

/// Convert a fixed-point 4-by-4 matrix to floating-point.
pub unsafe fn matrix_fixed_to_float(m: *const GLfixed) -> [GLfloat; 16] {
    let mut matrix = [0f32; 16];
    for (i, cell) in matrix.iter_mut().enumerate() {
        *cell = fixed_to_float(m.add(i).read_unaligned());
    }
    matrix
}

/// Type of a parameter, used in [ParamTable].
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ParamType {
    /// `GLboolean`
    Boolean,
    /// `GLfloat`
    Float,
    /// `GLint`
    Int,
    /// Placeholder type for things like colors which are floating-point
    /// but don't have the usual conversion behavior to/from integers etc.
    /// [ParamTable] will accept it for floating-point inputs only.
    /// TODO: Remove this and add proper types for colors etc.
    FloatSpecial,
    /// Hack to achieve `#[non_exhaustive]`-like behavior within this crate,
    /// since more types might be added in future
    _NonExhaustive,
}

/// Table of parameter names, component types and component counts.
///
/// This is a helper for implementing the common pattern in OpenGL where a set
/// of parameters named by [GLenum] values can be accessed via functions with
/// suffixes like `f`, `fv`, `i`, `iv`, etc.
pub struct ParamTable(pub &'static [(GLenum, ParamType, u8)]);

impl ParamTable {
    /// Look up the component type and count for a parameter. Returns a safe
    /// default (`ParamType::Float`, 1) for unknown names rather than panicking
    /// the host, so misbehaving guest code only loses correctness for the
    /// specific call rather than tearing the whole emulator down.
    pub fn get_type_info(&self, pname: GLenum) -> (ParamType, u8) {
        match self.0.iter().find(|&&(pname2, _, _)| pname == pname2) {
            Some(&(_, type_, count)) => (type_, count),
            None => {
                log!(
                    "Warning: ParamTable::get_type_info: unhandled parameter name {pname:#x}; defaulting to (Float, 1)."
                );
                (ParamType::Float, 1)
            }
        }
    }

    /// Assert that a parameter name is recognized.
    pub fn assert_known_param(&self, pname: GLenum) {
        self.get_type_info(pname);
    }

    pub fn contains(&self, pname: GLenum) -> bool {
        self.0.iter().any(|(pname2, _, _)| pname == *pname2)
    }

    /// Check that a parameter name is recognized and that the parameter has a
    /// particular component count. Logs a warning instead of panicking the
    /// host on mismatch, so the worst case is just a malformed GL call.
    pub fn assert_component_count(&self, pname: GLenum, provided_count: u8) {
        let (_type, actual_count) = self.get_type_info(pname);
        if actual_count != provided_count {
            log!(
                "Warning: ParamTable::assert_component_count: parameter {pname:#x} has component count {actual_count}, {provided_count} given; continuing anyway."
            );
        }
    }

    /// Implements a fixed-point scalar (`x`) setter by calling a provided
    /// floating-point scalar (`f`) or integer scalar (`i`) setter as
    /// as appropriate.
    ///
    /// This will panic if the name is not recognized or the parameter is not
    /// a scalar.
    pub unsafe fn setx<FF, FI>(&self, setf: FF, seti: FI, pname: GLenum, param: GLfixed)
    where
        FF: FnOnce(GLfloat),
        FI: FnOnce(GLint),
    {
        let (type_, component_count) = self.get_type_info(pname);
        assert!(component_count == 1);
        // Yes, the OpenGL standard lets you mismatch types. Yes, it requires
        // an implicit conversion. Yes, it requires no scaling of fixed-point
        // values when converting to integer. :(
        // On the other hand, fixed-to-float/float-to-fixed conversion is always
        // the same even for the weird float-ish values.
        match type_ {
            ParamType::Float | ParamType::FloatSpecial => setf(fixed_to_float(param)),
            _ => seti(param),
        }
    }

    /// Implements a fixed-point vector (`xv`) setter by calling a provided
    /// floating-point vector (`fv`) or integer vector (`iv`) setter as
    /// as appropriate.
    ///
    /// This will panic if the name is not recognized.
    pub unsafe fn setxv<FFV, FIV>(
        &self,
        setfv: FFV,
        setiv: FIV,
        pname: GLenum,
        params: *const GLfixed,
    ) where
        FFV: FnOnce(*const GLfloat),
        FIV: FnOnce(*const GLint),
    {
        let (type_, count) = self.get_type_info(pname);
        // Yes, the OpenGL standard is like this (see above).
        match type_ {
            // Fixed-to-float/float-to-fixed conversion is always the same even
            // for the weird float-ish values.
            ParamType::Float | ParamType::FloatSpecial => {
                let mut params_float = [0.0; 16]; // probably the max?
                let params_float = &mut params_float[..usize::from(count)];
                for (i, param_float) in params_float.iter_mut().enumerate() {
                    *param_float = fixed_to_float(params.add(i).read())
                }
                setfv(params_float.as_ptr())
            }
            _ => setiv(params),
        }
    }
}

/// Helper for implementing `glCompressedTexImage2D`: if `internalformat` is
/// one of the `IMG_texture_compression_pvrtc` formats, decode it via the
/// software PVRTC decoder and call `glTexImage2D` to upload the resulting
/// RGBA8 texture. Returns:
/// * `true` if the format was a PVRTC variant and the upload was attempted
///   (regardless of whether the upload itself succeeded — that's between the
///   driver and the caller).
/// * `false` if `internalformat` is not a PVRTC variant; the caller is then
///   responsible for handling the format some other way (passthrough,
///   paletted-texture decode, ...).
///
/// Previously this helper used `assert!` for `border == 0` and for the
/// payload-size check inside `decode_pvrtc`. Real-world iPhone OS games
/// occasionally pass in slightly truncated payloads (broken builds, in-app
/// procedural textures with off-by-one mip sizes, third-party loaders that
/// trim trailing zeros), and a panic in the present pipeline brings down the
/// whole host process. Treat malformed input as a soft failure: log once and
/// return `true` *without* uploading anything, so the caller doesn't
/// re-attempt (the data is unusable either way) and the rest of the frame
/// can still draw.
#[allow(clippy::too_many_arguments)]
pub fn try_decode_pvrtc(
    gles: &mut dyn GLES,
    target: GLenum,
    level: GLint,
    internalformat: GLenum,
    width: GLsizei,
    height: GLsizei,
    border: GLint,
    pvrtc_data: &[u8],
) -> bool {
    let is_2bit = match internalformat {
        gles11::COMPRESSED_RGB_PVRTC_4BPPV1_IMG | gles11::COMPRESSED_RGBA_PVRTC_4BPPV1_IMG => false,
        gles11::COMPRESSED_RGB_PVRTC_2BPPV1_IMG | gles11::COMPRESSED_RGBA_PVRTC_2BPPV1_IMG => true,
        _ => return false,
    };

    if border != 0 {
        log!(
            "Warning: try_decode_pvrtc: invalid non-zero border ({border}) for PVRTC \
             upload {width}x{height} (level {level}, format {internalformat:#x}); \
             skipping upload."
        );
        return true;
    }

    let Ok(width_u) = u32::try_from(width) else {
        log!(
            "Warning: try_decode_pvrtc: invalid width {width} for PVRTC upload \
             (level {level}, format {internalformat:#x}); skipping upload."
        );
        return true;
    };
    let Ok(height_u) = u32::try_from(height) else {
        log!(
            "Warning: try_decode_pvrtc: invalid height {height} for PVRTC upload \
             (level {level}, format {internalformat:#x}); skipping upload."
        );
        return true;
    };

    // The IMG_texture_compression_pvrtc spec specifies a fixed payload size
    // for any given (is_2bit, width, height) tuple. Validate it here so that
    // a short / oversized input from the guest doesn't panic the host
    // PVRTC decoder. (`decode_pvrtc` itself still asserts internally as
    // a defence-in-depth measure, but we shouldn't rely on that — see the
    // function-level doc comment.)
    let expected_size = if is_2bit {
        (width_u.max(16) as usize * height_u.max(8) as usize * 2).div_ceil(8)
    } else {
        (width_u.max(8) as usize * height_u.max(8) as usize * 4).div_ceil(8)
    };
    if pvrtc_data.len() != expected_size {
        log!(
            "Warning: try_decode_pvrtc: PVRTC payload size mismatch for \
             {width}x{height} (level {level}, format {internalformat:#x}, \
             is_2bit={is_2bit}): got {} bytes, expected {expected_size}; \
             skipping upload.",
            pvrtc_data.len(),
        );
        return true;
    }

    let pixels = crate::image::decode_pvrtc(pvrtc_data, is_2bit, width_u, height_u);
    unsafe {
        gles.TexImage2D(
            target,
            level,
            gles11::RGBA as _,
            width,
            height,
            border,
            gles11::RGBA,
            gles11::UNSIGNED_BYTE,
            pixels.as_ptr() as *const _,
        )
    };
    true
}

pub struct PalettedTextureFormat {
    /// * `true` for 4-bit (nibble) index, 16-color palette.
    /// * `false` for 8-bit (byte) index, 256-color palette.
    pub index_is_nibble: bool,
    /// `glTexImage2D`-style `format` for palette entries: `GL_RGB` or `GL_RGBA`
    pub palette_entry_format: GLenum,
    /// `glTexImage2D`-style `type` for palette entries: `GL_UNSIGNED_BYTE` or
    /// some `GL_UNSIGNED_SHORT_` value
    pub palette_entry_type: GLenum,
}
impl PalettedTextureFormat {
    /// If the provided format is from `OES_compressed_paletted_texture`,
    /// return [Some] with information about it, or [None] otherwise.
    pub fn get_info(internalformat: GLenum) -> Option<Self> {
        match internalformat {
            gles11::PALETTE4_RGB8_OES => Some(Self {
                index_is_nibble: true,
                palette_entry_format: gles11::RGB,
                palette_entry_type: gles11::UNSIGNED_BYTE,
            }),
            gles11::PALETTE4_RGBA8_OES => Some(Self {
                index_is_nibble: true,
                palette_entry_format: gles11::RGBA,
                palette_entry_type: gles11::UNSIGNED_BYTE,
            }),
            gles11::PALETTE4_R5_G6_B5_OES => Some(Self {
                index_is_nibble: true,
                palette_entry_format: gles11::RGB,
                palette_entry_type: gles11::UNSIGNED_SHORT_5_6_5,
            }),
            gles11::PALETTE4_RGBA4_OES => Some(Self {
                index_is_nibble: true,
                palette_entry_format: gles11::RGBA,
                palette_entry_type: gles11::UNSIGNED_SHORT_4_4_4_4,
            }),
            gles11::PALETTE4_RGB5_A1_OES => Some(Self {
                index_is_nibble: true,
                palette_entry_format: gles11::RGBA,
                palette_entry_type: gles11::UNSIGNED_SHORT_5_5_5_1,
            }),
            gles11::PALETTE8_RGB8_OES => Some(Self {
                index_is_nibble: false,
                palette_entry_format: gles11::RGB,
                palette_entry_type: gles11::UNSIGNED_BYTE,
            }),
            gles11::PALETTE8_RGBA8_OES => Some(Self {
                index_is_nibble: false,
                palette_entry_format: gles11::RGBA,
                palette_entry_type: gles11::UNSIGNED_BYTE,
            }),
            gles11::PALETTE8_R5_G6_B5_OES => Some(Self {
                index_is_nibble: false,
                palette_entry_format: gles11::RGB,
                palette_entry_type: gles11::UNSIGNED_SHORT_5_6_5,
            }),
            gles11::PALETTE8_RGBA4_OES => Some(Self {
                index_is_nibble: false,
                palette_entry_format: gles11::RGBA,
                palette_entry_type: gles11::UNSIGNED_SHORT_4_4_4_4,
            }),
            gles11::PALETTE8_RGB5_A1_OES => Some(Self {
                index_is_nibble: false,
                palette_entry_format: gles11::RGBA,
                palette_entry_type: gles11::UNSIGNED_SHORT_5_5_5_1,
            }),
            _ => None,
        }
    }
}
