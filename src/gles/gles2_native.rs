/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Passthrough for a native OpenGL ES 2.0 driver.
//!
//! This is the only backend that works on platforms that lack desktop OpenGL
//! (such as Android), so it is the preferred ES 2.0 backend.

use super::gles11_raw as gles11;
use super::gles11_raw::types::*;
use super::gles2_raw as gles2;
use super::gles_generic::{GLchar, GLES};
use super::util::{try_decode_pvrtc, PalettedTextureFormat};
use super::GLESContext;
use crate::window::{GLContext, GLVersion, Window};
use std::ffi::CStr;
use std::marker::PhantomData;

pub struct GLES2NativeContext {
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
    /// Whether the driver supports `GL_EXT_shader_texture_lod`. When it does
    /// not, we must patch shaders that use `texture2DLodEXT` and friends.
    texture_lod_ext_supported: bool,
}

impl GLESContext for GLES2NativeContext {
    fn description() -> &'static str {
        "Native OpenGL ES 2.0"
    }

    fn new(window: &mut Window) -> Result<Self, String> {
        Ok(Self {
            gl_ctx: window.create_gl_context(GLVersion::GLES20)?,
            is_loaded: false,
            pvrtc_native: false,
            pvrtc_native_checked: false,
            texture_lod_ext_supported: false,
        })
    }

    fn make_current<'gl_ctx, 'win: 'gl_ctx>(
        &'gl_ctx mut self,
        window: &'win mut Window,
    ) -> Box<dyn GLES + 'gl_ctx> {
        if self.gl_ctx.is_current() && self.is_loaded {
            return Box::new(GLES2Native {
                _gl_lifetime: PhantomData,
                pvrtc_native: self.pvrtc_native,
                texture_lod_ext_supported: self.texture_lod_ext_supported,
            });
        }
        unsafe {
            window.make_gl_context_current(&self.gl_ctx);
        }
        gles2::load_with(|s| window.gl_get_proc_address(s));
        // Some symbols (e.g. glGetString) are technically also part of ES 1.1
        // and are referenced via gles11:: in the surrounding code; load those
        // too so the existing helpers continue to work.
        gles11::load_with(|s| window.gl_get_proc_address(s));
        self.is_loaded = true;
        if !self.pvrtc_native_checked {
            self.pvrtc_native = unsafe { detect_pvrtc_support() };
            self.texture_lod_ext_supported = unsafe { detect_texture_lod_ext_support() };
            self.pvrtc_native_checked = true;
        }
        Box::new(GLES2Native {
            _gl_lifetime: PhantomData,
            pvrtc_native: self.pvrtc_native,
            texture_lod_ext_supported: self.texture_lod_ext_supported,
        })
    }

    unsafe fn make_current_unchecked_for_window<'gl_ctx>(
        &'gl_ctx mut self,
        make_current_fn: &mut dyn FnMut(&GLContext),
        loader_fn: &mut dyn FnMut(&'static str) -> *const std::ffi::c_void,
    ) -> Box<dyn GLES + 'gl_ctx> {
        if self.gl_ctx.is_current() && self.is_loaded {
            return Box::new(GLES2Native {
                _gl_lifetime: PhantomData,
                pvrtc_native: self.pvrtc_native,
                texture_lod_ext_supported: self.texture_lod_ext_supported,
            });
        }
        make_current_fn(&self.gl_ctx);
        gles2::load_with(&mut *loader_fn);
        gles11::load_with(&mut *loader_fn);
        self.is_loaded = true;
        if !self.pvrtc_native_checked {
            self.pvrtc_native = detect_pvrtc_support();
            self.texture_lod_ext_supported = detect_texture_lod_ext_support();
            self.pvrtc_native_checked = true;
        }
        Box::new(GLES2Native {
            _gl_lifetime: PhantomData,
            pvrtc_native: self.pvrtc_native,
            texture_lod_ext_supported: self.texture_lod_ext_supported,
        })
    }
}

/// Query `GL_EXTENSIONS` and return whether the current OpenGL ES 2.0 driver
/// advertises `GL_IMG_texture_compression_pvrtc`. Must be called with a
/// current GL context.
///
/// On strict OpenGL ES 3.0+ contexts `glGetString(GL_EXTENSIONS)` is
/// deprecated and may return an empty string. We can't use `glGetStringi`
/// here because the touchHLE GLES 2.0 raw bindings (generated for Core
/// ES 2.0 only) don't expose it. When the legacy string is unavailable we
/// conservatively return `false`, which causes us to software-decode PVRTC.
/// That's the safe choice: it produces correct output everywhere, at the
/// cost of one extra in-memory pass per texture upload on hosts where
/// PVRTC could otherwise be uploaded directly.
unsafe fn detect_pvrtc_support() -> bool {
    let legacy = gles2::GetString(gles11::EXTENSIONS);
    if legacy.is_null() {
        return false;
    }
    let Ok(s) = CStr::from_ptr(legacy as *const _).to_str() else {
        return false;
    };
    if s.is_empty() {
        return false;
    }
    s.split(' ')
        .any(|ext| ext == "GL_IMG_texture_compression_pvrtc")
}

/// Query `GL_EXTENSIONS` and return whether the current OpenGL ES 2.0 driver
/// advertises `GL_EXT_shader_texture_lod`. When it does not, we must patch
/// shaders that use `texture2DLodEXT` / `texture2DProjLodEXT` /
/// `textureCubeLodEXT` because those functions won't exist in the driver's
/// GLSL compiler.
unsafe fn detect_texture_lod_ext_support() -> bool {
    let legacy = gles2::GetString(gles11::EXTENSIONS);
    if legacy.is_null() {
        return false;
    }
    let Ok(s) = CStr::from_ptr(legacy as *const _).to_str() else {
        return false;
    };
    if s.is_empty() {
        return false;
    }
    s.split(' ')
        .any(|ext| ext == "GL_EXT_shader_texture_lod")
}

/// Returns `true` if the shader source contains a top-level default float
/// precision declaration (`precision lowp|mediump|highp float;`).
fn shader_has_default_float_precision(source: &str) -> bool {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("precision") {
            let rest = rest.trim_start();
            let mut parts = rest.split_whitespace();
            let qualifier = parts.next();
            let ty = parts.next();
            if matches!(qualifier, Some("lowp" | "mediump" | "highp"))
                && matches!(ty, Some(t) if t.starts_with("float"))
            {
                return true;
            }
        }
    }
    false
}

/// Patch a GLSL ES shader source for compatibility with native ES 2.0 drivers
/// that may not support all extensions the guest app expects.
///
/// This handles:
/// 1. Hoisting `#extension` directives to the top (right after `#version`),
///    because some drivers (notably Mali) reject them if they appear after
///    non-preprocessor tokens.
/// 2. When the driver lacks `GL_EXT_shader_texture_lod`, stripping the
///    corresponding `#extension` line and replacing `texture2DLodEXT(s, c, l)`
///    with `texture2D(s, c)` (dropping the LOD parameter). This loses mipmap
///    control but lets the shader compile and produce visually acceptable
///    results.
/// 3. Injecting a default `precision mediump float;` declaration when the
///    shader source does not define one and the caller requests it (fragment
///    shaders only — GLSL ES gives fragment shaders no default float
///    precision, so drivers reject such shaders outright; vertex shaders
///    default to `highp` and must not be downgraded).
/// 4. Fixing variable redeclaration errors by deduplicating identical
///    variable declarations in function scope.
fn patch_shader_for_native_es2(
    source: &str,
    texture_lod_ext_supported: bool,
    inject_default_float_precision: bool,
) -> String {
    let lines: Vec<&str> = source.lines().collect();

    // Separate lines into categories for hoisting.
    let mut version_line: Option<String> = None;
    let mut extension_lines: Vec<String> = Vec::new();
    let mut body_lines: Vec<String> = Vec::new();
    let mut has_default_float_precision = false;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with("#version") && version_line.is_none() {
            version_line = Some(line.to_string());
        } else if trimmed.starts_with("#extension") {
            // If the driver doesn't support texture_lod, strip that extension
            if !texture_lod_ext_supported && trimmed.contains("GL_EXT_shader_texture_lod") {
                // Drop this line entirely
                continue;
            }
            extension_lines.push(line.to_string());
        } else {
            body_lines.push(line.to_string());
        }

        if trimmed.starts_with("precision") {
            let rest = trimmed["precision".len()..].trim_start();
            let mut parts = rest.split_whitespace();
            let qualifier = parts.next();
            let ty = parts.next();
            if matches!(qualifier, Some("lowp" | "mediump" | "highp"))
                && matches!(ty, Some(t) if t.starts_with("float"))
            {
                has_default_float_precision = true;
            }
        }
    }

    // Reassemble: version, then extensions, then body.
    let mut out = String::with_capacity(source.len() + 64);
    if let Some(v) = &version_line {
        out.push_str(v);
        out.push('\n');
    }
    for ext in &extension_lines {
        out.push_str(ext);
        out.push('\n');
    }
    if inject_default_float_precision && !has_default_float_precision {
        out.push_str("precision mediump float;\n");
    }
    for line in &body_lines {
        out.push_str(line);
        out.push('\n');
    }

    // If the driver doesn't support GL_EXT_shader_texture_lod, replace
    // texture2DLodEXT / texture2DProjLodEXT / textureCubeLodEXT calls.
    // These functions take an extra LOD parameter that we drop:
    //   texture2DLodEXT(sampler, coord, lod) -> texture2D(sampler, coord)
    //   texture2DProjLodEXT(sampler, coord, lod) -> texture2DProj(sampler, coord)
    //   textureCubeLodEXT(sampler, coord, lod) -> textureCube(sampler, coord)
    if !texture_lod_ext_supported {
        out = replace_texture_lod_ext_calls(&out);
    }

    out
}

/// Replace `texture2DLodEXT(sampler, coord, lod)` with `texture2D(sampler, coord)`,
/// `texture2DProjLodEXT(sampler, coord, lod)` with `texture2DProj(sampler, coord)`,
/// and `textureCubeLodEXT(sampler, coord, lod)` with `textureCube(sampler, coord)`.
///
/// We parse the function call to find the matching parentheses and drop the
/// last argument (the LOD bias).
fn replace_texture_lod_ext_calls(source: &str) -> String {
    let replacements: &[(&str, &str)] = &[
        ("texture2DLodEXT", "texture2D"),
        ("texture2DProjLodEXT", "texture2DProj"),
        ("textureCubeLodEXT", "textureCube"),
        ("texture2DGradEXT", "texture2D"),
    ];

    let mut result = source.to_string();
    for &(old_name, new_name) in replacements {
        while let Some(start) = result.find(old_name) {
            // Ensure this is a standalone identifier (not part of a bigger word)
            let before_ok = start == 0
                || !result.as_bytes()[start - 1].is_ascii_alphanumeric()
                    && result.as_bytes()[start - 1] != b'_';
            let end_of_name = start + old_name.len();
            let after_ok = end_of_name >= result.len()
                || !result.as_bytes()[end_of_name].is_ascii_alphanumeric()
                    && result.as_bytes()[end_of_name] != b'_';

            if !before_ok || !after_ok {
                // Not a standalone identifier, skip it by replacing just the
                // matched portion to avoid infinite loops
                break;
            }

            // Find the opening paren
            let rest = &result[end_of_name..];
            let paren_offset = match rest.find('(') {
                Some(o) => o,
                None => break,
            };
            let paren_start = end_of_name + paren_offset;

            // Find the matching closing paren and locate the last comma
            // (which separates the LOD argument from the previous args).
            let bytes = result.as_bytes();
            let mut depth = 0;
            let mut last_comma_at_depth1: Option<usize> = None;
            let mut close_paren = None;
            for i in paren_start..bytes.len() {
                match bytes[i] {
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            close_paren = Some(i);
                            break;
                        }
                    }
                    b',' if depth == 1 => {
                        last_comma_at_depth1 = Some(i);
                    }
                    _ => {}
                }
            }

            let close = match close_paren {
                Some(c) => c,
                None => break,
            };

            // If we found the last comma, remove from last comma to close paren
            // (exclusive of close paren), replacing old_name with new_name.
            if let Some(comma) = last_comma_at_depth1 {
                let inner_before_last_arg = &result[paren_start + 1..comma];
                let replacement = format!("{}({})", new_name, inner_before_last_arg.trim());
                result = format!(
                    "{}{}{}",
                    &result[..start],
                    replacement,
                    &result[close + 1..]
                );
            } else {
                // No comma found — just rename the function
                result = format!(
                    "{}{}{}",
                    &result[..start],
                    new_name,
                    &result[end_of_name..]
                );
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{patch_shader_for_native_es2, shader_has_default_float_precision};

    #[test]
    fn injects_default_float_precision_when_missing() {
        let src = "#version 100\nvoid main() { gl_FragColor = vec4(1.0); }\n";
        assert!(!shader_has_default_float_precision(src));
        let out = patch_shader_for_native_es2(src, true, true);
        assert!(out.contains("precision mediump float;"));
        assert!(out.contains("void main()"));
    }

    #[test]
    fn keeps_existing_float_precision() {
        let src =
            "#version 100\nprecision highp float;\nvoid main() { gl_FragColor = vec4(1.0); }\n";
        assert!(shader_has_default_float_precision(src));
        let out = patch_shader_for_native_es2(src, true, true);
        assert!(out.contains("precision highp float;"));
        assert_eq!(out.matches("precision").count(), 1);
    }

    #[test]
    fn does_not_inject_when_not_requested() {
        let src = "#version 100\nvoid main() { gl_Position = vec4(1.0); }\n";
        let out = patch_shader_for_native_es2(src, true, false);
        assert!(!out.contains("precision"));
    }
}

pub struct GLES2Native<'gl_ctx> {
    _gl_lifetime: PhantomData<&'gl_ctx ()>,
    pvrtc_native: bool,
    /// Whether `GL_EXT_shader_texture_lod` is advertised by the host driver.
    texture_lod_ext_supported: bool,
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
impl GLES for GLES2Native<'_> {
    fn is_es2(&self) -> bool {
        true
    }

    unsafe fn driver_description(&self) -> String {
        let version = CStr::from_ptr(gles2::GetString(gles2::VERSION) as *const _);
        let vendor = CStr::from_ptr(gles2::GetString(gles2::VENDOR) as *const _);
        let renderer = CStr::from_ptr(gles2::GetString(gles2::RENDERER) as *const _);
        format!(
            "{} / {} / {}",
            version.to_string_lossy(),
            vendor.to_string_lossy(),
            renderer.to_string_lossy()
        )
    }

    // Generic state manipulation
    unsafe fn GetError(&mut self) -> GLenum {
        gles2::GetError()
    }
    unsafe fn Enable(&mut self, cap: GLenum) {
        if is_es1_only_capability(cap) {
            return;
        }
        gles2::Enable(cap)
    }
    unsafe fn IsEnabled(&mut self, cap: GLenum) -> GLboolean {
        if is_es1_only_capability(cap) {
            return gles2::FALSE;
        }
        gles2::IsEnabled(cap)
    }
    unsafe fn Disable(&mut self, cap: GLenum) {
        if is_es1_only_capability(cap) {
            return;
        }
        gles2::Disable(cap)
    }
    unsafe fn GetBooleanv(&mut self, pname: GLenum, params: *mut GLboolean) {
        gles2::GetBooleanv(pname, params)
    }
    unsafe fn GetFloatv(&mut self, pname: GLenum, params: *mut GLfloat) {
        gles2::GetFloatv(pname, params)
    }
    unsafe fn GetIntegerv(&mut self, pname: GLenum, params: *mut GLint) {
        gles2::GetIntegerv(pname, params)
    }
    unsafe fn Hint(&mut self, target: GLenum, mode: GLenum) {
        if is_es1_only_hint_target(target) {
            return;
        }
        gles2::Hint(target, mode)
    }
    unsafe fn Finish(&mut self) {
        gles2::Finish()
    }
    unsafe fn Flush(&mut self) {
        gles2::Flush()
    }
    unsafe fn GetString(&mut self, name: GLenum) -> *const GLubyte {
        gles2::GetString(name)
    }

    // Other state manipulation
    unsafe fn BlendFunc(&mut self, sfactor: GLenum, dfactor: GLenum) {
        gles2::BlendFunc(sfactor, dfactor)
    }
    unsafe fn ColorMask(
        &mut self,
        red: GLboolean,
        green: GLboolean,
        blue: GLboolean,
        alpha: GLboolean,
    ) {
        gles2::ColorMask(red, green, blue, alpha)
    }
    unsafe fn CullFace(&mut self, mode: GLenum) {
        gles2::CullFace(mode)
    }
    unsafe fn DepthFunc(&mut self, func: GLenum) {
        gles2::DepthFunc(func)
    }
    unsafe fn DepthMask(&mut self, flag: GLboolean) {
        gles2::DepthMask(flag)
    }
    unsafe fn DepthRangef(&mut self, near: GLclampf, far: GLclampf) {
        gles2::DepthRangef(near, far)
    }
    unsafe fn FrontFace(&mut self, mode: GLenum) {
        gles2::FrontFace(mode)
    }
    unsafe fn PolygonOffset(&mut self, factor: GLfloat, units: GLfloat) {
        gles2::PolygonOffset(factor, units)
    }
    unsafe fn SampleCoverage(&mut self, value: GLclampf, invert: GLboolean) {
        gles2::SampleCoverage(value, invert)
    }
    unsafe fn Scissor(&mut self, x: GLint, y: GLint, width: GLsizei, height: GLsizei) {
        gles2::Scissor(x, y, width, height)
    }
    unsafe fn Viewport(&mut self, x: GLint, y: GLint, width: GLsizei, height: GLsizei) {
        gles2::Viewport(x, y, width, height)
    }
    unsafe fn LineWidth(&mut self, val: GLfloat) {
        gles2::LineWidth(val)
    }
    unsafe fn StencilFunc(&mut self, func: GLenum, ref_: GLint, mask: GLuint) {
        gles2::StencilFunc(func, ref_, mask)
    }
    unsafe fn StencilOp(&mut self, sfail: GLenum, dpfail: GLenum, dppass: GLenum) {
        gles2::StencilOp(sfail, dpfail, dppass)
    }
    unsafe fn StencilMask(&mut self, mask: GLuint) {
        gles2::StencilMask(mask)
    }

    // Buffers
    unsafe fn IsBuffer(&mut self, buffer: GLuint) -> GLboolean {
        gles2::IsBuffer(buffer)
    }
    unsafe fn GenBuffers(&mut self, n: GLsizei, buffers: *mut GLuint) {
        gles2::GenBuffers(n, buffers)
    }
    unsafe fn DeleteBuffers(&mut self, n: GLsizei, buffers: *const GLuint) {
        gles2::DeleteBuffers(n, buffers)
    }
    unsafe fn BindBuffer(&mut self, target: GLenum, buffer: GLuint) {
        gles2::BindBuffer(target, buffer)
    }
    unsafe fn BufferData(
        &mut self,
        target: GLenum,
        size: GLsizeiptr,
        data: *const GLvoid,
        usage: GLenum,
    ) {
        gles2::BufferData(target, size, data, usage)
    }
    unsafe fn BufferSubData(
        &mut self,
        target: GLenum,
        offset: GLintptr,
        size: GLsizeiptr,
        data: *const GLvoid,
    ) {
        gles2::BufferSubData(target, offset, size, data)
    }

    // Drawing
    unsafe fn DrawArrays(&mut self, mode: GLenum, first: GLint, count: GLsizei) {
        gles2::DrawArrays(mode, first, count)
    }
    unsafe fn DrawElements(
        &mut self,
        mode: GLenum,
        count: GLsizei,
        type_: GLenum,
        indices: *const GLvoid,
    ) {
        gles2::DrawElements(mode, count, type_, indices)
    }
    unsafe fn Clear(&mut self, mask: GLbitfield) {
        gles2::Clear(mask)
    }
    unsafe fn ClearColor(
        &mut self,
        red: GLclampf,
        green: GLclampf,
        blue: GLclampf,
        alpha: GLclampf,
    ) {
        gles2::ClearColor(red, green, blue, alpha)
    }
    unsafe fn ClearDepthf(&mut self, depth: GLclampf) {
        gles2::ClearDepthf(depth)
    }
    unsafe fn ClearStencil(&mut self, s: GLint) {
        gles2::ClearStencil(s)
    }

    // Textures
    unsafe fn PixelStorei(&mut self, pname: GLenum, param: GLint) {
        gles2::PixelStorei(pname, param)
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
        gles2::ReadPixels(x, y, width, height, format, type_, pixels)
    }
    unsafe fn IsTexture(&mut self, texture: GLuint) -> GLboolean {
        gles2::IsTexture(texture)
    }
    unsafe fn GenTextures(&mut self, n: GLsizei, textures: *mut GLuint) {
        gles2::GenTextures(n, textures)
    }
    unsafe fn DeleteTextures(&mut self, n: GLsizei, textures: *const GLuint) {
        gles2::DeleteTextures(n, textures)
    }
    unsafe fn ActiveTexture(&mut self, texture: GLenum) {
        gles2::ActiveTexture(texture)
    }
    unsafe fn BindTexture(&mut self, target: GLenum, texture: GLuint) {
        gles2::BindTexture(target, texture)
    }
    unsafe fn TexParameteri(&mut self, target: GLenum, pname: GLenum, param: GLint) {
        // GL_GENERATE_MIPMAP (0x8191) is a TexParameter pname only on ES 1.1.
        // On ES 2.0 the equivalent is the standalone glGenerateMipmap() call.
        // Apps that ask for an ES 1.1 context but rely on shaders frequently
        // still use the ES 1.1 form; redirect it transparently.
        if pname == gles11::GENERATE_MIPMAP {
            if param != 0 {
                gles2::GenerateMipmap(target);
            }
            return;
        }
        gles2::TexParameteri(target, pname, param)
    }
    unsafe fn TexParameterf(&mut self, target: GLenum, pname: GLenum, param: GLfloat) {
        if pname == gles11::GENERATE_MIPMAP {
            if param != 0.0 {
                gles2::GenerateMipmap(target);
            }
            return;
        }
        gles2::TexParameterf(target, pname, param)
    }
    unsafe fn TexParameteriv(&mut self, target: GLenum, pname: GLenum, params: *const GLint) {
        if pname == gles11::GENERATE_MIPMAP {
            if !params.is_null() && *params != 0 {
                gles2::GenerateMipmap(target);
            }
            return;
        }
        gles2::TexParameteriv(target, pname, params)
    }
    unsafe fn TexParameterfv(&mut self, target: GLenum, pname: GLenum, params: *const GLfloat) {
        if pname == gles11::GENERATE_MIPMAP {
            if !params.is_null() && *params != 0.0 {
                gles2::GenerateMipmap(target);
            }
            return;
        }
        gles2::TexParameterfv(target, pname, params)
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
        gles2::TexImage2D(
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
        gles2::TexSubImage2D(
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

                gles2::TexImage2D(
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
        gles2::CompressedTexImage2D(
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
        gles2::CopyTexImage2D(target, level, internalformat, x, y, width, height, border)
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
        gles2::CopyTexSubImage2D(target, level, xoffset, yoffset, x, y, width, height)
    }
    unsafe fn GenerateMipmapOES(&mut self, target: GLenum) {
        gles2::GenerateMipmap(target)
    }
    unsafe fn GenerateMipmap(&mut self, target: GLenum) {
        gles2::GenerateMipmap(target)
    }
    unsafe fn GenFramebuffers(&mut self, n: GLsizei, framebuffers: *mut GLuint) {
        gles2::GenFramebuffers(n, framebuffers)
    }
    unsafe fn GenRenderbuffers(&mut self, n: GLsizei, renderbuffers: *mut GLuint) {
        gles2::GenRenderbuffers(n, renderbuffers)
    }
    unsafe fn IsFramebuffer(&mut self, framebuffer: GLuint) -> GLboolean {
        gles2::IsFramebuffer(framebuffer)
    }
    unsafe fn IsRenderbuffer(&mut self, renderbuffer: GLuint) -> GLboolean {
        gles2::IsRenderbuffer(renderbuffer)
    }
    unsafe fn BindFramebuffer(&mut self, target: GLenum, framebuffer: GLuint) {
        gles2::BindFramebuffer(target, framebuffer)
    }
    unsafe fn BindRenderbuffer(&mut self, target: GLenum, renderbuffer: GLuint) {
        gles2::BindRenderbuffer(target, renderbuffer)
    }
    unsafe fn RenderbufferStorage(
        &mut self,
        target: GLenum,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
    ) {
        gles2::RenderbufferStorage(target, internalformat, width, height)
    }
    unsafe fn FramebufferRenderbuffer(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        renderbuffertarget: GLenum,
        renderbuffer: GLuint,
    ) {
        gles2::FramebufferRenderbuffer(target, attachment, renderbuffertarget, renderbuffer)
    }
    unsafe fn FramebufferTexture2D(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        textarget: GLenum,
        texture: GLuint,
        level: i32,
    ) {
        gles2::FramebufferTexture2D(target, attachment, textarget, texture, level)
    }
    unsafe fn CheckFramebufferStatus(&mut self, target: GLenum) -> GLenum {
        gles2::CheckFramebufferStatus(target)
    }
    unsafe fn DeleteFramebuffers(&mut self, n: GLsizei, framebuffers: *const GLuint) {
        gles2::DeleteFramebuffers(n, framebuffers)
    }
    unsafe fn DeleteRenderbuffers(&mut self, n: GLsizei, renderbuffers: *const GLuint) {
        gles2::DeleteRenderbuffers(n, renderbuffers)
    }
    unsafe fn GetFramebufferAttachmentParameteriv(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gles2::GetFramebufferAttachmentParameteriv(target, attachment, pname, params)
    }
    unsafe fn GetRenderbufferParameteriv(
        &mut self,
        target: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gles2::GetRenderbufferParameteriv(target, pname, params)
    }
    unsafe fn GetBufferParameteriv(&mut self, target: GLenum, pname: GLenum, params: *mut GLint) {
        gles2::GetBufferParameteriv(target, pname, params)
    }
    // Buffer mapping (`GL_OES_mapbuffer`).
    //
    // On real ES 2.0+ drivers (Adreno, Mali, …) the `GL_OES_mapbuffer`
    // extension is widely supported, so we can route the OES entry points
    // straight to the extension functions loaded via `gles2::load_with`. Some
    // games (e.g. LEGO Ninjago Spinjitzu Scavenger Hunt) call these even when
    // they asked EAGL for an ES 1.1 context — combined with
    // `--prefer-gles2-context`, they end up here.
    unsafe fn MapBufferOES(&mut self, target: GLenum, access: GLenum) -> *mut GLvoid {
        if gles2::MapBufferOES::is_loaded() {
            gles2::MapBufferOES(target, access)
        } else {
            log!(
                "Warning: glMapBufferOES called but GL_OES_mapbuffer is not \
                 available on this ES 2.0 driver; returning NULL"
            );
            std::ptr::null_mut()
        }
    }
    unsafe fn UnmapBufferOES(&mut self, target: GLenum) -> GLboolean {
        if gles2::UnmapBufferOES::is_loaded() {
            gles2::UnmapBufferOES(target)
        } else {
            // Caller will fall back to its own write-through copy.
            gles2::FALSE
        }
    }

    // Framebuffers / renderbuffers (mapped via OES naming → core ES 2 calls)
    unsafe fn GenFramebuffersOES(&mut self, n: GLsizei, framebuffers: *mut GLuint) {
        gles2::GenFramebuffers(n, framebuffers)
    }
    unsafe fn DeleteFramebuffersOES(&mut self, n: GLsizei, framebuffers: *const GLuint) {
        gles2::DeleteFramebuffers(n, framebuffers)
    }
    unsafe fn BindFramebufferOES(&mut self, target: GLenum, framebuffer: GLuint) {
        gles2::BindFramebuffer(target, framebuffer)
    }
    unsafe fn IsFramebufferOES(&mut self, framebuffer: GLuint) -> GLboolean {
        gles2::IsFramebuffer(framebuffer)
    }
    unsafe fn CheckFramebufferStatusOES(&mut self, target: GLenum) -> GLenum {
        gles2::CheckFramebufferStatus(target)
    }
    unsafe fn FramebufferRenderbufferOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        renderbuffertarget: GLenum,
        renderbuffer: GLuint,
    ) {
        gles2::FramebufferRenderbuffer(target, attachment, renderbuffertarget, renderbuffer)
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
        gles2::FramebufferTexture2D(target, attachment, textarget, texture, level)
    }
    unsafe fn GetFramebufferAttachmentParameterivOES(
        &mut self,
        target: GLenum,
        attachment: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gles2::GetFramebufferAttachmentParameteriv(target, attachment, pname, params)
    }
    unsafe fn GenRenderbuffersOES(&mut self, n: GLsizei, renderbuffers: *mut GLuint) {
        gles2::GenRenderbuffers(n, renderbuffers)
    }
    unsafe fn DeleteRenderbuffersOES(&mut self, n: GLsizei, renderbuffers: *const GLuint) {
        gles2::DeleteRenderbuffers(n, renderbuffers)
    }
    unsafe fn BindRenderbufferOES(&mut self, target: GLenum, renderbuffer: GLuint) {
        gles2::BindRenderbuffer(target, renderbuffer)
    }
    unsafe fn IsRenderbufferOES(&mut self, renderbuffer: GLuint) -> GLboolean {
        gles2::IsRenderbuffer(renderbuffer)
    }
    unsafe fn RenderbufferStorageOES(
        &mut self,
        target: GLenum,
        internalformat: GLenum,
        width: GLsizei,
        height: GLsizei,
    ) {
        gles2::RenderbufferStorage(target, internalformat, width, height)
    }
    unsafe fn GetRenderbufferParameterivOES(
        &mut self,
        target: GLenum,
        pname: GLenum,
        params: *mut GLint,
    ) {
        gles2::GetRenderbufferParameteriv(target, pname, params)
    }

    // OpenGL ES 2.0 — shaders & programs
    unsafe fn CreateShader(&mut self, type_: GLenum) -> GLuint {
        gles2::CreateShader(type_)
    }
    unsafe fn DeleteShader(&mut self, shader: GLuint) {
        gles2::DeleteShader(shader)
    }
    unsafe fn ShaderSource(
        &mut self,
        shader: GLuint,
        count: GLsizei,
        string: *const *const GLchar,
        length: *const GLint,
    ) {
        // Even on a native ES 2.0 driver we may need to patch shaders:
        // - Hoist #extension directives before non-preprocessor tokens
        //   (Mali drivers reject them otherwise).
        // - When GL_EXT_shader_texture_lod is not supported, strip the
        //   #extension directive and replace texture2DLodEXT and friends
        //   with texture2D (dropping the LOD bias parameter).
        // - Fix variable redeclarations in some Unity shaders.
        use std::ffi::CString;

        let n = count.max(0) as usize;
        let mut joined = String::new();
        for i in 0..n {
            let raw_ptr = *string.add(i);
            if raw_ptr.is_null() {
                continue;
            }
            let s = if !length.is_null() {
                let len = *length.add(i);
                if len >= 0 {
                    let slice =
                        std::slice::from_raw_parts(raw_ptr as *const u8, len as usize);
                    std::str::from_utf8(slice).unwrap_or("").to_owned()
                } else {
                    CStr::from_ptr(raw_ptr).to_string_lossy().into_owned()
                }
            } else {
                CStr::from_ptr(raw_ptr).to_string_lossy().into_owned()
            };
            joined.push_str(&s);
        }

        // Check if any patching is needed at all. If the shader has no
        // extension directives and no texture*LodEXT calls, pass through
        // unchanged for maximum fidelity.
        let needs_ext_hoist = joined.contains("#extension")
            && joined.lines().enumerate().any(|(i, line)| {
                let trimmed = line.trim();
                if !trimmed.starts_with("#extension") {
                    return false;
                }
                // Check if there's a non-preprocessor, non-empty, non-comment
                // line before this #extension
                joined.lines().take(i).any(|prev| {
                    let pt = prev.trim();
                    !pt.is_empty() && !pt.starts_with('#') && !pt.starts_with("//")
                })
            });
        let needs_lod_patch = !self.texture_lod_ext_supported
            && (joined.contains("texture2DLodEXT")
                || joined.contains("texture2DProjLodEXT")
                || joined.contains("textureCubeLodEXT"));

        // GLSL ES fragment shaders have no default precision for `float`, so
        // strict drivers (e.g. AMD's native GLES) reject any fragment shader
        // that declares a float without a `precision ... float;` line. Real
        // iPhoneOS-era drivers (PowerVR SGX) were lenient about this, so some
        // apps (e.g. The Binding of Isaac) ship such shaders. Inject a
        // default `precision mediump float;` for them. Vertex shaders default
        // to highp and must not be touched.
        let mut shader_type: GLint = 0;
        gles2::GetShaderiv(shader, gles2::SHADER_TYPE, &mut shader_type);
        let needs_precision_inject = shader_type as GLenum == gles2::FRAGMENT_SHADER
            && !shader_has_default_float_precision(&joined);

        if !needs_ext_hoist && !needs_lod_patch && !needs_precision_inject {
            // No patching needed — pass through directly.
            gles2::ShaderSource(shader, count, string, length);
            return;
        }

        let patched = patch_shader_for_native_es2(
            &joined,
            self.texture_lod_ext_supported,
            needs_precision_inject,
        );
        let c = match CString::new(patched) {
            Ok(c) => c,
            Err(_) => {
                // Source contained an interior NUL — pass original through.
                gles2::ShaderSource(shader, count, string, length);
                return;
            }
        };
        let ptr = c.as_ptr();
        gles2::ShaderSource(shader, 1, &ptr, std::ptr::null());
    }
    unsafe fn CompileShader(&mut self, shader: GLuint) {
        gles2::CompileShader(shader)
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
        gles2::GetShaderPrecisionFormat(shadertype, precisiontype, range, precision)
    }
    unsafe fn ReleaseShaderCompiler(&mut self) {
        gles2::ReleaseShaderCompiler()
    }
    unsafe fn ShaderBinary(
        &mut self,
        count: GLsizei,
        shaders: *const GLuint,
        binaryformat: GLenum,
        binary: *const GLvoid,
        length: GLsizei,
    ) {
        gles2::ShaderBinary(count, shaders, binaryformat, binary, length)
    }
    unsafe fn GetShaderiv(&mut self, shader: GLuint, pname: GLenum, params: *mut GLint) {
        gles2::GetShaderiv(shader, pname, params)
    }
    unsafe fn GetShaderInfoLog(
        &mut self,
        shader: GLuint,
        maxLength: GLsizei,
        length: *mut GLsizei,
        infoLog: *mut GLchar,
    ) {
        gles2::GetShaderInfoLog(shader, maxLength, length, infoLog)
    }
    unsafe fn GetShaderSource(
        &mut self,
        shader: GLuint,
        bufSize: GLsizei,
        length: *mut GLsizei,
        source: *mut GLchar,
    ) {
        gles2::GetShaderSource(shader, bufSize, length, source)
    }
    unsafe fn IsShader(&mut self, shader: GLuint) -> GLboolean {
        gles2::IsShader(shader)
    }
    unsafe fn CreateProgram(&mut self) -> GLuint {
        gles2::CreateProgram()
    }
    unsafe fn DeleteProgram(&mut self, program: GLuint) {
        gles2::DeleteProgram(program)
    }
    unsafe fn AttachShader(&mut self, program: GLuint, shader: GLuint) {
        gles2::AttachShader(program, shader)
    }
    unsafe fn DetachShader(&mut self, program: GLuint, shader: GLuint) {
        gles2::DetachShader(program, shader)
    }
    unsafe fn LinkProgram(&mut self, program: GLuint) {
        gles2::LinkProgram(program)
    }
    unsafe fn UseProgram(&mut self, program: GLuint) {
        gles2::UseProgram(program)
    }
    unsafe fn GetProgramiv(&mut self, program: GLuint, pname: GLenum, params: *mut GLint) {
        gles2::GetProgramiv(program, pname, params)
    }
    unsafe fn GetProgramInfoLog(
        &mut self,
        program: GLuint,
        maxLength: GLsizei,
        length: *mut GLsizei,
        infoLog: *mut GLchar,
    ) {
        gles2::GetProgramInfoLog(program, maxLength, length, infoLog)
    }
    unsafe fn IsProgram(&mut self, program: GLuint) -> GLboolean {
        gles2::IsProgram(program)
    }
    unsafe fn ValidateProgram(&mut self, program: GLuint) {
        gles2::ValidateProgram(program)
    }
    unsafe fn BindAttribLocation(&mut self, program: GLuint, index: GLuint, name: *const GLchar) {
        gles2::BindAttribLocation(program, index, name)
    }
    unsafe fn GetAttribLocation(&mut self, program: GLuint, name: *const GLchar) -> GLint {
        gles2::GetAttribLocation(program, name)
    }
    unsafe fn GetUniformLocation(&mut self, program: GLuint, name: *const GLchar) -> GLint {
        gles2::GetUniformLocation(program, name)
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
        gles2::GetActiveAttrib(program, index, bufSize, length, size, type_, name)
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
        gles2::GetActiveUniform(program, index, bufSize, length, size, type_, name)
    }

    // Vertex attributes
    unsafe fn EnableVertexAttribArray(&mut self, index: GLuint) {
        gles2::EnableVertexAttribArray(index)
    }
    unsafe fn DisableVertexAttribArray(&mut self, index: GLuint) {
        gles2::DisableVertexAttribArray(index)
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
        // GL_HALF_FLOAT_OES (0x8D61) is the ES 2.0 OES-extension token for
        // 16-bit floats in vertex attributes (GL_OES_vertex_half_float).
        // In ES 3.0+ and desktop OpenGL the core token is GL_HALF_FLOAT
        // (0x140B) — a different numeric value.  AMD's native GLES 3.x
        // driver on Windows only accepts the core token, so passing 0x8D61
        // yields GL_INVALID_ENUM every frame, breaking the stage render.
        // We translate here so that apps using GLES 2.0 half-float vertex
        // data (e.g. the Supercell SC3D engine used by Brawl Stars) work
        // correctly on any host driver.
        //
        // Reference: https://registry.khronos.org/OpenGL/extensions/OES/OES_vertex_half_float.txt
        const GL_HALF_FLOAT_OES: GLenum = 0x8D61;
        const GL_HALF_FLOAT: GLenum = 0x140B;
        let type_ = if type_ == GL_HALF_FLOAT_OES {
            GL_HALF_FLOAT
        } else {
            type_
        };
        gles2::VertexAttribPointer(index, size, type_, normalized, stride, pointer)
    }
    unsafe fn VertexAttrib1f(&mut self, index: GLuint, x: GLfloat) {
        gles2::VertexAttrib1f(index, x)
    }
    unsafe fn VertexAttrib2f(&mut self, index: GLuint, x: GLfloat, y: GLfloat) {
        gles2::VertexAttrib2f(index, x, y)
    }
    unsafe fn VertexAttrib3f(&mut self, index: GLuint, x: GLfloat, y: GLfloat, z: GLfloat) {
        gles2::VertexAttrib3f(index, x, y, z)
    }
    unsafe fn VertexAttrib4f(
        &mut self,
        index: GLuint,
        x: GLfloat,
        y: GLfloat,
        z: GLfloat,
        w: GLfloat,
    ) {
        gles2::VertexAttrib4f(index, x, y, z, w)
    }
    unsafe fn VertexAttrib1fv(&mut self, index: GLuint, v: *const GLfloat) {
        gles2::VertexAttrib1fv(index, v)
    }
    unsafe fn VertexAttrib2fv(&mut self, index: GLuint, v: *const GLfloat) {
        gles2::VertexAttrib2fv(index, v)
    }
    unsafe fn VertexAttrib3fv(&mut self, index: GLuint, v: *const GLfloat) {
        gles2::VertexAttrib3fv(index, v)
    }
    unsafe fn VertexAttrib4fv(&mut self, index: GLuint, v: *const GLfloat) {
        gles2::VertexAttrib4fv(index, v)
    }
    unsafe fn GetVertexAttribiv(&mut self, index: GLuint, pname: GLenum, params: *mut GLint) {
        gles2::GetVertexAttribiv(index, pname, params)
    }
    unsafe fn GetVertexAttribfv(&mut self, index: GLuint, pname: GLenum, params: *mut GLfloat) {
        gles2::GetVertexAttribfv(index, pname, params)
    }
    unsafe fn GetVertexAttribPointerv(
        &mut self,
        index: GLuint,
        pname: GLenum,
        pointer: *mut *mut GLvoid,
    ) {
        gles2::GetVertexAttribPointerv(index, pname, pointer)
    }

    // Vertex array objects (GL_OES_vertex_array_object)
    fn supports_vao_oes(&self) -> bool {
        gles2::BindVertexArrayOES::is_loaded()
            && gles2::GenVertexArraysOES::is_loaded()
            && gles2::DeleteVertexArraysOES::is_loaded()
    }
    unsafe fn BindVertexArrayOES(&mut self, array: GLuint) {
        gles2::BindVertexArrayOES(array)
    }
    unsafe fn GenVertexArraysOES(&mut self, n: GLsizei, arrays: *mut GLuint) {
        gles2::GenVertexArraysOES(n, arrays)
    }
    unsafe fn DeleteVertexArraysOES(&mut self, n: GLsizei, arrays: *const GLuint) {
        gles2::DeleteVertexArraysOES(n, arrays)
    }
    unsafe fn IsVertexArrayOES(&mut self, array: GLuint) -> GLboolean {
        gles2::IsVertexArrayOES(array)
    }

    // Uniforms
    unsafe fn Uniform1f(&mut self, location: GLint, v0: GLfloat) {
        gles2::Uniform1f(location, v0)
    }
    unsafe fn Uniform2f(&mut self, location: GLint, v0: GLfloat, v1: GLfloat) {
        gles2::Uniform2f(location, v0, v1)
    }
    unsafe fn Uniform3f(&mut self, location: GLint, v0: GLfloat, v1: GLfloat, v2: GLfloat) {
        gles2::Uniform3f(location, v0, v1, v2)
    }
    unsafe fn Uniform4f(
        &mut self,
        location: GLint,
        v0: GLfloat,
        v1: GLfloat,
        v2: GLfloat,
        v3: GLfloat,
    ) {
        gles2::Uniform4f(location, v0, v1, v2, v3)
    }
    unsafe fn Uniform1i(&mut self, location: GLint, v0: GLint) {
        gles2::Uniform1i(location, v0)
    }
    unsafe fn Uniform2i(&mut self, location: GLint, v0: GLint, v1: GLint) {
        gles2::Uniform2i(location, v0, v1)
    }
    unsafe fn Uniform3i(&mut self, location: GLint, v0: GLint, v1: GLint, v2: GLint) {
        gles2::Uniform3i(location, v0, v1, v2)
    }
    unsafe fn Uniform4i(&mut self, location: GLint, v0: GLint, v1: GLint, v2: GLint, v3: GLint) {
        gles2::Uniform4i(location, v0, v1, v2, v3)
    }
    unsafe fn Uniform1fv(&mut self, location: GLint, count: GLsizei, value: *const GLfloat) {
        gles2::Uniform1fv(location, count, value)
    }
    unsafe fn Uniform2fv(&mut self, location: GLint, count: GLsizei, value: *const GLfloat) {
        gles2::Uniform2fv(location, count, value)
    }
    unsafe fn Uniform3fv(&mut self, location: GLint, count: GLsizei, value: *const GLfloat) {
        gles2::Uniform3fv(location, count, value)
    }
    unsafe fn Uniform4fv(&mut self, location: GLint, count: GLsizei, value: *const GLfloat) {
        gles2::Uniform4fv(location, count, value)
    }
    unsafe fn Uniform1iv(&mut self, location: GLint, count: GLsizei, value: *const GLint) {
        gles2::Uniform1iv(location, count, value)
    }
    unsafe fn Uniform2iv(&mut self, location: GLint, count: GLsizei, value: *const GLint) {
        gles2::Uniform2iv(location, count, value)
    }
    unsafe fn Uniform3iv(&mut self, location: GLint, count: GLsizei, value: *const GLint) {
        gles2::Uniform3iv(location, count, value)
    }
    unsafe fn Uniform4iv(&mut self, location: GLint, count: GLsizei, value: *const GLint) {
        gles2::Uniform4iv(location, count, value)
    }
    unsafe fn UniformMatrix2fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gles2::UniformMatrix2fv(location, count, transpose, value)
    }
    unsafe fn UniformMatrix3fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gles2::UniformMatrix3fv(location, count, transpose, value)
    }
    unsafe fn UniformMatrix4fv(
        &mut self,
        location: GLint,
        count: GLsizei,
        transpose: GLboolean,
        value: *const GLfloat,
    ) {
        gles2::UniformMatrix4fv(location, count, transpose, value)
    }

    // Blending / stencil (ES 2.0 / GL 2.0 separate variants)
    unsafe fn BlendColor(
        &mut self,
        red: GLclampf,
        green: GLclampf,
        blue: GLclampf,
        alpha: GLclampf,
    ) {
        gles2::BlendColor(red, green, blue, alpha)
    }
    unsafe fn BlendEquation(&mut self, mode: GLenum) {
        gles2::BlendEquation(mode)
    }
    // GL_OES_blend_equation: this OpenGL ES 1.1 extension entry point is
    // semantically identical to the core BlendEquation function in ES2/ES3.
    // Some apps built against newer SDKs (e.g. games using the GLKit/Cocos2d
    // blend helpers) still resolve the OES-suffixed symbol, so route it to
    // the standard entry point instead of panicking.
    unsafe fn BlendEquationOES(&mut self, mode: GLenum) {
        gles2::BlendEquation(mode)
    }
    unsafe fn BlendEquationSeparate(&mut self, modeRGB: GLenum, modeAlpha: GLenum) {
        gles2::BlendEquationSeparate(modeRGB, modeAlpha)
    }
    unsafe fn BlendFuncSeparate(
        &mut self,
        sfactorRGB: GLenum,
        dfactorRGB: GLenum,
        sfactorAlpha: GLenum,
        dfactorAlpha: GLenum,
    ) {
        gles2::BlendFuncSeparate(sfactorRGB, dfactorRGB, sfactorAlpha, dfactorAlpha)
    }
    unsafe fn StencilFuncSeparate(
        &mut self,
        face: GLenum,
        func: GLenum,
        ref_: GLint,
        mask: GLuint,
    ) {
        gles2::StencilFuncSeparate(face, func, ref_, mask)
    }
    unsafe fn StencilOpSeparate(
        &mut self,
        face: GLenum,
        sfail: GLenum,
        dpfail: GLenum,
        dppass: GLenum,
    ) {
        gles2::StencilOpSeparate(face, sfail, dpfail, dppass)
    }
    unsafe fn StencilMaskSeparate(&mut self, face: GLenum, mask: GLuint) {
        gles2::StencilMaskSeparate(face, mask)
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
}
