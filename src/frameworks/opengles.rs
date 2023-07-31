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

#[derive(Default)]
pub struct State {
    /// Current EAGLContext for each thread
    current_ctxs: std::collections::HashMap<crate::ThreadID, Option<crate::objc::id>>,
    /// Which thread's EAGLContext is currently active
    current_ctx_thread: Option<crate::ThreadID>,
}
impl State {
    fn current_ctx_for_thread(&mut self, thread: crate::ThreadID) -> &mut Option<crate::objc::id> {
        self.current_ctxs.entry(thread).or_insert(None);
        self.current_ctxs.get_mut(&thread).unwrap()
    }
}

fn sync_context<'a>(
    state: &mut State,
    objc: &'a mut crate::objc::ObjC,
    window: &mut crate::window::Window,
    current_thread: crate::ThreadID,
) -> &'a mut dyn crate::gles::GLES {
    let current_ctx = state.current_ctx_for_thread(current_thread);
    let host_obj = objc.borrow_mut::<eagl::EAGLContextHostObject>(current_ctx.unwrap());
    let gles_ctx = host_obj.gles_ctx.as_deref_mut().unwrap();

    if window.is_app_gl_ctx_no_longer_current() || state.current_ctx_thread != Some(current_thread)
    {
        log_dbg!(
            "Restoring guest app OpenGL context for thread {}.",
            current_thread
        );
        gles_ctx.make_current(window);
    }

    gles_ctx
}
