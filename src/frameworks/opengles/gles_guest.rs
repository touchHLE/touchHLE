//! Wrapper functions exposing OpenGL ES to the guest.

use super::GLES;
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, GuestUSize, Mem, MutPtr};
use crate::window::gles11::types::*;
use crate::Environment;

fn with_ctx_and_mem<T, U>(env: &mut Environment, f: T) -> U
where
    T: FnOnce(&mut dyn GLES, &mut Mem) -> U,
{
    let (_eagl, ref mut gles) = env.framework_state.opengles.current_ctx.as_mut().unwrap();
    //panic_on_gl_errors(&mut **gles);
    let res = f(&mut **gles, &mut env.mem);
    //panic_on_gl_errors(&mut **gles);
    #[allow(clippy::let_and_return)]
    res
}

/// Useful for debugging
#[allow(dead_code)]
fn panic_on_gl_errors(gles: &mut dyn GLES) {
    let mut did_error = false;
    loop {
        let err = unsafe { gles.GetError() };
        if err == 0 {
            break;
        }
        did_error = true;
        println!("glGetError() => {:#x}", err);
    }
    if did_error {
        panic!();
    }
}

// Matrix stack operations
fn glMatrixMode(env: &mut Environment, mode: GLenum) {
    with_ctx_and_mem(env, |gles, _mem| {
        unsafe { gles.MatrixMode(mode) };
    });
}
fn glLoadIdentity(env: &mut Environment) {
    with_ctx_and_mem(env, |gles, _mem| {
        unsafe { gles.LoadIdentity() };
    });
}
fn glLoadMatrixf(env: &mut Environment, m: ConstPtr<GLfloat>) {
    with_ctx_and_mem(env, |gles, mem| {
        let m = mem.ptr_at(m, 16);
        unsafe { gles.LoadMatrixf(m) };
    });
}
fn glLoadMatrixx(env: &mut Environment, m: ConstPtr<GLfixed>) {
    with_ctx_and_mem(env, |gles, mem| {
        let m = mem.ptr_at(m, 16);
        unsafe { gles.LoadMatrixx(m) };
    });
}
fn glMultMatrixf(env: &mut Environment, m: ConstPtr<GLfloat>) {
    with_ctx_and_mem(env, |gles, mem| {
        let m = mem.ptr_at(m, 16);
        unsafe { gles.MultMatrixf(m) };
    });
}
fn glMultMatrixx(env: &mut Environment, m: ConstPtr<GLfixed>) {
    with_ctx_and_mem(env, |gles, mem| {
        let m = mem.ptr_at(m, 16);
        unsafe { gles.MultMatrixx(m) };
    });
}
fn glPushMatrix(env: &mut Environment) {
    with_ctx_and_mem(env, |gles, _mem| {
        unsafe { gles.PushMatrix() };
    });
}
fn glPopMatrix(env: &mut Environment) {
    with_ctx_and_mem(env, |gles, _mem| {
        unsafe { gles.PopMatrix() };
    });
}
fn glOrthof(
    env: &mut Environment,
    left: GLfloat,
    right: GLfloat,
    bottom: GLfloat,
    top: GLfloat,
    near: GLfloat,
    far: GLfloat,
) {
    with_ctx_and_mem(env, |gles, _mem| {
        unsafe { gles.Orthof(left, right, bottom, top, near, far) };
    });
}
fn glOrthox(
    env: &mut Environment,
    left: GLfixed,
    right: GLfixed,
    bottom: GLfixed,
    top: GLfixed,
    near: GLfixed,
    far: GLfixed,
) {
    with_ctx_and_mem(env, |gles, _mem| {
        unsafe { gles.Orthox(left, right, bottom, top, near, far) };
    });
}
fn glFrustumf(
    env: &mut Environment,
    left: GLfloat,
    right: GLfloat,
    bottom: GLfloat,
    top: GLfloat,
    near: GLfloat,
    far: GLfloat,
) {
    with_ctx_and_mem(env, |gles, _mem| {
        unsafe { gles.Frustumf(left, right, bottom, top, near, far) };
    });
}
fn glFrustumx(
    env: &mut Environment,
    left: GLfixed,
    right: GLfixed,
    bottom: GLfixed,
    top: GLfixed,
    near: GLfixed,
    far: GLfixed,
) {
    with_ctx_and_mem(env, |gles, _mem| {
        unsafe { gles.Frustumx(left, right, bottom, top, near, far) };
    });
}
fn glRotatef(env: &mut Environment, angle: GLfloat, x: GLfloat, y: GLfloat, z: GLfloat) {
    with_ctx_and_mem(env, |gles, _mem| {
        unsafe { gles.Rotatef(angle, x, y, z) };
    });
}
fn glRotatex(env: &mut Environment, angle: GLfixed, x: GLfixed, y: GLfixed, z: GLfixed) {
    with_ctx_and_mem(env, |gles, _mem| {
        unsafe { gles.Rotatex(angle, x, y, z) };
    });
}
fn glScalef(env: &mut Environment, x: GLfloat, y: GLfloat, z: GLfloat) {
    with_ctx_and_mem(env, |gles, _mem| {
        unsafe { gles.Scalef(x, y, z) };
    });
}
fn glScalex(env: &mut Environment, x: GLfixed, y: GLfixed, z: GLfixed) {
    with_ctx_and_mem(env, |gles, _mem| {
        unsafe { gles.Scalex(x, y, z) };
    });
}
fn glTranslatef(env: &mut Environment, x: GLfloat, y: GLfloat, z: GLfloat) {
    with_ctx_and_mem(env, |gles, _mem| {
        unsafe { gles.Translatef(x, y, z) };
    });
}
fn glTranslatex(env: &mut Environment, x: GLfixed, y: GLfixed, z: GLfixed) {
    with_ctx_and_mem(env, |gles, _mem| {
        unsafe { gles.Translatex(x, y, z) };
    });
}

// OES_framebuffer_object
fn glGenFramebuffersOES(env: &mut Environment, n: GLsizei, framebuffers: MutPtr<GLuint>) {
    with_ctx_and_mem(env, |gles, mem| {
        let n_usize: GuestUSize = n.try_into().unwrap();
        let framebuffers = mem.ptr_at_mut(framebuffers, n_usize);
        unsafe { gles.GenFramebuffersOES(n, framebuffers) }
    })
}
fn glGenRenderbuffersOES(env: &mut Environment, n: GLsizei, renderbuffers: MutPtr<GLuint>) {
    with_ctx_and_mem(env, |gles, mem| {
        let n_usize: GuestUSize = n.try_into().unwrap();
        let renderbuffers = mem.ptr_at_mut(renderbuffers, n_usize);
        unsafe { gles.GenRenderbuffersOES(n, renderbuffers) }
    })
}
fn glBindFramebufferOES(env: &mut Environment, target: GLenum, framebuffer: GLuint) {
    with_ctx_and_mem(env, |gles, _mem| unsafe {
        gles.BindFramebufferOES(target, framebuffer)
    })
}
fn glBindRenderbufferOES(env: &mut Environment, target: GLenum, renderbuffer: GLuint) {
    with_ctx_and_mem(env, |gles, _mem| unsafe {
        gles.BindRenderbufferOES(target, renderbuffer)
    })
}
fn glRenderbufferStorageOES(
    env: &mut Environment,
    target: GLenum,
    internalformat: GLenum,
    width: GLsizei,
    height: GLsizei,
) {
    with_ctx_and_mem(env, |gles, _mem| unsafe {
        gles.RenderbufferStorageOES(target, internalformat, width, height)
    })
}
fn glFramebufferRenderbufferOES(
    env: &mut Environment,
    target: GLenum,
    attachment: GLenum,
    renderbuffertarget: GLenum,
    renderbuffer: GLuint,
) {
    with_ctx_and_mem(env, |gles, _mem| unsafe {
        gles.FramebufferRenderbufferOES(target, attachment, renderbuffertarget, renderbuffer)
    })
}
fn glGetRenderbufferParameterivOES(
    env: &mut Environment,
    target: GLenum,
    pname: GLenum,
    params: MutPtr<GLint>,
) {
    with_ctx_and_mem(env, |gles, mem| {
        let params = mem.ptr_at_mut(params, 1);
        unsafe { gles.GetRenderbufferParameterivOES(target, pname, params) }
    })
}
fn glCheckFramebufferStatusOES(env: &mut Environment, target: GLenum) -> GLenum {
    with_ctx_and_mem(env, |gles, _mem| unsafe {
        gles.CheckFramebufferStatusOES(target)
    })
}

pub const FUNCTIONS: FunctionExports = &[
    // Matrix stack operations
    export_c_func!(glMatrixMode(_)),
    export_c_func!(glLoadIdentity()),
    export_c_func!(glLoadMatrixf(_)),
    export_c_func!(glLoadMatrixx(_)),
    export_c_func!(glMultMatrixf(_)),
    export_c_func!(glMultMatrixx(_)),
    export_c_func!(glPushMatrix()),
    export_c_func!(glPopMatrix()),
    export_c_func!(glOrthof(_, _, _, _, _, _)),
    export_c_func!(glOrthox(_, _, _, _, _, _)),
    export_c_func!(glFrustumf(_, _, _, _, _, _)),
    export_c_func!(glFrustumx(_, _, _, _, _, _)),
    export_c_func!(glRotatef(_, _, _, _)),
    export_c_func!(glRotatex(_, _, _, _)),
    export_c_func!(glScalef(_, _, _)),
    export_c_func!(glScalex(_, _, _)),
    export_c_func!(glTranslatef(_, _, _)),
    export_c_func!(glTranslatex(_, _, _)),
    // OES_framebuffer_object
    export_c_func!(glGenFramebuffersOES(_, _)),
    export_c_func!(glGenRenderbuffersOES(_, _)),
    export_c_func!(glBindFramebufferOES(_, _)),
    export_c_func!(glBindRenderbufferOES(_, _)),
    export_c_func!(glRenderbufferStorageOES(_, _, _, _)),
    export_c_func!(glFramebufferRenderbufferOES(_, _, _, _)),
    export_c_func!(glGetRenderbufferParameterivOES(_, _, _)),
    export_c_func!(glCheckFramebufferStatusOES(_)),
];
