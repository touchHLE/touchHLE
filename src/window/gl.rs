/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! OpenGL context creation etc.

use sdl2::video::GLProfile;

pub use touchHLE_gl_bindings::{gl21compat, gles11};

pub enum GLVersion {
    /// OpenGL ES 1.1
    GLES11,
    /// OpenGL 2.1 compatibility profile
    GL21Compat,
}

pub struct GLContext {
    gl_ctx: sdl2::video::GLContext,
    version: GLVersion,
}

pub fn create_gl_context(
    video_ctx: &sdl2::VideoSubsystem,
    window: &sdl2::video::Window,
    version: GLVersion,
) -> Result<GLContext, String> {
    let attr = video_ctx.gl_attr();
    match version {
        GLVersion::GLES11 => {
            attr.set_context_version(1, 1);
            attr.set_context_profile(GLProfile::GLES);
        }
        GLVersion::GL21Compat => {
            attr.set_context_version(2, 1);
            attr.set_context_profile(GLProfile::Compatibility);
        }
    }

    let gl_ctx = window.gl_create_context()?;

    Ok(GLContext { gl_ctx, version })
}

pub fn make_gl_context_current(
    video_ctx: &sdl2::VideoSubsystem,
    window: &sdl2::video::Window,
    gl_ctx: &GLContext,
) {
    window.gl_make_current(&gl_ctx.gl_ctx).unwrap();
    match gl_ctx.version {
        GLVersion::GLES11 => gles11::load_with(|s| video_ctx.gl_get_proc_address(s) as *const _),
        GLVersion::GL21Compat => {
            gl21compat::load_with(|s| video_ctx.gl_get_proc_address(s) as *const _)
        }
    }
}
