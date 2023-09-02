/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use gl_generator::{Api, Fallbacks, GlobalGenerator, Profile, Registry};
use std::fs::File;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let mut file = File::create(out_dir.join("gl21compat.rs")).unwrap();
    Registry::new(
        Api::Gl,
        (2, 1),
        Profile::Compatibility,
        Fallbacks::None,
        [
            "GL_EXT_framebuffer_object",
            "GL_EXT_texture_filter_anisotropic",
            "GL_EXT_texture_lod_bias",
            "GL_ARB_matrix_palette",
            "GL_ARB_vertex_blend",
        ],
    )
    .write_bindings(GlobalGenerator, &mut file)
    .unwrap();

    let mut file = File::create(out_dir.join("gles11.rs")).unwrap();
    Registry::new(
        Api::Gles1,
        (1, 1),
        Profile::Core,
        Fallbacks::None,
        [
            "GL_OES_framebuffer_object",
            "GL_OES_rgb8_rgba8",
            "GL_EXT_texture_filter_anisotropic",
            "GL_IMG_texture_compression_pvrtc",
            "GL_EXT_texture_lod_bias",
            "GL_EXT_texture_format_BGRA8888",
            "GL_OES_draw_texture",
            "GL_OES_mapbuffer",
            // Part of the OpenGL ES 1.1 common profile.
            "GL_OES_compressed_paletted_texture",
            "GL_OES_matrix_palette",
        ],
    )
    .write_bindings(GlobalGenerator, &mut file)
    .unwrap();
}
