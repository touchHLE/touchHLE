/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! OpenGL ES abstraction and implementations.
//!
//! touchHLE uses OpenGL ES for several things. OpenGL ES is part of iPhone OS's
//! API surface and can be used by apps for rendering, so there must be an
//! implementation of it to expose to the app. Beyond that, there are various
//! internal uses for which any graphics API would work, but using the same one
//! makes things simpler:
//! - Presenting frames rendered by the app to the screen, with appropriate
//!   rotation and scaling.
//! - Drawing touchHLE's virtual cursor.
//! - Drawing the app's splash screen.
//! - Compositing the app's Core Animation layers (usually for UIKit views).
//!
//! touchHLE's OpenGL ES implementation consists of a series of layers. This
//! module contains the layers that aren't specific to a particular use:
//!
//! - [gles_generic] provides an abstraction over OpenGL ES implementations.
//! - [angle] provides OpenGL ES 1.1 and 2.0 contexts on top of ANGLE's Metal
//!   backend. All contexts share one share group (see [crate::window]), so GL
//!   objects can cross between ES 1.1 and 2.0 contexts.
//! - [gles11_raw] and [gles2_raw] provide raw bindings for OpenGL ES 1.1 and
//!   2.0 generated from the Khronos API headers. **The function bindings are
//!   only for use within this module.** The constants and types can be used
//!   outside it, however.
//! - [present] provides utilities for presenting frames to the window using an
//!   abstract OpenGL ES implementation.
//!
//! In contrast, [crate::frameworks::opengles] is a layer specific to OpenGL
//! ES's role as a part of the iPhone OS API surface. It wraps [gles_generic] to
//! expose OpenGL ES to the guest app.
//!
//! Useful resources for OpenGL ES 1.1:
//! - [Reference pages](https://registry.khronos.org/OpenGL-Refpages/es1.1/xhtml/)
//! - [Specification](https://registry.khronos.org/OpenGL/specs/es/1.1/es_full_spec_1.1.pdf)
//! - Apple's [OpenGL ES Hardware Platform Guide for iOS](https://developer.apple.com/library/archive/documentation/OpenGLES/Conceptual/OpenGLESHardwarePlatformGuide_iOS/OpenGLESPlatforms/OpenGLESPlatforms.html)
//! - Extensions:
//!   - [OES_framebuffer_object](https://registry.khronos.org/OpenGL/extensions/OES/OES_framebuffer_object.txt)
//!   - [IMG_texture_compression_pvrtc](https://registry.khronos.org/OpenGL/extensions/IMG/IMG_texture_compression_pvrtc.txt)
//!   - [OES_compressed_paletted_texture](https://registry.khronos.org/OpenGL/extensions/OES/OES_compressed_paletted_texture.txt) (also incorporated into the main spec)
//!   - [OES_matrix_palette](https://registry.khronos.org/OpenGL/extensions/OES/OES_matrix_palette.txt)
//!   - [EXT_texture_format_BGRA8888](https://registry.khronos.org/OpenGL/extensions/EXT/EXT_texture_format_BGRA8888.txt)
//!   - [OES_blend_subtract](https://registry.khronos.org/OpenGL/extensions/OES/OES_blend_subtract.txt)
//!
//! Useful resources for OpenGL 2.1:
//! - [Reference pages](https://registry.khronos.org/OpenGL-Refpages/gl2.1/)
//! - [Specification](https://registry.khronos.org/OpenGL/specs/gl/glspec21.pdf)
//! - Extensions:
//!   - [EXT_framebuffer_object](https://registry.khronos.org/OpenGL/extensions/EXT/EXT_framebuffer_object.txt)
//!   - [ARB_matrix_palette](https://registry.khronos.org/OpenGL/extensions/ARB/ARB_matrix_palette.txt)
//!   - [ARB_vertex_blend](https://registry.khronos.org/OpenGL/extensions/ARB/ARB_vertex_blend.txt)
//!   - [EXT_blend_subtract](https://registry.khronos.org/OpenGL/extensions/EXT/EXT_blend_subtract.txt)
//!
//! Useful resources for both:
//! - Extensions:
//!   - [EXT_texture_filter_anisotropic](https://registry.khronos.org/OpenGL/extensions/EXT/EXT_texture_filter_anisotropic.txt)
//!   - [EXT_texture_lod_bias](https://registry.khronos.org/OpenGL/extensions/EXT/EXT_texture_lod_bias.txt)

pub mod angle;
mod gles_generic;
pub mod present;
mod util;

pub use touchHLE_gl_bindings::gles2 as gles2_raw;
pub use touchHLE_gl_bindings::gles11 as gles11_raw;

use crate::environment::Environment;
use crate::window::GLVersion;
use angle::ANGLEContext;
pub use gles_generic::{ArrayInfo, GLESContext, GLES, ARRAYS};

/// Create an OpenGL ES 1.1 context via ANGLE, panicking on failure.
pub fn create_gles1_ctx(env: &mut Environment) -> Box<dyn GLESContext> {
    env.on_parent_stack_in_coroutine(|window, _options| create_gles1_ctx_no_parent_stack(window))
}

/// Create an OpenGL ES 2.0 context via ANGLE, panicking on failure.
pub fn create_gles2_ctx(env: &mut Environment) -> Box<dyn GLESContext> {
    env.on_parent_stack_in_coroutine(|window, _options| {
        assert!(window.on_main_stack());
        log!("Creating an OpenGL ES 2.0 context ({}):", ANGLEContext::description());
        match ANGLEContext::new_with_version(window, GLVersion::GLES20) {
            Ok(ctx) => {
                log!("=> Success!");
                let boxed: Box<dyn GLESContext> = Box::new(ctx);
                boxed
            }
            Err(err) => panic!("Couldn't create OpenGL ES 2.0 context: {}", err),
        }
    })
}

/// Same as [create_gles1_ctx], but without calling
/// [Environment::on_parent_stack_in_coroutine]. Only should be called by
/// functions not inside a coroutine that can't use [Environment].
pub fn create_gles1_ctx_no_parent_stack(window: &mut crate::window::Window) -> Box<dyn GLESContext> {
    assert!(window.on_main_stack());
    log!("Creating an OpenGL ES 1.1 context ({}):", ANGLEContext::description());
    match ANGLEContext::new_with_version(window, GLVersion::GLES11) {
        Ok(ctx) => {
            log!("=> Success!");
            Box::new(ctx)
        }
        Err(err) => panic!("Couldn't create OpenGL ES 1.1 context: {}", err),
    }
}
