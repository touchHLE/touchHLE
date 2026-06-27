/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Full OpenGL ES 2.0 backend implemented on top of desktop OpenGL 3.3 Core.
//!
//! This is the fallback path used by [`crate::gles::create_gles2_ctx`] when
//! the host has no native OpenGL ES 2.0 driver. It supersedes the legacy
//! [`crate::gles::gles1_on_gl2`] ES 2.0 fallback: that path provided ES 2.0
//! by piggy-backing on OpenGL 2.1 compatibility profile and reused the ES 1.1
//! fixed-function emulator's machinery, which left several ES 2.0-only
//! entry points behind silent stubs. This module instead reuses the
//! [`crate::gles::gles3_on_gl3::GLES3OnGL3`] implementation — which exposes
//! every ES 2.0 entry point as a real call into desktop GL 3.3 Core — and
//! flips a single flag so the backend reports itself as ES 2.0 (i.e.
//! [`GLES::is_es3`] returns `false`) for guest apps that ask EAGL for
//! `kEAGLRenderingAPIOpenGLES2`.
//!
//! Why GL 3.3 Core (and not e.g. GL 2.1 compat) for an ES 2.0 host?
//!
//! - GL 3.3 Core is the *lowest* desktop GL version that exposes every ES 2.0
//!   feature with stable, non-deprecated entry points. Earlier versions
//!   (2.1 compat) require shimming around deprecated paths and don't
//!   guarantee `GL_ARB_ES2_compatibility` (which provides
//!   `glReleaseShaderCompiler` / `glGetShaderPrecisionFormat` / `glClearDepthf`
//!   / `glDepthRangef`).
//! - All modern desktop GL drivers (≥ 2012) advertise GL 3.3 Core, including
//!   Mesa's `llvmpipe` and `softpipe` software renderers we use in CI.
//! - `gles3_on_gl3` already implements every ES 2.0 entry point as a direct
//!   call into the 3.3 Core namespace; reusing it means a single source of
//!   truth for both `EAGL` ES 2.0 and ES 3.0 paths and zero behavioural
//!   drift between the two.
//!
//! GLSL ES 1.00 shader source is translated to desktop GLSL 1.20 by
//! [`crate::gles::gles2_glsl::translate_glsl_es_to_120`] before being passed
//! to the driver. The GL 3.3 Core context still accepts `#version 120`
//! shaders because compatibility with older shader profiles is part of the
//! core spec.

use super::gles3_on_gl3::GLES3OnGL3Context;
use super::gles_generic::GLES;
use super::GLESContext;
use crate::window::Window;

/// Factory that mirrors [`GLES3OnGL3Context`] but produces backends that
/// advertise themselves as OpenGL ES 2.0 (rather than ES 3.0). See the module
/// docs for rationale.
pub struct GLES2OnGL3Context {
    inner: GLES3OnGL3Context,
}

impl GLESContext for GLES2OnGL3Context {
    fn description() -> &'static str {
        "OpenGL ES 2.0 on OpenGL 3.3 Core"
    }

    fn new(window: &mut Window) -> Result<Self, String> {
        Ok(Self {
            inner: GLES3OnGL3Context::new_with_mode(window, /* advertise_es3= */ false)?,
        })
    }

    fn make_current<'gl_ctx, 'win: 'gl_ctx>(
        &'gl_ctx mut self,
        window: &'win mut Window,
    ) -> Box<dyn GLES + 'gl_ctx> {
        // Delegate verbatim to the GL 3.3 Core backend. The `advertise_es3`
        // flag stored inside `inner` is what gives the resulting trait object
        // its OpenGL ES 2.0 identity (see [`GLES::is_es3`]).
        self.inner.make_current(window)
    }

    unsafe fn make_current_unchecked_for_window<'gl_ctx>(
        &'gl_ctx mut self,
        make_current_fn: &mut dyn FnMut(&crate::window::GLContext),
        loader_fn: &mut dyn FnMut(&'static str) -> *const std::ffi::c_void,
    ) -> Box<dyn GLES + 'gl_ctx> {
        self.inner
            .make_current_unchecked_for_window(make_current_fn, loader_fn)
    }
}
