/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! EAGL.

use super::{GLES1OnGL2, GLES};
use crate::dyld::{ConstantExports, HostConstant};
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::NSUInteger;
use crate::objc::{id, msg, nil, objc_classes, release, retain, ClassExports, HostObject};
use crate::window::gles11;
use crate::window::Matrix;
use crate::Environment; // for constants

// These are used by the EAGLDrawable protocol implemented by CAEAGLayer.
// Since these have the ABI of constant symbols rather than literal constants,
// the values shouldn't matter, and haven't been checked against real iPhone OS.
pub const kEAGLDrawablePropertyColorFormat: &str = "ColorFormat";
pub const kEAGLDrawablePropertyRetainedBacking: &str = "RetainedBacking";
pub const kEAGLColorFormatRGBA8: &str = "RGBA8";
pub const kEAGLColorFormatRGB565: &str = "RGB565";

pub const CONSTANTS: ConstantExports = &[
    (
        "_kEAGLDrawablePropertyColorFormat",
        HostConstant::NSString(kEAGLDrawablePropertyColorFormat),
    ),
    (
        "_kEAGLDrawablePropertyRetainedBacking",
        HostConstant::NSString(kEAGLDrawablePropertyRetainedBacking),
    ),
    (
        "_kEAGLColorFormatRGBA8",
        HostConstant::NSString(kEAGLColorFormatRGBA8),
    ),
    (
        "_kEAGLColorFormatRGB565",
        HostConstant::NSString(kEAGLColorFormatRGB565),
    ),
];

type EAGLRenderingAPI = u32;
const kEAGLRenderingAPIOpenGLES1: EAGLRenderingAPI = 1;
#[allow(dead_code)]
const kEAGLRenderingAPIOpenGLES2: EAGLRenderingAPI = 2;
#[allow(dead_code)]
const kEAGLRenderingAPIOpenGLES3: EAGLRenderingAPI = 3;

pub(super) struct EAGLContextHostObject {
    pub(super) gles_ctx: Option<Box<dyn GLES>>,
}
impl HostObject for EAGLContextHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation EAGLContext: NSObject

+ (id)alloc {
    let host_object = Box::new(EAGLContextHostObject { gles_ctx: None });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

+ (id)currentContext {
    env.framework_state.opengles.current_ctx_for_thread(env.current_thread).unwrap_or(nil)
}
+ (bool)setCurrentContext:(id)context { // EAGLContext*
    retain(env, context);

    // Clear flag value, we're changing context anyway.
    let _ = env.window.is_app_gl_ctx_no_longer_current();

    let current_ctx = env.framework_state.opengles.current_ctx_for_thread(env.current_thread);

    if let Some(old_ctx) = std::mem::take(current_ctx) {
        release(env, old_ctx);
        env.framework_state.opengles.current_ctx_thread = None;
    }

    // reborrow
    let current_ctx = env.framework_state.opengles.current_ctx_for_thread(env.current_thread);

    if context != nil {
        let host_obj = env.objc.borrow_mut::<EAGLContextHostObject>(context);
        host_obj.gles_ctx.as_mut().unwrap().make_current(&mut env.window);
        *current_ctx = Some(context);
        env.framework_state.opengles.current_ctx_thread = Some(env.current_thread);
    }

    true
}

- (id)initWithAPI:(EAGLRenderingAPI)api {
    assert!(api == kEAGLRenderingAPIOpenGLES1);

    let gles1_ctx = Box::new(GLES1OnGL2::new(&mut env.window));

    *env.objc.borrow_mut(this) = EAGLContextHostObject {
        gles_ctx: Some(gles1_ctx),
    };

    this
}

- (bool)renderbufferStorage:(NSUInteger)target
               fromDrawable:(id)drawable { // EAGLDrawable (always CAEAGLayer*)
    assert!(target == gles11::RENDERBUFFER_OES);

    let props: id = msg![env; drawable drawableProperties];

    let format_key = get_static_str(env, kEAGLDrawablePropertyColorFormat);
    let format_rgba8 = get_static_str(env, kEAGLColorFormatRGBA8);
    let format_rgb565 = get_static_str(env, kEAGLColorFormatRGB565);

    let format: id = msg![env; props objectForKey:format_key];
    // Theoretically this should map formats like:
    // - kColorFormatRGBA8 => RGBA8_OES
    // - kColorFormatRGB565 => RGB565_OES
    // However, the specification of EXT_framebuffer_object allows the
    // implementation to arbitrarily restrict which formats can be rendered to,
    // and it seems like RGB565 isn't supported, at least on a machine with
    // Intel HD Graphics 615 running macOS Monterey. I don't think RGBA8 is
    // guaranteed either, but it at least seems to work.
    if !msg![env; format isEqualTo:format_rgba8] && !msg![env; format isEqualTo:format_rgb565] {
        log!("[renderbufferStorage:{:?} fromDrawable:{:?}] Warning: unhandled format {:?}, using RGBA8", target, drawable, format);
    }
    let internalformat = gles11::RGBA8_OES;

    // FIXME: get width and height from the layer!
    let (width, height) = env.window.size_unrotated_scalehacked();

    // Unclear from documentation if this method requires an appropriate context
    // to already be active, but that seems to be the case in practice?
    let gles = super::sync_context(&mut env.framework_state.opengles, &mut env.objc, &mut env.window, env.current_thread);
    unsafe {
        gles.RenderbufferStorageOES(target, internalformat, width.try_into().unwrap(), height.try_into().unwrap())
    }

    true
}

- (bool)presentRenderbuffer:(NSUInteger)target {
    assert!(target == gles11::RENDERBUFFER_OES);

    // Unclear from documentation if this method requires an appropriate context
    // to already be active, but that seems to be the case in practice?
    super::sync_context(&mut env.framework_state.opengles, &mut env.objc, &mut env.window, env.current_thread);
    unsafe {
        present_renderbuffer(env);
    }

    true
}

@end

};

/// Copies the renderbuffer provided by the app to the window's framebuffer,
/// rotated if necessary, and presents that framebuffer.
unsafe fn present_renderbuffer(env: &mut Environment) {
    // Renderbuffers can't be directly read from, but GL_EXT_framebuffer_blit
    // provides a way to blit between framebuffers, which may have renderbuffers
    // attached to them. Since OpenGL ES 1.1 doesn't have that extension, we
    // have to bypass the API abstraction layer here.
    //
    // GL_EXT_framebuffer_blit can't do rotation, so we will have to blit to a
    // framebuffer with a texture attached, then draw a textured quad.
    use crate::window::gl21compat as gl;
    use crate::window::gl21compat::types::*;

    let mut renderbuffer: GLuint = 0;
    let mut width: GLint = 0;
    let mut height: GLint = 0;
    gl::GetIntegerv(
        gl::RENDERBUFFER_BINDING_EXT,
        &mut renderbuffer as *mut _ as *mut _,
    );
    gl::GetRenderbufferParameterivEXT(gl::RENDERBUFFER_EXT, gl::RENDERBUFFER_WIDTH_EXT, &mut width);
    gl::GetRenderbufferParameterivEXT(
        gl::RENDERBUFFER_EXT,
        gl::RENDERBUFFER_HEIGHT_EXT,
        &mut height,
    );

    // To avoid confusing the guest app, we need to be able to undo any
    // state changes we make.
    let mut old_draw_framebuffer: GLuint = 0;
    let mut old_read_framebuffer: GLuint = 0;
    let mut old_texture_2d: GLuint = 0;
    gl::GetIntegerv(
        gl::DRAW_FRAMEBUFFER_BINDING_EXT,
        &mut old_draw_framebuffer as *mut _ as *mut _,
    );
    gl::GetIntegerv(
        gl::READ_FRAMEBUFFER_BINDING_EXT,
        &mut old_read_framebuffer as *mut _ as *mut _,
    );
    gl::GetIntegerv(
        gl::TEXTURE_BINDING_2D,
        &mut old_texture_2d as *mut _ as *mut _,
    );

    // Create a texture that we can copy the renderbuffer to
    let mut texture: GLuint = 0;
    gl::GenTextures(1, &mut texture);
    gl::BindTexture(gl::TEXTURE_2D, texture);
    gl::TexImage2D(
        gl::TEXTURE_2D,
        0,
        gl::RGBA as _,
        width,
        height,
        0,
        gl::RGBA,
        gl::UNSIGNED_BYTE,
        std::ptr::null(),
    );
    // texture will not have any mip levels so we must ensure filter does
    // not use them, else rendering will fail
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);

    // Create a framebuffer we can use to write to the texture
    let mut dst_framebuffer = 0;
    gl::GenFramebuffersEXT(1, &mut dst_framebuffer);
    gl::BindFramebufferEXT(gl::DRAW_FRAMEBUFFER_EXT, dst_framebuffer);
    gl::FramebufferTexture2DEXT(
        gl::DRAW_FRAMEBUFFER_EXT,
        gl::COLOR_ATTACHMENT0_EXT,
        gl::TEXTURE_2D,
        texture,
        0,
    );

    // Create a framebuffer we can use to read from the renderbuffer
    let mut src_framebuffer = 0;
    gl::GenFramebuffersEXT(1, &mut src_framebuffer);
    gl::BindFramebufferEXT(gl::READ_FRAMEBUFFER_EXT, src_framebuffer);
    gl::FramebufferRenderbufferEXT(
        gl::READ_FRAMEBUFFER_EXT,
        gl::COLOR_ATTACHMENT0_EXT,
        gl::RENDERBUFFER_EXT,
        renderbuffer,
    );

    // Blit!
    gl::BlitFramebufferEXT(
        0,
        0,
        width,
        height,
        0,
        0,
        width,
        height,
        gl::COLOR_BUFFER_BIT,
        gl::LINEAR,
    );

    // Clean up the framebuffer objects since we no longer need them.
    // This also sets the framebuffer bindings back to zero, so rendering
    // will go to the default framebuffer (the window).
    gl::DeleteFramebuffersEXT(2, [dst_framebuffer, src_framebuffer].as_ptr());

    // There are a huge number of pieces of state that can affect rendering.
    // Backing up and then clearing all of it is the easiest way to ensure
    // that drawing the quad works.
    gl::PushClientAttrib(gl::CLIENT_ALL_ATTRIB_BITS);
    for array in super::gles1_on_gl2::ARRAYS {
        gl::DisableClientState(array.name);
    }
    gl::PushAttrib(gl::ALL_ATTRIB_BITS);
    for &cap in super::gles1_on_gl2::CAPABILITIES {
        gl::Disable(cap);
    }
    let mut old_matrix_mode: GLenum = 0;
    gl::GetIntegerv(gl::MATRIX_MODE, &mut old_matrix_mode as *mut _ as *mut _);
    for mode in [gl::MODELVIEW, gl::PROJECTION, gl::TEXTURE] {
        gl::MatrixMode(mode);
        gl::PushMatrix();
        gl::LoadIdentity();
    }
    let mut old_array_buffer: GLuint = 0;
    gl::GetIntegerv(
        gl::ARRAY_BUFFER_BINDING,
        &mut old_array_buffer as *mut _ as *mut _,
    );

    // Draw the quad
    let viewport_size = env.window.size_in_current_orientation();
    gl::Viewport(0, 0, viewport_size.0 as _, viewport_size.1 as _);
    gl::ClearColor(0.0, 0.0, 0.0, 1.0);
    gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT | gl::STENCIL_BUFFER_BIT);
    gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    let vertices: [f32; 12] = [
        -1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0,
    ];
    gl::EnableClientState(gl::VERTEX_ARRAY);
    gl::VertexPointer(2, gl::FLOAT, 0, vertices.as_ptr() as *const GLvoid);
    let tex_coords: [f32; 12] = [0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    gl::EnableClientState(gl::TEXTURE_COORD_ARRAY);
    gl::TexCoordPointer(2, gl::FLOAT, 0, tex_coords.as_ptr() as *const GLvoid);
    let matrix = Matrix::<4>::from(&env.window.output_rotation_matrix());
    gl::MatrixMode(gl::TEXTURE);
    gl::LoadMatrixf(matrix.columns().as_ptr() as *const _);
    gl::Enable(gl::TEXTURE_2D);
    gl::DrawArrays(gl::TRIANGLES, 0, 6);

    // Display virtual cursor
    if let Some((x, y, pressed)) = env.window.virtual_cursor_visible_at() {
        gl::DisableClientState(gl::TEXTURE_COORD_ARRAY);
        gl::Disable(gl::TEXTURE_2D);

        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::ONE, gl::ONE_MINUS_SRC_ALPHA);
        gl::Color4f(0.0, 0.0, 0.0, if pressed { 2.0 / 3.0 } else { 1.0 / 3.0 });

        let radius = 10.0;

        let mut vertices = vertices;
        for i in (0..vertices.len()).step_by(2) {
            vertices[i] = (vertices[i] * radius + x) / (viewport_size.0 as f32 / 2.0) - 1.0;
            vertices[i + 1] = 1.0 - (vertices[i + 1] * radius + y) / (viewport_size.1 as f32 / 2.0);
        }
        gl::VertexPointer(2, gl::FLOAT, 0, vertices.as_ptr() as *const GLvoid);
        gl::DrawArrays(gl::TRIANGLES, 0, 6);
    }

    // Clean up the texture
    gl::DeleteTextures(1, &texture);

    // Restore all the state saved before rendering
    gl::BindBuffer(gl::ARRAY_BUFFER, old_array_buffer);
    for mode in [gl::MODELVIEW, gl::PROJECTION, gl::TEXTURE] {
        gl::MatrixMode(mode);
        gl::PopMatrix();
    }
    gl::MatrixMode(old_matrix_mode);
    gl::PopAttrib();
    gl::PopClientAttrib();

    // SDL2's documentation warns 0 should be bound to the draw framebuffer
    // when swapping the window, so this is the perfect moment.
    env.window.swap_window();

    // Restore the other bindings
    gl::BindTexture(gl::TEXTURE_2D, old_texture_2d);
    gl::BindFramebufferEXT(gl::DRAW_FRAMEBUFFER_EXT, old_draw_framebuffer);
    gl::BindFramebufferEXT(gl::READ_FRAMEBUFFER_EXT, old_read_framebuffer);

    //{ let err = gl::GetError(); if err != 0 { panic!("{:#x}", err); } }
}
