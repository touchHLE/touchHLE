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
            // EXT_framebuffer_blit gives us glBlitFramebufferEXT plus the
            // GL_READ_FRAMEBUFFER_EXT / GL_DRAW_FRAMEBUFFER_EXT targets that
            // map 1:1 to the values used by GL_APPLE_framebuffer_multisample's
            // GL_READ_FRAMEBUFFER_APPLE / GL_DRAW_FRAMEBUFFER_APPLE constants.
            // We need this to implement glResolveMultisampleFramebufferAPPLE
            // (i.e. blit the multisample sample-FB into the resolve-FB whose
            // color renderbuffer is the CAEAGLLayer drawable).
            "GL_EXT_framebuffer_blit",
            // Required so the underlying GL accepts multisample renderbuffer
            // storage if/when we choose to honour the requested sample count.
            "GL_EXT_framebuffer_multisample",
            "GL_EXT_texture_filter_anisotropic",
            "GL_EXT_texture_lod_bias",
            "GL_ARB_matrix_palette",
            "GL_ARB_vertex_blend",
            "GL_EXT_blend_subtract",
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
            "GL_OES_blend_subtract",
            // Per-vertex point sizes — used by ES 1.1 particle/sprite code.
            "GL_OES_point_size_array",
            // Vertex array objects bound by ES 1.1 apps via the OES suffix
            // are part of the OES_vertex_array_object extension; expose the
            // entry points so native backends can forward them.
            "GL_OES_vertex_array_object",
        ],
    )
    .write_bindings(GlobalGenerator, &mut file)
    .unwrap();

    let mut file = File::create(out_dir.join("gles2.rs")).unwrap();
    Registry::new(
        Api::Gles2,
        (2, 0),
        Profile::Core,
        Fallbacks::None,
        [
            "GL_OES_rgb8_rgba8",
            "GL_OES_depth24",
            "GL_OES_packed_depth_stencil",
            "GL_EXT_texture_filter_anisotropic",
            "GL_IMG_texture_compression_pvrtc",
            "GL_EXT_texture_format_BGRA8888",
            "GL_OES_mapbuffer",
            "GL_OES_vertex_array_object",
        ],
    )
    .write_bindings(GlobalGenerator, &mut file)
    .unwrap();

    // OpenGL ES 3.0 — full Core profile. This is what `kEAGLRenderingAPIOpenGLES3`
    // routes to. ES 3.0 is a strict superset of ES 2.0 in terms of API surface
    // and re-exposes everything ES 2.0 provides plus VAOs, transform feedback,
    // sampler objects, multiple render targets, integer attributes/uniforms,
    // UBOs, instanced rendering, sync objects, 3D textures, immutable storage,
    // pixel pack/unpack buffers, `glClearBuffer*`, etc.
    let mut file = File::create(out_dir.join("gles30.rs")).unwrap();
    Registry::new(
        Api::Gles2,
        (3, 0),
        Profile::Core,
        Fallbacks::None,
        [
            // Hardware-accelerated PVRTC is still relevant on real iOS GPUs
            // (the only format Apple's hardware compresses natively).
            "GL_IMG_texture_compression_pvrtc",
            // BGRA upload paths are pervasive in iPhone OS apps and Apple's
            // PowerVR drivers expose them as a core upload format. ES 3.0
            // doesn't subsume this extension by default.
            "GL_EXT_texture_format_BGRA8888",
            // Anisotropy is exposed by every desktop/mobile driver.
            "GL_EXT_texture_filter_anisotropic",
            // OES_mapbuffer is the legacy `glMapBufferOES`/`glUnmapBufferOES`
            // entry points used by ES 1.1 / ES 2.0 apps; on most ES 3.0
            // drivers they are still exported alongside the core
            // `glMapBufferRange`. Include them so guest code that drives an
            // ES 3.0 backend via an ES-1.1-shaped API can still map buffers.
            "GL_OES_mapbuffer",
        ],
    )
    .write_bindings(GlobalGenerator, &mut file)
    .unwrap();

    // Desktop OpenGL 3.3 Core profile — the fallback backend on hosts that
    // lack a native OpenGL ES 3.0 driver (most x86 Linux/macOS desktops).
    // 3.3 Core is the lowest desktop version that exposes all of ES 3.0's
    // mandatory feature surface (transform feedback, sampler objects,
    // instanced rendering, integer attributes / uniforms, sync objects,
    // immutable texture storage via `EXT_texture_storage`).
    let mut file = File::create(out_dir.join("gl33core.rs")).unwrap();
    Registry::new(
        Api::Gl,
        (3, 3),
        Profile::Core,
        Fallbacks::None,
        [
            // Immutable storage was promoted to GL 4.2 core but every modern
            // 3.3 driver advertises this extension; we use it for
            // `glTexStorage2D` / `glTexStorage3D`.
            "GL_ARB_texture_storage",
            // Anisotropy: every modern driver exposes it.
            "GL_EXT_texture_filter_anisotropic",
            // For `glInvalidateFramebuffer` (promoted to GL 4.3 core).
            "GL_ARB_invalidate_subdata",
            // For `glGetInternalformativ` (promoted to GL 4.2 core).
            "GL_ARB_internalformat_query",
            // For `glProgramBinary` / `glGetProgramBinary` (promoted to GL 4.1
            // core).
            "GL_ARB_get_program_binary",
            // For `glClearBuffer*` consistency on older drivers.
            "GL_EXT_draw_buffers2",
            // For transform feedback objects (`glBindTransformFeedback`,
            // `glGen/DeleteTransformFeedbacks`, `glIsTransformFeedback`,
            // `glPause/ResumeTransformFeedback`). Promoted to GL 4.0 core
            // but every desktop 3.3 driver exposes the extension.
            "GL_ARB_transform_feedback2",
            // For `glDrawArraysInstanced`/`glDrawElementsInstanced`. Promoted
            // to GL 3.1 core but still exposed by the extension on 3.3.
            "GL_ARB_draw_instanced",
            // For `glVertexAttribDivisor`. Promoted to GL 3.3 core, but listing
            // the extension keeps the binding source self-documenting.
            "GL_ARB_instanced_arrays",
            // For `glClearDepthf` / `glDepthRangef` / `glShaderBinary` /
            // `glReleaseShaderCompiler` / `glGetShaderPrecisionFormat` and
            // for the `GL_LOW_FLOAT`/`GL_HIGH_FLOAT`/... precision-format
            // enums used when reporting shader precision to ES 2/3 apps.
            "GL_ARB_ES2_compatibility",
        ],
    )
    .write_bindings(GlobalGenerator, &mut file)
    .unwrap();
}
