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

use crate::mem::ConstPtr;
pub use gles_guest::FUNCTIONS;
use touchHLE_gl_bindings::gles11::types::GLenum;

#[derive(Default)]
pub struct State {
    /// Current EAGLContext for each thread
    current_ctxs: std::collections::HashMap<crate::ThreadId, Option<crate::objc::id>>,
    /// Which thread's EAGLContext is currently active
    current_ctx_thread: Option<crate::ThreadId>,
    strings_cache: std::collections::HashMap<GLenum, ConstPtr<u8>>,
}
impl State {
    fn current_ctx_for_thread(&mut self, thread: crate::ThreadId) -> &mut Option<crate::objc::id> {
        self.current_ctxs.entry(thread).or_insert(None);
        self.current_ctxs.get_mut(&thread).unwrap()
    }
}

fn sync_context<'a, F>(
    state: &mut State,
    objc: &'a mut crate::objc::ObjC,
    window: &mut crate::window::Window,
    current_thread: crate::ThreadId,
    mut action: F,
) where
    F: FnMut(&mut dyn crate::gles::GLES, &'a mut crate::objc::ObjC, &mut crate::window::Window),
{
    let current_ctx = state.current_ctx_for_thread(current_thread);
    let host_obj = objc.borrow_mut::<eagl::EAGLContextHostObject>(current_ctx.unwrap());
    let gles_ctx_rc = host_obj.gles_ctx.clone().unwrap();
    let mut gles_ctx_refcell = gles_ctx_rc.borrow_mut();
    let gles_ctx: &mut dyn crate::gles::GLES = &mut **gles_ctx_refcell;

    if window.is_app_gl_ctx_no_longer_current() || state.current_ctx_thread != Some(current_thread)
    {
        log_dbg!(
            "Restoring guest app OpenGL context for thread {}.",
            current_thread
        );
        gles_ctx.make_current(window);
    }

    action(gles_ctx, objc, window);
}
