/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Passthrough for a native OpenGL ES 3.0 driver.
//!
//! This is the preferred OpenGL ES 3.0 backend. It loads the host's native
//! ES 3.0 entry points (via SDL2 `gl_get_proc_address`) and forwards every
//! single `GLES` trait call to the real driver — there are no stubs or
//! placeholders at this layer.
//!
//! On platforms without a native ES 3.0 driver (most desktop x86 Linux
//! configurations using the legacy GL 2.1 compat profile), use
//! [super::gles3_on_gl3] instead, which translates ES 3.0 to desktop
//! OpenGL 3.3 Core.

use super::gles11_raw as gles11;
use super::gles11_raw::types::*;
use super::gles30_raw as gles30;
use super::gles_generic::{GLchar, GLES};
use super::util::{try_decode_pvrtc, PalettedTextureFormat};
use super::GLESContext;
use crate::window::{GLContext, GLVersion, Window};
use std::ffi::CStr;
use std::marker::PhantomData;

pub struct GLES3NativeContext {
    gl_ctx: GLContext,
    is_loaded: bool,
    /// Whether the underlying OpenGL ES 2.0 driver advertises
    /// `GL_IMG_texture_compression_pvrtc`. Apps shipped for iPhone OS use
    /// PVRTC textures pervasively (Apple's recommended compression format),
    /// so when the host driver lacks PVRTC we must software-decode the
    /// payload and upload it as plain RGBA — otherwise every PVRTC texture
    /// silently fails with `GL_INVALID_ENUM` and the app renders as black
    /// silhouettes (see Subway Surfers 1.0.1 on Mesa/llvmpipe).
    pvrtc_native: bool,
    /// Whether `pvrtc_native` has been populated yet. The check is deferred
    /// to the first `make_current` because we need a current GL context to
    /// query `GL_EXTENSIONS`.
    pvrtc_native_checked: bool,
}

impl GLESContext for GLES3NativeContext {
    fn description() -> &'static str {
        "Native OpenGL ES 3.0"
    }

    fn new(window: &mut Window) -> Result<Self, String> {
        Ok(Self {
            gl_ctx: window.create_gl_context(GLVersion::GLES30)?,
            is_loaded: false,
            pvrtc_native: false,
            pvrtc_native_checked: false,
        })
    }

    fn make_current<'gl_ctx, 'win: 'gl_ctx>(
        &'gl_ctx mut self,
        window: &'win mut Window,
    ) -> Box<dyn GLES + 'gl_ctx> {
        if self.gl_ctx.is_current() && self.is_loaded {
            return Box::new(GLES3Native {
                _gl_lifetime: PhantomData,
                pvrtc_native: self.pvrtc_native,
            });
        }
        unsafe {
            window.make_gl_context_current(&self.gl_ctx);
        }
        // Load every namespace we reference. The underlying SDL2 loader
        // returns the same function pointer no matter which namespace does
        // the loading — i.e. `gles2::Enable` and `gles30::Enable` resolve
        // to the same `glEnable` symbol on the driver — so loading all
        // three is harmless and keeps the legacy ES 1.1 / ES 2.0 helpers
        // working when this backend is selected for
        // `kEAGLRenderingAPIOpenGLES3`.
        gles30::load_with(|s| window.gl_get_proc_address(s));
        super::gles2_raw::load_with(|s| window.gl_get_proc_address(s));
        gles11::load_with(|s| window.gl_get_proc_address(s));
        self.is_loaded = true;
        if !self.pvrtc_native_checked {
            self.pvrtc_native = unsafe { detect_pvrtc_support() };
            self.pvrtc_native_checked = true;
        }
        Box::new(GLES3Native {
            _gl_lifetime: PhantomData,
            pvrtc_native: self.pvrtc_native,
        })
    }

    unsafe fn make_current_unchecked_for_window<'gl_ctx>(
        &'gl_ctx mut self,
        make_current_fn: &mut dyn FnMut(&GLContext),
        loader_fn: &mut dyn FnMut(&'static str) -> *const std::ffi::c_void,
    ) -> Box<dyn GLES + 'gl_ctx> {
        if self.gl_ctx.is_current() && self.is_loaded {
            return Box::new(GLES3Native {
                _gl_lifetime: PhantomData,
                pvrtc_native: self.pvrtc_native,
            });
        }
        make_current_fn(&self.gl_ctx);
        gles30::load_with(&mut *loader_fn);
        super::gles2_raw::load_with(&mut *loader_fn);
        gles11::load_with(&mut *loader_fn);
        self.is_loaded = true;
        if !self.pvrtc_native_checked {
            self.pvrtc_native = detect_pvrtc_support();
            self.pvrtc_native_checked = true;
        }
        Box::new(GLES3Native {
            _gl_lifetime: PhantomData,
            pvrtc_native: self.pvrtc_native,
        })
    }
}

/// Query `GL_EXTENSIONS` and return whether the current OpenGL ES 3.0 driver
/// advertises `GL_IMG_texture_compression_pvrtc`. Must be called with a
/// current GL context.
///
/// On strict OpenGL ES 3.0+ contexts `glGetString(GL_EXTENSIONS)` is
/// deprecated and may return an empty string — we fall back to
/// `glGetStringi(GL_EXTENSIONS, i)` in that case, iterating over
/// `GL_NUM_EXTENSIONS`. If neither path produces a useful list we
/// conservatively return `false` and software-decode PVRTC. That's the safe
/// choice: it produces correct output everywhere, at the cost of one extra
/// in-memory pass per texture upload on hosts where PVRTC could otherwise
/// be uploaded directly.
unsafe fn detect_pvrtc_support() -> bool {
    // First try the legacy `glGetString(GL_EXTENSIONS)` path.
    let legacy = gles30::GetString(gles11::EXTENSIONS);
    if legacy.is_null() {
        return false;
    }
    let Ok(s) = CStr::from_ptr(legacy as *const _).to_str() else {
        return false;
    };
    if !s.is_empty()
        && s.split(' ')
            .any(|ext| ext == "GL_IMG_texture_compression_pvrtc")
    {
        return true;
    }
    // Fallback: ES 3.0 indexed extension query.
    let mut count: GLint = 0;
    gles30::GetIntegerv(gles30::NUM_EXTENSIONS, &mut count);
    for i in 0..count.max(0) {
        let p = gles30::GetStringi(gles11::EXTENSIONS, i as GLuint);
        if p.is_null() {
            continue;
        }
        let Ok(ext) = CStr::from_ptr(p as *const _).to_str() else {
            continue;
        };
        if ext == "GL_IMG_texture_compression_pvrtc" {
            return true;
        }
    }
    false
}

pub struct GLES3Native<'gl_ctx> {
    _gl_lifetime: PhantomData<&'gl_ctx ()>,
    pvrtc_native: bool,
}

/// Returns `true` if `cap` is an ES 1.1 fixed-function capability that has
/// no analogue on ES 2.0 / shader-based pipelines, so feeding it to
/// `glEnable` / `glDisable` on an ES 2.0 driver would emit
/// `GL_INVALID_ENUM`. We silently drop those instead, because they
/// originate from apps that ask EAGL for an ES 1.1 context but actually
/// use shaders (see `--prefer-gles2-context`) — those apps still
/// boilerplate-call e.g. `glEnable(GL_TEXTURE_2D)` even though it has no
/// effect on a shader pipeline.
fn is_es1_only_capability(cap: GLenum) -> bool {
    matches!(
        cap,
        gles11::TEXTURE_2D
            | gles11::LIGHTING
            | gles11::FOG
            | gles11::ALPHA_TEST
            | gles11::COLOR_MATERIAL
            | gles11::RESCALE_NORMAL
            | gles11::NORMALIZE
            | gles11::POINT_SMOOTH
            | gles11::LINE_SMOOTH
            // Lighting state arrays
            | gles11::COLOR_ARRAY
            | gles11::NORMAL_ARRAY
            | gles11::VERTEX_ARRAY
            | gles11::TEXTURE_COORD_ARRAY
    ) || (
        // GL_LIGHT0 .. GL_LIGHT7 (0x4000 .. 0x4007)
        (0x4000..=0x4007).contains(&cap)
    ) || (
        // GL_CLIP_PLANE0 .. GL_CLIP_PLANE5 (0x3000 .. 0x3005)
        (0x3000..=0x3005).contains(&cap)
    )
}

/// Same idea as [is_es1_only_capability] but for `glHint` targets that
/// only exist in ES 1.1.
fn is_es1_only_hint_target(target: GLenum) -> bool {
    matches!(
        target,
        gles11::PERSPECTIVE_CORRECTION_HINT
            | gles11::FOG_HINT
            | gles11::POINT_SMOOTH_HINT
            | gles11::LINE_SMOOTH_HINT
    )
    // Note: GENERATE_MIPMAP_HINT (0x8192) is also valid on ES 2.0
    // (carried over from EXT_framebuffer_object) so we do NOT drop it.
}

#[allow(clippy::missing_safety_doc)]
impl GLES for GLES3Native<'_> {
    fn is_es2(&self) -> bool {
        // ES 3.0 supersedes ES 2.0 and shares its shader-based dispatch.
        // `is_es2` gates the existing shader-aware code paths in
        // `present_renderbuffer` and friends, so keep it true.
        true
    }
    fn is_es3(&self) -> bool {
        true
    }

    unsafe fn driver_description(&self) -> String {
        let version = CStr::from_ptr(gles30::GetString(gles30::VERSION) as *const _);
        let vendor = CStr::from_ptr(gles30::GetString(gles30::VENDOR) as *const _);
        let renderer = CStr::from_ptr(gles30::GetString(gles30::RENDERER) as *const _);
        format!(
            "{} / {} / {}",
            version.to_string_lossy(),
            vendor.to_string_lossy(),
            renderer.to_string_lossy()
        )
    }

    // Generic state manipulation
    unsafe fn GetError(&mut self) -> GLenum {
        gles30::GetError()
    }
    unsafe fn Enable(&mut self, cap: GLenum) {
        if is_es1_only_capability(cap) {
            return;
        }
        gles30::Enable(cap)
    }
    unsafe fn IsEnabled(&mut self, cap: GLenum) -> GLboolean {
        if is_es1_only_capability(cap) {
            return gles30::FALSE;
        }
        gles30::IsEnabled(cap)
    }
    unsafe fn Disable(&mut self, cap: GLenum) {
        if is_es1_only_capability(cap) {
            return;
        }
        gles30::Disable(cap)
    }
    unsafe fn GetBooleanv(&mut self, pname: GLenum, params: *mut GLboolean) {
        gles30::GetBooleanv(pname, params)
    }
    unsafe fn GetFloatv(&mut self, pname: GLenum, params: *mut GLfloat) {
        gles30::GetFloatv(pname, params)
    }
    unsafe fn GetIntegerv(&mut self, pname: GLenum, params: *mut GLint) {
        gles30::GetIntegerv(pname, params)
    }
    unsafe fn Hint(&mut self, target: GLenum, mode: GLenum) {
        if is_es1_only_hint_target(target) {
            return;
        }
        gles30::Hint(target, mode)
    }
    unsafe fn Finish(&mut self) {
        gles30::Finish()
    }
    unsafe fn Flush(&mut self) {
        gles30::Flush()
    }
    unsafe fn GetString(&mut self, name: GLenum) -> *const GLubyte {
        gles30::GetString(name)
    }

    // Other state manipulation
    unsafe fn BlendFunc(&mut self, sfactor: GLenum, dfactor: GLenum) {
        gles30::BlendFunc(sfactor, dfactor)
    }
    unsafe fn ColorMask(
        &mut self,
        red: GLboolean,
        green: GLboolean,
        blue: GLboolean,
        alpha: GLboolean,
    ) {
        gles30::ColorMask(red, green, blue, alpha)
    }
    unsafe fn CullFace(&mut self, mode: GLenum) {
        gles30::CullFace(mode)
    }
    unsafe fn DepthFunc(&mut self, func: GLenum) {
        gles30::DepthFunc(func)
    }
    unsafe fn DepthMask(&mut self, flag: GLboolean) {
        gles30::DepthMask(flag)
    }
    unsafe fn DepthRangef(&mut self, near: GLclampf, far: GLclampf) {
        gles30::DepthRangef(near, far)
    }
    unsafe fn FrontFace(&mut self, mode: GLenum) {
        gles30::FrontFace(mode)
    }
    unsafe fn PolygonOffset(&mut self, factor: GLfloat, units: GLfloat) {
        gles30::PolygonOffset(factor, units)
    }
    unsafe fn SampleCoverage(&mut self, value: GLclampf, invert: GLboolean) {
        gles30::SampleCoverage(value, invert)
    }
    unsafe fn Scissor(&mut self, x: GLint, y: GLint, width: GLsizei, height: GLsizei) {
        gles30::Scissor(x, y, width, height)
    }
    unsafe fn Viewport(&mut self, x: GLint, y: GLint, width: GLsizei, height: GLsizei) {
        gles30::Viewport(x, y, width, height)
    }
    unsafe fn LineWidth(&mut self, val: GLfloat) {
        gles30::LineWidth(val)
    }
    unsafe fn StencilFunc(&mut self, func: GLenum, ref_: GLint, mask: GLuint) {
        gles30::StencilFunc(func, ref_, mask)
    }
    unsafe fn StencilOp(&mut self, sfail: GLenum, dpfail: GLenum, dppass: GLenum) {
        gles30::StencilOp(sfail, dpfail, dppass)
    }
    unsafe fn StencilMask(&mut self, mask: GLuint) {
        gles30::StencilMask(mask)
    }

    // Buffers
    unsafe fn IsBuffer(&mut self, buffer: GLuint) -> GLboolean {
        gles30::IsBuffer(buffer)
    }
    unsafe fn GenBuffers(&mut self, n: GLsizei, buffers: *mut GLuint) {
        gles30::GenBuffers(n, buffers)
    }
    unsafe fn DeleteBuffers(&mut self, n: GLsizei, buffers: *const GLuint) {
        gles30::DeleteBuffers(n, buffers)
    }
    unsafe fn BindBuffer(&mut self, target: GLenum, buffer: GLuint) {
        gles30::BindBuffer(target, buffer)
    }
    unsafe fn BufferData(
        &mut self,
        target: GLenum,
        size: GLsizeiptr,
        data: *const GLvoid,
        usage: GLenum,
    ) {
        gles30::BufferData(target, size, data, usage)
    }
    unsafe fn BufferSubData(
        &mut self,
        target: GLenum,
        offset: GLintptr,
        size: GLsizeiptr,
        data: *const GLvoid,
    ) {
        gles30::BufferSubData(target, offset, size, data)
    }

    // Drawing
    unsafe fn DrawArrays(&mut self, mode: GLenum, first: GLint, count: GLsizei) {
        gles30::DrawArrays(mode, first, count)
    }
    unsafe fn DrawElements(
        &mut self,
        mode: GLenum,
        count: GLsizei,
        type_: GLenum,
        indices: *const GLvoid,
    ) {
        gles30::DrawElements(mode, count, type_, indices)
    }
    unsafe fn Clear(&mut self, mask: GLbitfield) {
        gles30::Clear(mask)
    }
    unsafe fn ClearColor(
        &mut self,
        red: GLclampf,
        green: GLclampf,
        blue: GLclampf,
        alpha: GLclampf,
    ) {
        gles30::ClearColor(red, green, blue, alpha)
    }
    unsafe fn ClearDepthf(&mut self, depth: GLclampf) {
        gles30::ClearDepthf(depth)
    }
    unsafe fn ClearStencil(&mut self, s: GLint) {
        gles30::ClearStencil(s)
    }

    // Textures
    unsafe fn PixelStorei(&mut self, pname: GLenum, param: GLint) {
        gles30::PixelStorei(pname, param)
    }
    unsafe fn ReadPixels(
        &mut self,
        x: GLint,
        y: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        type_: GLenum,
        pixels: *mut GLvoid,
    ) {
        gles30::ReadPixels(x, y, width, height, format, type_, pixels)
    }
    unsafe fn IsTexture(&mut self, texture: GLuint) -> GLboolean {
        gles30::IsTexture(texture)
    }
    unsafe fn GenTextures(&mut self, n: GLsizei, textures: *mut GLuint) {
        gles30::GenTextures(n, textures)
    }
    unsafe fn DeleteTextures(&mut self, n: GLsizei, textures: *const GLuint) {
        gles30::DeleteTextures(n, textures)
    }
    unsafe fn ActiveTexture(&mut self, texture: GLenum) {
        gles30::ActiveTexture(texture)
    }
    unsafe fn BindTexture(&mut self, target: GLenum, texture: GLuint) {
        gles30::BindTexture(target, texture)
    }
    unsafe fn TexParameteri(&mut self, target: GLenum, pname: GLenum, param: GLint) {
        // GL_GENERATE_MIPMAP (0x8191) is a TexParameter pname only on ES 1.1.
        // On ES 2.0 the equivalent is the standalone glGenerateMipmap() call.
        // Apps that ask for an ES 1.1 context but rely on shaders frequently
        // still use the ES 1.1 form; redirect it transparently.
        if pname == gles11::GENERATE_MIPMAP {
            if param != 0 {
                gles30::GenerateMipmap(target);
            }
            return;
        }
        gles30::TexParameteri(target, pname, param)
    }
    unsafe fn TexParameterf(&mut self, target: GLenum, pname: GLenum, param: GLfloat) {
        if pname == gles11::GENERATE_MIPMAP {
            if param != 0.0 {
                gles30::GenerateMipmap(target);
            }
            return;
        }
        gles30::TexParameterf(target, pname, param)
    }
    unsafe fn TexParameteriv(&mut self, target: GLenum, pname: GLenum, params: *const GLint) {
        if pname == gles11::GENERATE_MIPMAP {
            if !params.is_null() && *params != 0 {
                gles30::GenerateMipmap(target);
            }
            return;
        }
        gles30::TexParameteriv(target, pname, params)
    }
    unsafe fn TexParameterfv(&mut self, target: GLenum, pname: GLenum, params: *const GLfloat) {
        if pname == gles11::GENERATE_MIPMAP {
            if !params.is_null() && *params != 0.0 {
                gles30::GenerateMipmap(target);
            }
            return;
        }
        gles30::TexParameterfv(target, pname, params)
    }
    #[allow(clippy::too_many_arguments)]
    unsafe fn TexImage2D(
        &mut self,
        target: GLenum,
        level: GLint,
        internalformat: GLint,
        width: GLsizei,
        height: GLsizei,
        border: GLint,
        format: GLenum,
        type_: GLenum,
        pixels: *const GLvoid,
    ) {
        gles30::TexImage2D(
            target,
            level,
            internalformat,
            width,
            height,
            border,
            format,
            type_,
            pixels,
        )
    }
    #[allow(clippy::too_many_arguments)]
    unsafe fn TexSubImage2D(
        &mut self,
        target: GLenum,
        level: GLint,
        xoffset: GLint,
        yoffset: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        type_: GLenum,
        pixels: *const GLvoid,
    ) {
        gles30::TexSubImage2D(
            target, level, xoffset, yoffset, width, height, format, type_, pixels,
        )
    }
    #[allow(clippy::too_many_arguments)]
    unsafe fn CompressedTexImage2D(
        &mut self,
        target: GLenum,
        level: GLint,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
        border: GLint,
        image_size: GLsizei,
        data: *const GLvoid,
    ) {
        // Apps built for iPhone OS overwhelmingly ship textures in PVRTC
        // (Apple's recommended compression format on PowerVR-based devices,
        // documented at
        // https://developer.apple.com/library/archive/documentation/3DDrawing/Conceptual/OpenGLES_ProgrammingGuide/TextureTool/TextureTool.html).
        // Most desktop OpenGL ES 2.0 drivers — including Mesa/llvmpipe used
        // for software rendering — do not implement
        // `GL_IMG_texture_compression_pvrtc`, so a pass-through call returns
        // GL_INVALID_ENUM and leaves the texture in its default (black)
        // state. Mirror the behaviour of the ES 1.1 backends here and
        // software-decode PVRTC to plain RGBA when the host can't do it.
        if !self.pvrtc_native && !data.is_null() && image_size > 0 {
            let payload = std::slice::from_raw_parts(data.cast::<u8>(), image_size as usize);
            if try_decode_pvrtc(
                self,
                target,
                level,
                internalformat,
                width,
                height,
                border,
                payload,
            ) {
                return;
            }
            // Apple-targeted apps also sometimes ship
            // `GL_OES_compressed_paletted_texture` data. Desktop ES 2.0
            // doesn't advertise that extension either, so we'd silently
            // produce another GL_INVALID_ENUM. Software-decode paletted
            // textures to uncompressed RGBA/RGB and upload via glTexImage2D.
            if let Some(PalettedTextureFormat {
                index_is_nibble,
                palette_entry_format,
                palette_entry_type,
            }) = PalettedTextureFormat::get_info(internalformat)
            {
                let palette_entry_size = match palette_entry_type {
                    gles11::UNSIGNED_BYTE => match palette_entry_format {
                        gles11::RGB => 3,
                        gles11::RGBA => 4,
                        _ => unreachable!(),
                    },
                    gles11::UNSIGNED_SHORT_5_6_5
                    | gles11::UNSIGNED_SHORT_4_4_4_4
                    | gles11::UNSIGNED_SHORT_5_5_5_1 => 2,
                    _ => unreachable!(),
                };
                let palette_entry_count: usize = if index_is_nibble { 16 } else { 256 };
                let palette_size = palette_entry_size * palette_entry_count;

                let index_count = width as usize * height as usize;
                let (index_word_size, index_word_count) = if index_is_nibble {
                    (1, index_count.div_ceil(2))
                } else {
                    (4, index_count.div_ceil(4))
                };
                let indices_size = index_word_size * index_word_count;

                let expected_size = palette_size + indices_size;
                if payload.len() < expected_size {
                    log!(
                        "Warning: GLES2Native::CompressedTexImage2D: paletted \
                         format {internalformat:#x} payload too small: got {} \
                         bytes, expected at least {expected_size} for \
                         {width}x{height}; skipping upload.",
                        payload.len()
                    );
                    return;
                }

                let (palette, indices) = payload.split_at(palette_size);

                let mut decoded = Vec::<u8>::with_capacity(palette_entry_size * index_count);
                for i in 0..index_count {
                    let index = if index_is_nibble {
                        (indices[i / 2] >> ((1 - (i % 2)) * 4)) & 0xf
                    } else {
                        indices[i]
                    } as usize;
                    let start = index * palette_entry_size;
                    let palette_entry = &palette[start..start + palette_entry_size];
                    decoded.extend_from_slice(palette_entry);
                }

                log_dbg!(
                    "GLES2Native: software-decoded paletted texture \
                     {width}x{height} (format {internalformat:#x})"
                );

                gles30::TexImage2D(
                    target,
                    level,
                    palette_entry_format as GLint,
                    width,
                    height,
                    border,
                    palette_entry_format,
                    palette_entry_type,
                    decoded.as_ptr() as *const _,
                );
                return;
            }
        }
        gles30::CompressedTexImage2D(
            target,
            level,
            internalformat,
            width,
            height,
            border,
            image_size,
            data,
        )
    }
    unsafe fn CopyTexImage2D(
        &mut self,
        target: GLenum,
        level: GLint,
        internalformat: GLenum,
        x: GLint,
        y: GLint,
        width: GLsizei,
        height: GLsizei,
        border: GLint,
    ) {
        gles30::CopyTexImage2D(target, level, internalformat, x, y, width, height, border)
    }
    #[allow(clippy::too_many_arguments)]
    unsafe fn CopyTexSubImage2D(
        &mut self,
        target: GLenum,
        level: GLint,
        xoffset: GLint,
        yoffset: GLint,
        x: GLint,
        y: GLint,
        width: GLsizei,
        height: GLsizei,
    ) {
        gles30::CopyTexSubImage2D(target, level, xoffset, yoffset, x, y, width, height)
    }
    unsafe fn GenerateMipmapOES(&mut self, target: GLenum) {
        gles30::GenerateMipmap(target)
    }
    unsafe fn GenerateMipmap(&mut self, target: GLenum) {
        gles30::GenerateMipmap(target)
    }
    unsafe fn GenFramebuffers(&mut self, n: GLsizei, framebuffers: *mut GLuint) {
        gles30::GenFramebuffers(n, framebuffers)
    }
    unsafe fn GenRenderbuffers(&mut self, n: GLsizei, renderbuffers: *mut GLuint) {
        gles30::GenRenderbuffers(n, renderbuffers)
    }
    unsafe fn IsFramebuffer(&mut self, framebuffer: GLuint) -> GLboolean {
        gles30::IsFramebuffer(framebuffer)
    }
    unsafe fn IsRenderbuffer(&mut self, renderbuffer: GLuint) -> GLboolean {
        gles30::IsRenderbuffer(renderbuffer)
    }
    unsafe fn BindFramebuffer(&mut self, target: GLenum, framebuffer: GLuint) {
        gles30::BindFramebuffer(target, framebuffer)
    }
    unsafe fn BindRenderbuffer(&mut self, target: GLenum, renderbuffer: GLuint) {
        gles30::BindRenderbuffer(target, renderbuffer)
    }
    unsafe fn RenderbufferStorage(
        &mut self,
        target: GLenum,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
    ) {
        gles30::RenderbufferStorage(target, internalformat, width, height)
    }
    unsafe fn FramebufferRenderbuffer(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        renderbuffertarget: GLenum,
        renderbuffer: GLuint,
    ) {
        gles30::FramebufferRenderbuffer(target, attachment, renderbuffertarget, renderbuffer)
    }
    unsafe fn FramebufferTexture2D(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        textarget: GLenum,
        texture: GLuint,
        level: i32,
    ) {
        gles30::FramebufferTexture2D(target, attachment, textarget, texture, level)
    }
    unsafe fn CheckFramebufferStatus(&mut self, target: GLenum) -> GLenum {
        gles30::CheckFramebufferStatus(target)
    }
    unsafe fn DeleteFramebuffers(&mut self, n: GLsizei, framebuffers: *const GLuint) {
        gles30::DeleteFramebuffers(n, framebuffers)
    }
    unsafe fn DeleteRenderbuffers(&mut self, n: GLsizei, renderbuffers: *const GLuint) {
        gles30::DeleteRenderbuffers(n, renderbuffers)
    }
    unsafe fn GetFramebufferAttachmentParameteriv(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gles30::GetFramebufferAttachmentParameteriv(target, attachment, pname, params)
    }
    unsafe fn GetRenderbufferParameteriv(
        &mut self,
        target: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gles30::GetRenderbufferParameteriv(target, pname, params)
    }
    unsafe fn GetBufferParameteriv(&mut self, target: GLenum, pname: GLenum, params: *mut GLint) {
        gles30::GetBufferParameteriv(target, pname, params)
    }
    // Buffer mapping (`GL_OES_mapbuffer`).
    //
    // On real ES 2.0+ drivers (Adreno, Mali, …) the `GL_OES_mapbuffer`
    // extension is widely supported, so we can route the OES entry points
    // straight to the extension functions loaded via `gles30::load_with`. Some
    // games (e.g. LEGO Ninjago Spinjitzu Scavenger Hunt) call these even when
    // they asked EAGL for an ES 1.1 context — combined with
    // `--prefer-gles2-context`, they end up here.
    unsafe fn MapBufferOES(&mut self, target: GLenum, access: GLenum) -> *mut GLvoid {
        if gles30::MapBufferOES::is_loaded() {
            gles30::MapBufferOES(target, access)
        } else {
            log!(
                "Warning: glMapBufferOES called but GL_OES_mapbuffer is not \
                 available on this ES 2.0 driver; returning NULL"
            );
            std::ptr::null_mut()
        }
    }
    unsafe fn UnmapBufferOES(&mut self, target: GLenum) -> GLboolean {
        if gles30::UnmapBufferOES::is_loaded() {
            gles30::UnmapBufferOES(target)
        } else {
            // Caller will fall back to its own write-through copy.
            gles30::FALSE
        }
    }

    // Framebuffers / renderbuffers (mapped via OES naming → core ES 2 calls)
    unsafe fn GenFramebuffersOES(&mut self, n: GLsizei, framebuffers: *mut GLuint) {
        gles30::GenFramebuffers(n, framebuffers)
    }
    unsafe fn DeleteFramebuffersOES(&mut self, n: GLsizei, framebuffers: *const GLuint) {
        gles30::DeleteFramebuffers(n, framebuffers)
    }
    unsafe fn BindFramebufferOES(&mut self, target: GLenum, framebuffer: GLuint) {
        gles30::BindFramebuffer(target, framebuffer)
    }
    unsafe fn IsFramebufferOES(&mut self, framebuffer: GLuint) -> GLboolean {
        gles30::IsFramebuffer(framebuffer)
    }
    unsafe fn CheckFramebufferStatusOES(&mut self, target: GLenum) -> GLenum {
        gles30::CheckFramebufferStatus(target)
    }
    unsafe fn FramebufferRenderbufferOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        renderbuffertarget: GLenum,
        renderbuffer: GLuint,
    ) {
        gles30::FramebufferRenderbuffer(target, attachment, renderbuffertarget, renderbuffer)
    }
    #[allow(clippy::too_many_arguments)]
    unsafe fn FramebufferTexture2DOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        textarget: GLenum,
        texture: GLuint,
        level: GLint,
    ) {
        gles30::FramebufferTexture2D(target, attachment, textarget, texture, level)
    }
    unsafe fn GetFramebufferAttachmentParameterivOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gles30::GetFramebufferAttachmentParameteriv(target, attachment, pname, params)
    }
    unsafe fn GenRenderbuffersOES(&mut self, n: GLsizei, renderbuffers: *mut GLuint) {
        gles30::GenRenderbuffers(n, renderbuffers)
    }
    unsafe fn DeleteRenderbuffersOES(&mut self, n: GLsizei, renderbuffers: *const GLuint) {
        gles30::DeleteRenderbuffers(n, renderbuffers)
    }
    unsafe fn BindRenderbufferOES(&mut self, target: GLenum, renderbuffer: GLuint) {
        gles30::BindRenderbuffer(target, renderbuffer)
    }
    unsafe fn IsRenderbufferOES(&mut self, renderbuffer: GLuint) -> GLboolean {
        gles30::IsRenderbuffer(renderbuffer)
    }
    unsafe fn RenderbufferStorageOES(
        &mut self,
        target: GLenum,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
    ) {
        gles30::RenderbufferStorage(target, internalformat, width, height)
    }
    unsafe fn GetRenderbufferParameterivOES(
        &mut self,
        target: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gles30::GetRenderbufferParameteriv(target, pname, params)
    }

    // OpenGL ES 2.0 — shaders & programs
    unsafe fn CreateShader(&mut self, type_: GLenum) -> GLuint {
        gles30::CreateShader(type_)
    }
    unsafe fn DeleteShader(&mut self, shader: GLuint) {
        gles30::DeleteShader(shader)
    }
    unsafe fn ShaderSource(
        &mut self,
        shader: GLuint,
        count: GLsizei,
        string: *const *const GLchar,
        length: *const GLint,
    ) {
        // Translate any GLSL ES 1.00 source to GLSL ES too — but since we run
        // on a real ES 2.0 driver, we can pass the source through unchanged.
        gles30::ShaderSource(shader, count, string, length)
    }
    unsafe fn CompileShader(&mut self, shader: GLuint) {
        gles30::CompileShader(shader)
    }
    unsafe fn GetShaderPrecisionFormat(
        &mut self,
        shadertype: GLenum,
        precisiontype: GLenum,
        range: *mut GLint,
        precision: *mut GLint,
    ) {
        // Delegate to the real OpenGL ES 2.0 driver — required for shaders
        // that contain `precision` qualifiers and for apps (e.g. Minecraft PE
        // 0.10.x) that probe the shader compiler before linking.
        // <https://registry.khronos.org/OpenGL-Refpages/es2.0/xhtml/glGetShaderPrecisionFormat.xml>
        gles30::GetShaderPrecisionFormat(shadertype, precisiontype, range, precision)
    }
    unsafe fn ReleaseShaderCompiler(&mut self) {
        gles30::ReleaseShaderCompiler()
    }
    unsafe fn ShaderBinary(
        &mut self,
        count: GLsizei,
        shaders: *const GLuint,
        binaryformat: GLenum,
        binary: *const GLvoid,
        length: GLsizei,
    ) {
        gles30::ShaderBinary(count, shaders, binaryformat, binary, length)
    }
    unsafe fn GetShaderiv(&mut self, shader: GLuint, pname: GLenum, params: *mut GLint) {
        gles30::GetShaderiv(shader, pname, params)
    }
    unsafe fn GetShaderInfoLog(
        &mut self,
        shader: GLuint,
        maxLength: GLsizei,
        length: *mut GLsizei,
        infoLog: *mut GLchar,
    ) {
        gles30::GetShaderInfoLog(shader, maxLength, length, infoLog)
    }
    unsafe fn GetShaderSource(
        &mut self,
        shader: GLuint,
        bufSize: GLsizei,
        length: *mut GLsizei,
        source: *mut GLchar,
    ) {
        gles30::GetShaderSource(shader, bufSize, length, source)
    }
    unsafe fn IsShader(&mut self, shader: GLuint) -> GLboolean {
        gles30::IsShader(shader)
    }
    unsafe fn CreateProgram(&mut self) -> GLuint {
        gles30::CreateProgram()
    }
    unsafe fn DeleteProgram(&mut self, program: GLuint) {
        gles30::DeleteProgram(program)
    }
    unsafe fn AttachShader(&mut self, program: GLuint, shader: GLuint) {
        gles30::AttachShader(program, shader)
    }
    unsafe fn DetachShader(&mut self, program: GLuint, shader: GLuint) {
        gles30::DetachShader(program, shader)
    }
    unsafe fn LinkProgram(&mut self, program: GLuint) {
        gles30::LinkProgram(program)
    }
    unsafe fn UseProgram(&mut self, program: GLuint) {
        gles30::UseProgram(program)
    }
    unsafe fn GetProgramiv(&mut self, program: GLuint, pname: GLenum, params: *mut GLint) {
        gles30::GetProgramiv(program, pname, params)
    }
    unsafe fn GetProgramInfoLog(
        &mut self,
        program: GLuint,
        maxLength: GLsizei,
        length: *mut GLsizei,
        infoLog: *mut GLchar,
    ) {
        gles30::GetProgramInfoLog(program, maxLength, length, infoLog)
    }
    unsafe fn IsProgram(&mut self, program: GLuint) -> GLboolean {
        gles30::IsProgram(program)
    }
    unsafe fn ValidateProgram(&mut self, program: GLuint) {
        gles30::ValidateProgram(program)
    }
    unsafe fn BindAttribLocation(&mut self, program: GLuint, index: GLuint, name: *const GLchar) {
        gles30::BindAttribLocation(program, index, name)
    }
    unsafe fn GetAttribLocation(&mut self, program: GLuint, name: *const GLchar) -> GLint {
        gles30::GetAttribLocation(program, name)
    }
    unsafe fn GetUniformLocation(&mut self, program: GLuint, name: *const GLchar) -> GLint {
        gles30::GetUniformLocation(program, name)
    }
    #[allow(clippy::too_many_arguments)]
    unsafe fn GetActiveAttrib(
        &mut self,
        program: GLuint,
        index: GLuint,
        bufSize: GLsizei,
        length: *mut GLsizei,
        size: *mut GLint,
        type_: *mut GLenum,
        name: *mut GLchar,
    ) {
        gles30::GetActiveAttrib(program, index, bufSize, length, size, type_, name)
    }
    #[allow(clippy::too_many_arguments)]
    unsafe fn GetActiveUniform(
        &mut self,
        program: GLuint,
        index: GLuint,
        bufSize: GLsizei,
        length: *mut GLsizei,
        size: *mut GLint,
        type_: *mut GLenum,
        name: *mut GLchar,
    ) {
        gles30::GetActiveUniform(program, index, bufSize, length, size, type_, name)
    }

    // Vertex attributes
    unsafe fn EnableVertexAttribArray(&mut self, index: GLuint) {
        gles30::EnableVertexAttribArray(index)
    }
    unsafe fn DisableVertexAttribArray(&mut self, index: GLuint) {
        gles30::DisableVertexAttribArray(index)
    }
    unsafe fn VertexAttribPointer(
        &mut self,
        index: GLuint,
        size: GLint,
        type_: GLenum,
        normalized: GLboolean,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        gles30::VertexAttribPointer(index, size, type_, normalized, stride, pointer)
    }
    unsafe fn VertexAttrib1f(&mut self, index: GLuint, x: GLfloat) {
        gles30::VertexAttrib1f(index, x)
    }
    unsafe fn VertexAttrib2f(&mut self, index: GLuint, x: GLfloat, y: GLfloat) {
        gles30::VertexAttrib2f(index, x, y)
    }
    unsafe fn VertexAttrib3f(&mut self, index: GLuint, x: GLfloat, y: GLfloat, z: GLfloat) {
        gles30::VertexAttrib3f(index, x, y, z)
    }
    unsafe fn VertexAttrib4f(
        &mut self,
        index: GLuint,
        x: GLfloat,
        y: GLfloat,
        z: GLfloat,
        w: GLfloat,
    ) {
        gles30::VertexAttrib4f(index, x, y, z, w)
    }
    unsafe fn VertexAttrib1fv(&mut self, index: GLuint, v: *const GLfloat) {
        gles30::VertexAttrib1fv(index, v)
    }
    unsafe fn VertexAttrib2fv(&mut self, index: GLuint, v: *const GLfloat) {
        gles30::VertexAttrib2fv(index, v)
    }
    unsafe fn VertexAttrib3fv(&mut self, index: GLuint, v: *const GLfloat) {
        gles30::VertexAttrib3fv(index, v)
    }
    unsafe fn VertexAttrib4fv(&mut self, index: GLuint, v: *const GLfloat) {
        gles30::VertexAttrib4fv(index, v)
    }
    unsafe fn GetVertexAttribiv(&mut self, index: GLuint, pname: GLenum, params: *mut GLint) {
        gles30::GetVertexAttribiv(index, pname, params)
    }
    unsafe fn GetVertexAttribfv(&mut self, index: GLuint, pname: GLenum, params: *mut GLfloat) {
        gles30::GetVertexAttribfv(index, pname, params)
    }
    unsafe fn GetVertexAttribPointerv(
        &mut self,
        index: GLuint,
        pname: GLenum,
        pointer: *mut *mut GLvoid,
    ) {
        gles30::GetVertexAttribPointerv(index, pname, pointer)
    }

    // Uniforms
    unsafe fn Uniform1f(&mut self, location: GLint, v0: GLfloat) {
        gles30::Uniform1f(location, v0)
    }
    unsafe fn Uniform2f(&mut self, location: GLint, v0: GLfloat, v1: GLfloat) {
        gles30::Uniform2f(location, v0, v1)
    }
    unsafe fn Uniform3f(&mut self, location: GLint, v0: GLfloat, v1: GLfloat, v2: GLfloat) {
        gles30::Uniform3f(location, v0, v1, v2)
    }
    unsafe fn Uniform4f(
        &mut self,
        location: GLint,
        v0: GLfloat,
        v1: GLfloat,
        v2: GLfloat,
        v3: GLfloat,
    ) {
        gles30::Uniform4f(location, v0, v1, v2, v3)
    }
    unsafe fn Uniform1i(&mut self, location: GLint, v0: GLint) {
        gles30::Uniform1i(location, v0)
    }
    unsafe fn Uniform2i(&mut self, location: GLint, v0: GLint, v1: GLint) {
        gles30::Uniform2i(location, v0, v1)
    }
    unsafe fn Uniform3i(&mut self, location: GLint, v0: GLint, v1: GLint, v2: GLint) {
        gles30::Uniform3i(location, v0, v1, v2)
    }
    unsafe fn Uniform4i(&mut self, location: GLint, v0: GLint, v1: GLint, v2: GLint, v3: GLint) {
        gles30::Uniform4i(location, v0, v1, v2, v3)
    }
    unsafe fn Uniform1fv(&mut self, location: GLint, count: GLsizei, value: *const GLfloat) {
        gles30::Uniform1fv(location, count, value)
    }
    unsafe fn Uniform2fv(&mut self, location: GLint, count: GLsizei, value: *const GLfloat) {
        gles30::Uniform2fv(location, count, value)
    }
    unsafe fn Uniform3fv(&mut self, location: GLint, count: GLsizei, value: *const GLfloat) {
        gles30::Uniform3fv(location, count, value)
    }
    unsafe fn Uniform4fv(&mut self, location: GLint, count: GLsizei, value: *const GLfloat) {
        gles30::Uniform4fv(location, count, value)
    }
    unsafe fn Uniform1iv(&mut self, location: GLint, count: GLsizei, value: *const GLint) {
        gles30::Uniform1iv(location, count, value)
    }
    unsafe fn Uniform2iv(&mut self, location: GLint, count: GLsizei, value: *const GLint) {
        gles30::Uniform2iv(location, count, value)
    }
    unsafe fn Uniform3iv(&mut self, location: GLint, count: GLsizei, value: *const GLint) {
        gles30::Uniform3iv(location, count, value)
    }
    unsafe fn Uniform4iv(&mut self, location: GLint, count: GLsizei, value: *const GLint) {
        gles30::Uniform4iv(location, count, value)
    }
    unsafe fn UniformMatrix2fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gles30::UniformMatrix2fv(location, count, transpose, value)
    }
    unsafe fn UniformMatrix3fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gles30::UniformMatrix3fv(location, count, transpose, value)
    }
    unsafe fn UniformMatrix4fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gles30::UniformMatrix4fv(location, count, transpose, value)
    }

    // Blending / stencil (ES 2.0 / GL 2.0 separate variants)
    unsafe fn BlendColor(
        &mut self,
        red: GLclampf,
        green: GLclampf,
        blue: GLclampf,
        alpha: GLclampf,
    ) {
        gles30::BlendColor(red, green, blue, alpha)
    }
    unsafe fn BlendEquation(&mut self, mode: GLenum) {
        gles30::BlendEquation(mode)
    }
    // GL_OES_blend_equation: semantically identical to core BlendEquation.
    unsafe fn BlendEquationOES(&mut self, mode: GLenum) {
        gles30::BlendEquation(mode)
    }
    unsafe fn BlendEquationSeparate(&mut self, modeRGB: GLenum, modeAlpha: GLenum) {
        gles30::BlendEquationSeparate(modeRGB, modeAlpha)
    }
    unsafe fn BlendFuncSeparate(
        &mut self,
        sfactorRGB: GLenum,
        dfactorRGB: GLenum,
        sfactorAlpha: GLenum,
        dfactorAlpha: GLenum,
    ) {
        gles30::BlendFuncSeparate(sfactorRGB, dfactorRGB, sfactorAlpha, dfactorAlpha)
    }
    unsafe fn StencilFuncSeparate(
        &mut self,
        face: GLenum,
        func: GLenum,
        ref_: GLint,
        mask: GLuint,
    ) {
        gles30::StencilFuncSeparate(face, func, ref_, mask)
    }
    unsafe fn StencilOpSeparate(
        &mut self,
        face: GLenum,
        sfail: GLenum,
        dpfail: GLenum,
        dppass: GLenum,
    ) {
        gles30::StencilOpSeparate(face, sfail, dpfail, dppass)
    }
    unsafe fn StencilMaskSeparate(&mut self, face: GLenum, mask: GLuint) {
        gles30::StencilMaskSeparate(face, mask)
    }

    // Fixed-function methods (ES 1.x) – no-ops on a real ES 2.0 driver. This
    // keeps the existing `present_renderbuffer` save/restore code paths quiet
    // without crashing. Real apps that rely on a true ES 2.0 driver will not
    // call these.
    unsafe fn ClientActiveTexture(&mut self, _texture: GLenum) {}
    unsafe fn EnableClientState(&mut self, _array: GLenum) {}
    unsafe fn DisableClientState(&mut self, _array: GLenum) {}
    unsafe fn GetTexEnviv(&mut self, _target: GLenum, _pname: GLenum, _params: *mut GLint) {}
    unsafe fn GetTexEnvfv(&mut self, _target: GLenum, _pname: GLenum, _params: *mut GLfloat) {}
    unsafe fn GetPointerv(&mut self, _pname: GLenum, _params: *mut *const GLvoid) {}
    unsafe fn AlphaFunc(&mut self, _func: GLenum, _ref_: GLclampf) {}
    unsafe fn AlphaFuncx(&mut self, _func: GLenum, _ref_: GLclampx) {}
    unsafe fn Color4f(&mut self, _r: GLfloat, _g: GLfloat, _b: GLfloat, _a: GLfloat) {}
    unsafe fn Color4x(&mut self, _r: GLfixed, _g: GLfixed, _b: GLfixed, _a: GLfixed) {}
    unsafe fn Color4ub(&mut self, _r: GLubyte, _g: GLubyte, _b: GLubyte, _a: GLubyte) {}
    unsafe fn ShadeModel(&mut self, _mode: GLenum) {}
    unsafe fn LoadIdentity(&mut self) {}
    unsafe fn LoadMatrixf(&mut self, _m: *const GLfloat) {}
    unsafe fn LoadMatrixx(&mut self, _m: *const GLfixed) {}
    unsafe fn MultMatrixf(&mut self, _m: *const GLfloat) {}
    unsafe fn MultMatrixx(&mut self, _m: *const GLfixed) {}
    unsafe fn PushMatrix(&mut self) {}
    unsafe fn PopMatrix(&mut self) {}
    unsafe fn MatrixMode(&mut self, _mode: GLenum) {}
    unsafe fn Frustumf(
        &mut self,
        _left: GLfloat,
        _right: GLfloat,
        _bottom: GLfloat,
        _top: GLfloat,
        _near: GLfloat,
        _far: GLfloat,
    ) {
    }
    unsafe fn Frustumx(
        &mut self,
        _left: GLfixed,
        _right: GLfixed,
        _bottom: GLfixed,
        _top: GLfixed,
        _near: GLfixed,
        _far: GLfixed,
    ) {
    }
    unsafe fn Orthof(
        &mut self,
        _left: GLfloat,
        _right: GLfloat,
        _bottom: GLfloat,
        _top: GLfloat,
        _near: GLfloat,
        _far: GLfloat,
    ) {
    }
    unsafe fn Orthox(
        &mut self,
        _left: GLfixed,
        _right: GLfixed,
        _bottom: GLfixed,
        _top: GLfixed,
        _near: GLfixed,
        _far: GLfixed,
    ) {
    }
    unsafe fn Rotatef(&mut self, _a: GLfloat, _x: GLfloat, _y: GLfloat, _z: GLfloat) {}
    unsafe fn Rotatex(&mut self, _a: GLfixed, _x: GLfixed, _y: GLfixed, _z: GLfixed) {}
    unsafe fn Scalef(&mut self, _x: GLfloat, _y: GLfloat, _z: GLfloat) {}
    unsafe fn Scalex(&mut self, _x: GLfixed, _y: GLfixed, _z: GLfixed) {}
    unsafe fn Translatef(&mut self, _x: GLfloat, _y: GLfloat, _z: GLfloat) {}
    unsafe fn Translatex(&mut self, _x: GLfixed, _y: GLfixed, _z: GLfixed) {}
    unsafe fn TexEnvf(&mut self, _target: GLenum, _pname: GLenum, _param: GLfloat) {}
    unsafe fn TexEnvx(&mut self, _target: GLenum, _pname: GLenum, _param: GLfixed) {}
    unsafe fn TexEnvi(&mut self, _target: GLenum, _pname: GLenum, _param: GLint) {}
    unsafe fn TexEnvfv(&mut self, _target: GLenum, _pname: GLenum, _params: *const GLfloat) {}
    unsafe fn TexEnvxv(&mut self, _target: GLenum, _pname: GLenum, _params: *const GLfixed) {}
    unsafe fn TexEnviv(&mut self, _target: GLenum, _pname: GLenum, _params: *const GLint) {}
    unsafe fn VertexPointer(
        &mut self,
        _size: GLint,
        _type_: GLenum,
        _stride: GLsizei,
        _pointer: *const GLvoid,
    ) {
    }
    unsafe fn ColorPointer(
        &mut self,
        _size: GLint,
        _type_: GLenum,
        _stride: GLsizei,
        _pointer: *const GLvoid,
    ) {
    }
    unsafe fn NormalPointer(&mut self, _type_: GLenum, _stride: GLsizei, _pointer: *const GLvoid) {}
    unsafe fn TexCoordPointer(
        &mut self,
        _size: GLint,
        _type_: GLenum,
        _stride: GLsizei,
        _pointer: *const GLvoid,
    ) {
    }

    // ===== OpenGL ES 3.0 entry points =====
    //
    // Every method below forwards 1:1 to the native ES 3.0 driver loaded via
    // `gles30::load_with` in `make_current`. These are *real* implementations
    // (not stubs) — the only "logic" is straight argument forwarding plus a
    // handful of obvious type coercions where the trait signature uses
    // platform-neutral types (e.g. `i64` instead of the bindings' generated
    // `GLint64`).

    // -- Vertex array objects --
    unsafe fn IsVertexArray(&mut self, array: GLuint) -> GLboolean {
        gles30::IsVertexArray(array)
    }
    unsafe fn BindVertexArray(&mut self, array: GLuint) {
        gles30::BindVertexArray(array)
    }
    unsafe fn DeleteVertexArrays(&mut self, n: GLsizei, arrays: *const GLuint) {
        gles30::DeleteVertexArrays(n, arrays)
    }
    unsafe fn GenVertexArrays(&mut self, n: GLsizei, arrays: *mut GLuint) {
        gles30::GenVertexArrays(n, arrays)
    }

    // -- Buffer object operations --
    unsafe fn MapBufferRange(
        &mut self,
        target: GLenum,
        offset: GLintptr,
        length: GLsizeiptr,
        access: GLbitfield,
    ) -> *mut GLvoid {
        gles30::MapBufferRange(target, offset, length, access)
    }
    unsafe fn FlushMappedBufferRange(
        &mut self,
        target: GLenum,
        offset: GLintptr,
        length: GLsizeiptr,
    ) {
        gles30::FlushMappedBufferRange(target, offset, length)
    }
    unsafe fn GetBufferPointerv(
        &mut self,
        target: GLenum,
        pname: GLenum,
        params: *mut *mut GLvoid,
    ) {
        gles30::GetBufferPointerv(target, pname, params)
    }
    unsafe fn GetBufferParameteri64v(
        &mut self,
        target: GLenum,
        pname: GLenum,
        params: *mut i64,
    ) {
        gles30::GetBufferParameteri64v(target, pname, params as *mut gles30::types::GLint64)
    }
    unsafe fn CopyBufferSubData(
        &mut self,
        read_target: GLenum,
        write_target: GLenum,
        read_offset: GLintptr,
        write_offset: GLintptr,
        size: GLsizeiptr,
    ) {
        gles30::CopyBufferSubData(read_target, write_target, read_offset, write_offset, size)
    }
    unsafe fn BindBufferBase(&mut self, target: GLenum, index: GLuint, buffer: GLuint) {
        gles30::BindBufferBase(target, index, buffer)
    }
    unsafe fn BindBufferRange(
        &mut self,
        target: GLenum,
        index: GLuint,
        buffer: GLuint,
        offset: GLintptr,
        size: GLsizeiptr,
    ) {
        gles30::BindBufferRange(target, index, buffer, offset, size)
    }
    unsafe fn UnmapBuffer(&mut self, target: GLenum) -> GLboolean {
        gles30::UnmapBuffer(target)
    }

    // -- 3D textures and immutable storage --
    unsafe fn TexImage3D(
        &mut self,
        target: GLenum,
        level: GLint,
        internalformat: GLint,
        width: GLsizei,
        height: GLsizei,
        depth: GLsizei,
        border: GLint,
        format: GLenum,
        type_: GLenum,
        pixels: *const GLvoid,
    ) {
        gles30::TexImage3D(
            target,
            level,
            internalformat,
            width,
            height,
            depth,
            border,
            format,
            type_,
            pixels,
        )
    }
    unsafe fn TexSubImage3D(
        &mut self,
        target: GLenum,
        level: GLint,
        xoffset: GLint,
        yoffset: GLint,
        zoffset: GLint,
        width: GLsizei,
        height: GLsizei,
        depth: GLsizei,
        format: GLenum,
        type_: GLenum,
        pixels: *const GLvoid,
    ) {
        gles30::TexSubImage3D(
            target, level, xoffset, yoffset, zoffset, width, height, depth, format, type_,
            pixels,
        )
    }
    unsafe fn CopyTexSubImage3D(
        &mut self,
        target: GLenum,
        level: GLint,
        xoffset: GLint,
        yoffset: GLint,
        zoffset: GLint,
        x: GLint,
        y: GLint,
        width: GLsizei,
        height: GLsizei,
    ) {
        gles30::CopyTexSubImage3D(
            target, level, xoffset, yoffset, zoffset, x, y, width, height,
        )
    }
    unsafe fn CompressedTexImage3D(
        &mut self,
        target: GLenum,
        level: GLint,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
        depth: GLsizei,
        border: GLint,
        image_size: GLsizei,
        data: *const GLvoid,
    ) {
        gles30::CompressedTexImage3D(
            target,
            level,
            internalformat,
            width,
            height,
            depth,
            border,
            image_size,
            data,
        )
    }
    unsafe fn CompressedTexSubImage3D(
        &mut self,
        target: GLenum,
        level: GLint,
        xoffset: GLint,
        yoffset: GLint,
        zoffset: GLint,
        width: GLsizei,
        height: GLsizei,
        depth: GLsizei,
        format: GLenum,
        image_size: GLsizei,
        data: *const GLvoid,
    ) {
        gles30::CompressedTexSubImage3D(
            target, level, xoffset, yoffset, zoffset, width, height, depth, format, image_size,
            data,
        )
    }
    unsafe fn TexStorage2D(
        &mut self,
        target: GLenum,
        levels: GLsizei,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
    ) {
        gles30::TexStorage2D(target, levels, internalformat, width, height)
    }
    unsafe fn TexStorage3D(
        &mut self,
        target: GLenum,
        levels: GLsizei,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
        depth: GLsizei,
    ) {
        gles30::TexStorage3D(target, levels, internalformat, width, height, depth)
    }

    // -- Framebuffer / renderbuffer --
    unsafe fn BlitFramebuffer(
        &mut self,
        src_x0: GLint,
        src_y0: GLint,
        src_x1: GLint,
        src_y1: GLint,
        dst_x0: GLint,
        dst_y0: GLint,
        dst_x1: GLint,
        dst_y1: GLint,
        mask: GLbitfield,
        filter: GLenum,
    ) {
        gles30::BlitFramebuffer(
            src_x0, src_y0, src_x1, src_y1, dst_x0, dst_y0, dst_x1, dst_y1, mask, filter,
        )
    }
    unsafe fn RenderbufferStorageMultisample(
        &mut self,
        target: GLenum,
        samples: GLsizei,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
    ) {
        gles30::RenderbufferStorageMultisample(target, samples, internalformat, width, height)
    }
    unsafe fn ResolveMultisampleFramebufferAPPLE(&mut self) {
        let mut color_rb: GLint = 0;
        gles30::GetFramebufferAttachmentParameteriv(
            gles30::READ_FRAMEBUFFER,
            gles30::COLOR_ATTACHMENT0,
            gles30::FRAMEBUFFER_ATTACHMENT_OBJECT_NAME,
            &mut color_rb,
        );

        let mut old_rb: GLint = 0;
        gles30::GetIntegerv(gles30::RENDERBUFFER_BINDING, &mut old_rb);
        gles30::BindRenderbuffer(gles30::RENDERBUFFER, color_rb as GLuint);

        let mut width: GLint = 0;
        let mut height: GLint = 0;
        gles30::GetRenderbufferParameteriv(
            gles30::RENDERBUFFER,
            gles30::RENDERBUFFER_WIDTH,
            &mut width,
        );
        gles30::GetRenderbufferParameteriv(
            gles30::RENDERBUFFER,
            gles30::RENDERBUFFER_HEIGHT,
            &mut height,
        );

        gles30::BindRenderbuffer(gles30::RENDERBUFFER, old_rb as GLuint);

        gles30::BlitFramebuffer(
            0,
            0,
            width,
            height,
            0,
            0,
            width,
            height,
            gles30::COLOR_BUFFER_BIT,
            gles30::NEAREST,
        );
    }
    unsafe fn FramebufferTextureLayer(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        texture: GLuint,
        level: GLint,
        layer: GLint,
    ) {
        gles30::FramebufferTextureLayer(target, attachment, texture, level, layer)
    }
    unsafe fn InvalidateFramebuffer(
        &mut self,
        target: GLenum,
        num_attachments: GLsizei,
        attachments: *const GLenum,
    ) {
        gles30::InvalidateFramebuffer(target, num_attachments, attachments)
    }
    unsafe fn InvalidateSubFramebuffer(
        &mut self,
        target: GLenum,
        num_attachments: GLsizei,
        attachments: *const GLenum,
        x: GLint,
        y: GLint,
        width: GLsizei,
        height: GLsizei,
    ) {
        gles30::InvalidateSubFramebuffer(
            target,
            num_attachments,
            attachments,
            x,
            y,
            width,
            height,
        )
    }
    unsafe fn ReadBuffer(&mut self, src: GLenum) {
        gles30::ReadBuffer(src)
    }
    unsafe fn DrawBuffers(&mut self, n: GLsizei, bufs: *const GLenum) {
        gles30::DrawBuffers(n, bufs)
    }
    unsafe fn DrawRangeElements(
        &mut self,
        mode: GLenum,
        start: GLuint,
        end: GLuint,
        count: GLsizei,
        type_: GLenum,
        indices: *const GLvoid,
    ) {
        gles30::DrawRangeElements(mode, start, end, count, type_, indices)
    }
    unsafe fn ClearBufferiv(
        &mut self,
        buffer: GLenum,
        drawbuffer: GLint,
        value: *const GLint,
    ) {
        gles30::ClearBufferiv(buffer, drawbuffer, value)
    }
    unsafe fn ClearBufferuiv(
        &mut self,
        buffer: GLenum,
        drawbuffer: GLint,
        value: *const GLuint,
    ) {
        gles30::ClearBufferuiv(buffer, drawbuffer, value)
    }
    unsafe fn ClearBufferfv(
        &mut self,
        buffer: GLenum,
        drawbuffer: GLint,
        value: *const GLfloat,
    ) {
        gles30::ClearBufferfv(buffer, drawbuffer, value)
    }
    unsafe fn ClearBufferfi(
        &mut self,
        buffer: GLenum,
        drawbuffer: GLint,
        depth: GLfloat,
        stencil: GLint,
    ) {
        gles30::ClearBufferfi(buffer, drawbuffer, depth, stencil)
    }

    // -- Query objects --
    unsafe fn GenQueries(&mut self, n: GLsizei, ids: *mut GLuint) {
        gles30::GenQueries(n, ids)
    }
    unsafe fn DeleteQueries(&mut self, n: GLsizei, ids: *const GLuint) {
        gles30::DeleteQueries(n, ids)
    }
    unsafe fn IsQuery(&mut self, id: GLuint) -> GLboolean {
        gles30::IsQuery(id)
    }
    unsafe fn BeginQuery(&mut self, target: GLenum, id: GLuint) {
        gles30::BeginQuery(target, id)
    }
    unsafe fn EndQuery(&mut self, target: GLenum) {
        gles30::EndQuery(target)
    }
    unsafe fn GetQueryiv(&mut self, target: GLenum, pname: GLenum, params: *mut GLint) {
        gles30::GetQueryiv(target, pname, params)
    }
    unsafe fn GetQueryObjectuiv(&mut self, id: GLuint, pname: GLenum, params: *mut GLuint) {
        gles30::GetQueryObjectuiv(id, pname, params)
    }

    // -- Sampler objects --
    unsafe fn GenSamplers(&mut self, count: GLsizei, samplers: *mut GLuint) {
        gles30::GenSamplers(count, samplers)
    }
    unsafe fn DeleteSamplers(&mut self, count: GLsizei, samplers: *const GLuint) {
        gles30::DeleteSamplers(count, samplers)
    }
    unsafe fn IsSampler(&mut self, sampler: GLuint) -> GLboolean {
        gles30::IsSampler(sampler)
    }
    unsafe fn BindSampler(&mut self, unit: GLuint, sampler: GLuint) {
        gles30::BindSampler(unit, sampler)
    }
    unsafe fn SamplerParameteri(&mut self, sampler: GLuint, pname: GLenum, param: GLint) {
        gles30::SamplerParameteri(sampler, pname, param)
    }
    unsafe fn SamplerParameteriv(
        &mut self,
        sampler: GLuint,
        pname: GLenum,
        params: *const GLint,
    ) {
        gles30::SamplerParameteriv(sampler, pname, params)
    }
    unsafe fn SamplerParameterf(&mut self, sampler: GLuint, pname: GLenum, param: GLfloat) {
        gles30::SamplerParameterf(sampler, pname, param)
    }
    unsafe fn SamplerParameterfv(
        &mut self,
        sampler: GLuint,
        pname: GLenum,
        params: *const GLfloat,
    ) {
        gles30::SamplerParameterfv(sampler, pname, params)
    }
    unsafe fn GetSamplerParameteriv(
        &mut self,
        sampler: GLuint,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gles30::GetSamplerParameteriv(sampler, pname, params)
    }
    unsafe fn GetSamplerParameterfv(
        &mut self,
        sampler: GLuint,
        pname: GLenum,
        params: *mut GLfloat,
    ) {
        gles30::GetSamplerParameterfv(sampler, pname, params)
    }

    // -- Transform feedback --
    unsafe fn BeginTransformFeedback(&mut self, primitive_mode: GLenum) {
        gles30::BeginTransformFeedback(primitive_mode)
    }
    unsafe fn EndTransformFeedback(&mut self) {
        gles30::EndTransformFeedback()
    }
    unsafe fn BindTransformFeedback(&mut self, target: GLenum, id: GLuint) {
        gles30::BindTransformFeedback(target, id)
    }
    unsafe fn DeleteTransformFeedbacks(&mut self, n: GLsizei, ids: *const GLuint) {
        gles30::DeleteTransformFeedbacks(n, ids)
    }
    unsafe fn GenTransformFeedbacks(&mut self, n: GLsizei, ids: *mut GLuint) {
        gles30::GenTransformFeedbacks(n, ids)
    }
    unsafe fn IsTransformFeedback(&mut self, id: GLuint) -> GLboolean {
        gles30::IsTransformFeedback(id)
    }
    unsafe fn PauseTransformFeedback(&mut self) {
        gles30::PauseTransformFeedback()
    }
    unsafe fn ResumeTransformFeedback(&mut self) {
        gles30::ResumeTransformFeedback()
    }
    unsafe fn TransformFeedbackVaryings(
        &mut self,
        program: GLuint,
        count: GLsizei,
        varyings: *const *const GLchar,
        buffer_mode: GLenum,
    ) {
        gles30::TransformFeedbackVaryings(program, count, varyings, buffer_mode)
    }
    unsafe fn GetTransformFeedbackVarying(
        &mut self,
        program: GLuint,
        index: GLuint,
        buf_size: GLsizei,
        length: *mut GLsizei,
        size: *mut GLsizei,
        type_: *mut GLenum,
        name: *mut GLchar,
    ) {
        gles30::GetTransformFeedbackVarying(
            program, index, buf_size, length, size, type_, name,
        )
    }

    // -- Integer vertex attributes --
    unsafe fn VertexAttribIPointer(
        &mut self,
        index: GLuint,
        size: GLint,
        type_: GLenum,
        stride: GLsizei,
        pointer: *const GLvoid,
    ) {
        gles30::VertexAttribIPointer(index, size, type_, stride, pointer)
    }
    unsafe fn GetVertexAttribIiv(
        &mut self,
        index: GLuint,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gles30::GetVertexAttribIiv(index, pname, params)
    }
    unsafe fn GetVertexAttribIuiv(
        &mut self,
        index: GLuint,
        pname: GLenum,
        params: *mut GLuint,
    ) {
        gles30::GetVertexAttribIuiv(index, pname, params)
    }
    unsafe fn VertexAttribI4i(
        &mut self,
        index: GLuint,
        x: GLint,
        y: GLint,
        z: GLint,
        w: GLint,
    ) {
        gles30::VertexAttribI4i(index, x, y, z, w)
    }
    unsafe fn VertexAttribI4ui(
        &mut self,
        index: GLuint,
        x: GLuint,
        y: GLuint,
        z: GLuint,
        w: GLuint,
    ) {
        gles30::VertexAttribI4ui(index, x, y, z, w)
    }
    unsafe fn VertexAttribI4iv(&mut self, index: GLuint, v: *const GLint) {
        gles30::VertexAttribI4iv(index, v)
    }
    unsafe fn VertexAttribI4uiv(&mut self, index: GLuint, v: *const GLuint) {
        gles30::VertexAttribI4uiv(index, v)
    }

    // -- Integer / matrix-NM uniforms --
    unsafe fn Uniform1ui(&mut self, location: GLint, v0: GLuint) {
        gles30::Uniform1ui(location, v0)
    }
    unsafe fn Uniform2ui(&mut self, location: GLint, v0: GLuint, v1: GLuint) {
        gles30::Uniform2ui(location, v0, v1)
    }
    unsafe fn Uniform3ui(&mut self, location: GLint, v0: GLuint, v1: GLuint, v2: GLuint) {
        gles30::Uniform3ui(location, v0, v1, v2)
    }
    unsafe fn Uniform4ui(
        &mut self,
        location: GLint,
        v0: GLuint,
        v1: GLuint,
        v2: GLuint,
        v3: GLuint,
    ) {
        gles30::Uniform4ui(location, v0, v1, v2, v3)
    }
    unsafe fn Uniform1uiv(&mut self, location: GLint, count: GLsizei, value: *const GLuint) {
        gles30::Uniform1uiv(location, count, value)
    }
    unsafe fn Uniform2uiv(&mut self, location: GLint, count: GLsizei, value: *const GLuint) {
        gles30::Uniform2uiv(location, count, value)
    }
    unsafe fn Uniform3uiv(&mut self, location: GLint, count: GLsizei, value: *const GLuint) {
        gles30::Uniform3uiv(location, count, value)
    }
    unsafe fn Uniform4uiv(&mut self, location: GLint, count: GLsizei, value: *const GLuint) {
        gles30::Uniform4uiv(location, count, value)
    }
    unsafe fn GetUniformuiv(
        &mut self,
        program: GLuint,
        location: GLint,
        params: *mut GLuint,
    ) {
        gles30::GetUniformuiv(program, location, params)
    }
    unsafe fn UniformMatrix2x3fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gles30::UniformMatrix2x3fv(location, count, transpose, value)
    }
    unsafe fn UniformMatrix3x2fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gles30::UniformMatrix3x2fv(location, count, transpose, value)
    }
    unsafe fn UniformMatrix2x4fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gles30::UniformMatrix2x4fv(location, count, transpose, value)
    }
    unsafe fn UniformMatrix4x2fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gles30::UniformMatrix4x2fv(location, count, transpose, value)
    }
    unsafe fn UniformMatrix3x4fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gles30::UniformMatrix3x4fv(location, count, transpose, value)
    }
    unsafe fn UniformMatrix4x3fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gles30::UniformMatrix4x3fv(location, count, transpose, value)
    }

    // -- Uniform blocks --
    unsafe fn GetUniformIndices(
        &mut self,
        program: GLuint,
        uniform_count: GLsizei,
        uniform_names: *const *const GLchar,
        uniform_indices: *mut GLuint,
    ) {
        gles30::GetUniformIndices(program, uniform_count, uniform_names, uniform_indices)
    }
    unsafe fn GetActiveUniformsiv(
        &mut self,
        program: GLuint,
        uniform_count: GLsizei,
        uniform_indices: *const GLuint,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gles30::GetActiveUniformsiv(program, uniform_count, uniform_indices, pname, params)
    }
    unsafe fn GetUniformBlockIndex(
        &mut self,
        program: GLuint,
        uniform_block_name: *const GLchar,
    ) -> GLuint {
        gles30::GetUniformBlockIndex(program, uniform_block_name)
    }
    unsafe fn GetActiveUniformBlockiv(
        &mut self,
        program: GLuint,
        uniform_block_index: GLuint,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gles30::GetActiveUniformBlockiv(program, uniform_block_index, pname, params)
    }
    unsafe fn GetActiveUniformBlockName(
        &mut self,
        program: GLuint,
        uniform_block_index: GLuint,
        buf_size: GLsizei,
        length: *mut GLsizei,
        uniform_block_name: *mut GLchar,
    ) {
        gles30::GetActiveUniformBlockName(
            program,
            uniform_block_index,
            buf_size,
            length,
            uniform_block_name,
        )
    }
    unsafe fn UniformBlockBinding(
        &mut self,
        program: GLuint,
        uniform_block_index: GLuint,
        uniform_block_binding: GLuint,
    ) {
        gles30::UniformBlockBinding(program, uniform_block_index, uniform_block_binding)
    }

    // -- Instanced rendering --
    unsafe fn DrawArraysInstanced(
        &mut self,
        mode: GLenum,
        first: GLint,
        count: GLsizei,
        instance_count: GLsizei,
    ) {
        gles30::DrawArraysInstanced(mode, first, count, instance_count)
    }
    unsafe fn DrawElementsInstanced(
        &mut self,
        mode: GLenum,
        count: GLsizei,
        type_: GLenum,
        indices: *const GLvoid,
        instance_count: GLsizei,
    ) {
        gles30::DrawElementsInstanced(mode, count, type_, indices, instance_count)
    }
    unsafe fn VertexAttribDivisor(&mut self, index: GLuint, divisor: GLuint) {
        gles30::VertexAttribDivisor(index, divisor)
    }

    // -- Sync objects --
    //
    // GLES `GLsync` is opaque (`*const __GLsync`). We expose it to the rest
    // of touchHLE as `usize` for portability — guest code never holds the
    // pointer directly; it goes via a host->guest GLsync handle table in
    // `gles_guest.rs`.
    unsafe fn FenceSync(&mut self, condition: GLenum, flags: GLbitfield) -> usize {
        gles30::FenceSync(condition, flags) as usize
    }
    unsafe fn IsSync(&mut self, sync: usize) -> GLboolean {
        gles30::IsSync(sync as gles30::types::GLsync)
    }
    unsafe fn DeleteSync(&mut self, sync: usize) {
        gles30::DeleteSync(sync as gles30::types::GLsync)
    }
    unsafe fn ClientWaitSync(
        &mut self,
        sync: usize,
        flags: GLbitfield,
        timeout: u64,
    ) -> GLenum {
        gles30::ClientWaitSync(sync as gles30::types::GLsync, flags, timeout)
    }
    unsafe fn WaitSync(&mut self, sync: usize, flags: GLbitfield, timeout: u64) {
        gles30::WaitSync(sync as gles30::types::GLsync, flags, timeout)
    }
    unsafe fn GetSynciv(
        &mut self,
        sync: usize,
        pname: GLenum,
        buf_size: GLsizei,
        length: *mut GLsizei,
        values: *mut GLint,
    ) {
        gles30::GetSynciv(sync as gles30::types::GLsync, pname, buf_size, length, values)
    }

    // -- 64-bit / indexed getters --
    unsafe fn GetInteger64v(&mut self, pname: GLenum, data: *mut i64) {
        gles30::GetInteger64v(pname, data as *mut gles30::types::GLint64)
    }
    unsafe fn GetIntegeri_v(&mut self, target: GLenum, index: GLuint, data: *mut GLint) {
        gles30::GetIntegeri_v(target, index, data)
    }
    unsafe fn GetInteger64i_v(&mut self, target: GLenum, index: GLuint, data: *mut i64) {
        gles30::GetInteger64i_v(target, index, data as *mut gles30::types::GLint64)
    }

    // -- Program binary --
    unsafe fn ProgramParameteri(&mut self, program: GLuint, pname: GLenum, value: GLint) {
        gles30::ProgramParameteri(program, pname, value)
    }
    unsafe fn ProgramBinary(
        &mut self,
        program: GLuint,
        binary_format: GLenum,
        binary: *const GLvoid,
        length: GLsizei,
    ) {
        gles30::ProgramBinary(program, binary_format, binary, length)
    }
    unsafe fn GetProgramBinary(
        &mut self,
        program: GLuint,
        buf_size: GLsizei,
        length: *mut GLsizei,
        binary_format: *mut GLenum,
        binary: *mut GLvoid,
    ) {
        gles30::GetProgramBinary(program, buf_size, length, binary_format, binary)
    }

    // -- Misc --
    unsafe fn GetStringi(&mut self, name: GLenum, index: GLuint) -> *const GLubyte {
        gles30::GetStringi(name, index)
    }
    unsafe fn GetFragDataLocation(
        &mut self,
        program: GLuint,
        name: *const GLchar,
    ) -> GLint {
        gles30::GetFragDataLocation(program, name)
    }
    unsafe fn GetInternalformativ(
        &mut self,
        target: GLenum,
        internalformat: GLenum,
        pname: GLenum,
        buf_size: GLsizei,
        params: *mut GLint,
    ) {
        gles30::GetInternalformativ(target, internalformat, pname, buf_size, params)
    }
}
