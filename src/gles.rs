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

pub mod gles1_native;
pub mod gles1_on_gl2;
pub mod gles2_glsl;
pub mod gles2_native;
pub mod gles2_on_gl3;
pub mod gles3_native;
pub mod gles3_on_gl3;
mod gles_generic;
pub mod present;
mod util;
use touchHLE_gl_bindings::gl21compat as gl21compat_raw;
use touchHLE_gl_bindings::gl33core as gl33core_raw;
pub use touchHLE_gl_bindings::gles11 as gles11_raw;
pub use touchHLE_gl_bindings::gles2 as gles2_raw;
pub use touchHLE_gl_bindings::gles30 as gles30_raw;

use crate::environment::Environment;
use gles1_native::GLES1NativeContext;
use gles1_on_gl2::GLES1OnGL2Context;
use gles2_native::GLES2NativeContext;
use gles2_on_gl3::GLES2OnGL3Context;
use gles3_native::GLES3NativeContext;
use gles3_on_gl3::GLES3OnGL3Context;
pub use gles_generic::GLESContext;
pub use gles_generic::GLES;

/// Labels for [GLES] implementations and an abstraction for constructing them.
#[derive(Copy, Clone)]
pub enum GLESImplementation {
    /// [gles1_native::GLES1Native].
    GLES1Native,
    /// [gles1_on_gl2::GLES1OnGL2].
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
    /// See [GLESContext::description].
    pub fn description(self) -> &'static str {
        match self {
            Self::GLES1Native => GLES1NativeContext::description(),
            Self::GLES1OnGL2 => GLES1OnGL2Context::description(),
        }
    }
    /// See [GLESContext::new].
    pub fn construct(
        self,
        window: &mut crate::window::Window,
    ) -> Result<Box<dyn GLESContext>, String> {
        fn boxer<T: GLESContext + 'static>(ctx: T) -> Box<dyn GLESContext> {
            Box::new(ctx)
        }
        match self {
            Self::GLES1Native => GLES1NativeContext::new(window).map(boxer),
            Self::GLES1OnGL2 => GLES1OnGL2Context::new(window).map(boxer),
        }
    }
}

/// Try to create an OpenGL ES 1.1 context using the configured strategies,
/// panicking on failure.
pub fn create_gles1_ctx(env: &mut Environment) -> Box<dyn GLESContext> {
    env.on_parent_stack_in_coroutine(|window, options| {
        create_gles1_ctx_no_parent_stack(window, options)
    })
}

/// Try to create an OpenGL ES 2.0 context, panicking on failure.
///
/// The preference order, from "most-correct" to "only as a last resort":
///
/// 1. [`GLES2NativeContext`] — a real OpenGL ES 2.0 driver. This is the
///    only thing that works on platforms without desktop OpenGL such as
///    Android, and on real iOS hardware emulation. Every ES 2.0 entry point
///    is a direct passthrough to the host driver.
/// 2. [`GLES2OnGL3Context`] — a full ES 2.0 backend built on top of
///    desktop OpenGL 3.3 Core. This shares its implementation with the ES
///    3.0 fallback ([`GLES3OnGL3Context`]), giving us a single source of
///    truth for ES 2.0 / ES 3.0 emulation, full shader support, and proper
///    GLSL ES → desktop GLSL translation via
///    [`gles2_glsl::translate_glsl_es_to_120`]. This is the preferred
///    fallback on x86 Linux/macOS desktops where Mesa lacks a native ES 2.0
///    surface.
/// 3. [`GLES1OnGL2Context`] — legacy fallback that piggy-backs on a desktop
///    OpenGL 2.1 compatibility profile context. Only used on the rare host
///    that has GL 2.1 compat but no GL 3.3 Core (e.g. very old macOS
///    installations); kept around for backwards compatibility.
pub fn create_gles2_ctx(env: &mut Environment) -> Box<dyn GLESContext> {
    env.on_parent_stack_in_coroutine(|window, _options| {
        assert!(window.on_main_stack());
        log!("Creating an OpenGL ES 2.0 context:");

        log!("Trying: {}", GLES2NativeContext::description());
        match GLES2NativeContext::new(window) {
            Ok(ctx) => {
                log!("=> Success!");
                let boxed: Box<dyn GLESContext> = Box::new(ctx);
                return boxed;
            }
            Err(err) => {
                log!("=> Failed: {}.", err);
            }
        }

        log!(
            "Trying: {} (used for OpenGL ES 2.0)",
            GLES2OnGL3Context::description()
        );
        match GLES2OnGL3Context::new(window) {
            Ok(ctx) => {
                log!("=> Success!");
                let boxed: Box<dyn GLESContext> = Box::new(ctx);
                return boxed;
            }
            Err(err) => {
                log!("=> Failed: {}.", err);
            }
        }

        log!(
            "Trying: {} (legacy GL 2.1 fallback for OpenGL ES 2.0)",
            GLES1OnGL2Context::description()
        );
        match GLES1OnGL2Context::new(window) {
            Ok(ctx) => {
                log!("=> Success!");
                let boxed: Box<dyn GLESContext> = Box::new(ctx);
                boxed
            }
            Err(err) => panic!("Couldn't create OpenGL ES 2.0 context: {}", err),
        }
    })
}

/// Try to create an OpenGL ES 3.0 context, panicking on failure.
///
/// This is the entry point used by [crate::frameworks::opengles::eagl] when
/// `EAGLContext initWithAPI:` is called with `kEAGLRenderingAPIOpenGLES3` (=
/// 3). It tries the native ES 3.0 backend first — the only thing that works
/// on Android and on desktop drivers configured for an ES context — and
/// falls back to the desktop GL 3.3 Core translation backend on hosts
/// without a native ES 3.0 driver (most x86 Linux/macOS desktops).
pub fn create_gles3_ctx(env: &mut Environment) -> Box<dyn GLESContext> {
    env.on_parent_stack_in_coroutine(|window, _options| {
        assert!(window.on_main_stack());
        log!("Creating an OpenGL ES 3.0 context:");

        log!("Trying: {}", GLES3NativeContext::description());
        match GLES3NativeContext::new(window) {
            Ok(ctx) => {
                log!("=> Success!");
                let boxed: Box<dyn GLESContext> = Box::new(ctx);
                return boxed;
            }
            Err(err) => {
                log!("=> Failed: {}.", err);
            }
        }

        log!(
            "Trying: {} (used for OpenGL ES 3.0)",
            GLES3OnGL3Context::description()
        );
        match GLES3OnGL3Context::new(window) {
            Ok(ctx) => {
                log!("=> Success!");
                let boxed: Box<dyn GLESContext> = Box::new(ctx);
                boxed
            }
            Err(err) => panic!("Couldn't create OpenGL ES 3.0 context: {}", err),
        }
    })
}

/// Same as [create_gles1_ctx], but without calling
/// [Environment::on_parent_stack_in_coroutine]. Only should be called by
/// functions not inside a coroutine that can't use [Environment].
pub fn create_gles1_ctx_no_parent_stack(
    window: &mut crate::window::Window,
    options: &crate::options::Options,
) -> Box<dyn GLESContext> {
    assert!(window.on_main_stack());
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
