/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Shared utilities.

use super::gles11_raw as gles11; // constants only
use super::gles11_raw::types::{GLenum, GLint, GLsizei};
use super::GLES;

/// Helper for implementing `glCompressedTexImage2D`: if `internalformat` is
/// one of the `IMG_texture_compression_pvrtc` formats, decode it and call
/// `glTexImage2D`. Returns `true` if this is done.
///
/// Note that this panics rather than create GL errors for invalid use (TODO?)
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

    assert!(border == 0);
    let pixels = crate::image::decode_pvrtc(
        pvrtc_data,
        is_2bit,
        width.try_into().unwrap(),
        height.try_into().unwrap(),
    );
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
