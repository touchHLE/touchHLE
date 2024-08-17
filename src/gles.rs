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
//! - Various modules provide implementations:
//!   - [gles1_native] passes through native OpenGL ES 1.1.
//!   - [gles1_on_gl2] provides an implementation of OpenGL ES 1.1 using OpenGL
//!     2.1 compatibility profile.
//!   - There might be more in future.
//! - [gles11_raw] provides raw bindings for OpenGL ES 1.1 generated from the
//!   Khronos API headers. **The function bindings are only for use within this
//!   module.** The constants and types can be used outside it, however.
//!   - [gl21compat_raw] is the same thing, but for OpenGL 2.1 compatibility
//!     profile, which can't be used outside this module at all.
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
//!
//! Useful resources for OpenGL 2.1:
//! - [Reference pages](https://registry.khronos.org/OpenGL-Refpages/gl2.1/)
//! - [Specification](https://registry.khronos.org/OpenGL/specs/gl/glspec21.pdf)
//! - Extensions:
//!   - [EXT_framebuffer_object](https://registry.khronos.org/OpenGL/extensions/EXT/EXT_framebuffer_object.txt)
//!   - [ARB_matrix_palette](https://registry.khronos.org/OpenGL/extensions/ARB/ARB_matrix_palette.txt)
//!   - [ARB_vertex_blend](https://registry.khronos.org/OpenGL/extensions/ARB/ARB_vertex_blend.txt)
//!
//! Useful resources for both:
//! - Extensions:
//!   - [EXT_texture_filter_anisotropic](https://registry.khronos.org/OpenGL/extensions/EXT/EXT_texture_filter_anisotropic.txt)
//!   - [EXT_texture_lod_bias](https://registry.khronos.org/OpenGL/extensions/EXT/EXT_texture_lod_bias.txt)

pub mod gles1_native;
pub mod gles1_on_gl2;
mod gles_generic;
pub mod present;
mod util;

use touchHLE_gl_bindings::gl21compat as gl21compat_raw;
pub use touchHLE_gl_bindings::gles11 as gles11_raw;

use gles1_native::GLES1Native;
use gles1_on_gl2::GLES1OnGL2;
pub use gles_generic::GLES;

/// Labels for [GLES] implementations and an abstraction for constructing them.
#[derive(Copy, Clone)]
pub enum GLESImplementation {
    /// [GLES1Native].
    GLES1Native,
    /// [GLES1OnGL2].
    GLES1OnGL2,
}
impl GLESImplementation {
    /// List of OpenGL ES 1.1 implementations in order of preference.
    pub const GLES1_IMPLEMENTATIONS: &'static [Self] = &[Self::GLES1Native, Self::GLES1OnGL2];
    /// Convert from short name used for command-line arguments. Returns [Err]
    /// if name is not recognized..
    pub fn from_short_name(name: &str) -> Result<Self, ()> {
        match name {
            "gles1_on_gl2" => Ok(Self::GLES1OnGL2),
            "gles1_native" => Ok(Self::GLES1Native),
            _ => Err(()),
        }
    }
    /// See [GLES::description].
    pub fn description(self) -> &'static str {
        match self {
            Self::GLES1Native => GLES1Native::description(),
            Self::GLES1OnGL2 => GLES1OnGL2::description(),
        }
    }
    /// See [GLES::new].
    pub fn construct(self, window: &mut crate::window::Window) -> Result<Box<dyn GLES>, String> {
        fn boxer<T: GLES + 'static>(ctx: T) -> Box<dyn GLES> {
            Box::new(ctx)
        }
        match self {
            Self::GLES1Native => GLES1Native::new(window).map(boxer),
            Self::GLES1OnGL2 => GLES1OnGL2::new(window).map(boxer),
        }
    }
}

/// Try to create an OpenGL ES 1.1 context using the configured strategies,
/// panicking on failure.
pub fn create_gles1_ctx(
    window: &mut crate::window::Window,
    options: &crate::options::Options,
) -> Box<dyn GLES> {
    log!("Creating an OpenGL ES 1.1 context:");
    let list = if let Some(ref preference) = options.gles1_implementation {
        std::slice::from_ref(preference)
    } else {
        GLESImplementation::GLES1_IMPLEMENTATIONS
    };
    let mut gles1_ctx = None;
    for implementation in list {
        log!("Trying: {}", implementation.description());
        match implementation.construct(window) {
            Ok(ctx) => {
                log!("=> Success!");
                gles1_ctx = Some(ctx);
                break;
            }
            Err(err) => {
                log!("=> Failed: {}.", err);
            }
        }
    }
    gles1_ctx.expect("Couldn't create OpenGL ES 1.1 context!")
}
