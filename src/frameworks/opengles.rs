/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! OpenGL ES and EAGL.
//!
//! This module is specific to OpenGL ES's role as a part of the iPhone OS API
//! surface. See [crate::gles] for other uses and a discussion of the broader
//! topic.

pub mod eagl;
mod gles_guest;

pub use gles_guest::FUNCTIONS;
use touchHLE_gl_bindings::gles11::types::GLenum;

use crate::mem::ConstPtr;

#[derive(Default)]
pub struct State {
    /// Current EAGLContext for each thread
    current_ctxs: std::collections::HashMap<crate::ThreadId, Option<crate::objc::id>>,
    // BEFOREMERGE Note: There's no need to cache this here anymore - GLES
    // instaces will automatically check if a context is active and not bother
    // switching if there's no need.
    strings_cache: std::collections::HashMap<GLenum, ConstPtr<u8>>,
}
impl State {
    fn current_ctx_for_thread(&mut self, thread: crate::ThreadId) -> &mut Option<crate::objc::id> {
        self.current_ctxs.entry(thread).or_insert(None);
        self.current_ctxs.get_mut(&thread).unwrap()
    }
}

#[must_use]
fn sync_context<'a>(
    state: &mut State,
    objc: &'a mut crate::objc::ObjC,
    window: &mut crate::window::Window,
    current_thread: crate::ThreadId,
) -> Box<dyn crate::gles::GLESContext + 'a> {
    let current_ctx = state.current_ctx_for_thread(current_thread);
    let host_obj = objc.borrow_mut::<eagl::EAGLContextHostObject>(current_ctx.unwrap());
    let gles_ctx = host_obj.gles_ctx.as_deref_mut().unwrap();
    gles_ctx.make_current(window)
}
