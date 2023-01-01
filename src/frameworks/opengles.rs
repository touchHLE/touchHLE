//! OpenGL ES and EAGL.
//!
//! The OpenGL ES implementation is arranged in layers:
//!
//! - `gles_generic` provides an abstraction over OpenGL ES implementations.
//! - `gles_guest` wraps `guest_generic` to expose OpenGL ES to the guest app.
//! - Various child modules provide implementations:
//!   - `gles1_on_gl2` provides an implementation of OpenGL ES 1.1 using OpenGL
//!     2.1 compatibility profile.
//!   - There are are no others currently, but an obvious future target is
//!     exposing real OpenGL ES 1.1 provided by Android.
//!
//! Useful resources for OpenGL ES 1.1:
//! - [Reference pages](https://registry.khronos.org/OpenGL-Refpages/es1.1/xhtml/)
//! - [Specification](https://registry.khronos.org/OpenGL/specs/es/1.1/es_full_spec_1.1.pdf)
//! - Extensions:
//!   - [OES_framebuffer_object](https://registry.khronos.org/OpenGL/extensions/OES/OES_framebuffer_object.txt)
//!
//! Useful resources for OpenGL 2.1:
//! - [Reference pages](https://registry.khronos.org/OpenGL-Refpages/gl2.1/)
//! - [Specification](https://registry.khronos.org/OpenGL/specs/gl/glspec21.pdf)
//! - Extensions:
//!   - [EXT_framebuffer_object](https://registry.khronos.org/OpenGL/extensions/EXT/EXT_framebuffer_object.txt)

pub mod eagl;
mod gles1_on_gl2;
mod gles_generic;
mod gles_guest;

use gles1_on_gl2::GLES1OnGL2;
use gles_generic::GLES;
pub use gles_guest::FUNCTIONS;

#[derive(Default)]
pub struct State {
    /// Current EAGLContext and GLES implementation
    current_ctx: Option<(crate::objc::id, Box<dyn GLES>)>,
}
