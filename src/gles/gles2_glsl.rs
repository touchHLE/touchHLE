/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! GLSL ES 1.00 → desktop GLSL 1.20 source translator.
//!
//! touchHLE's OpenGL ES 2.0 layer is implemented on top of OpenGL 2.1, which
//! does not understand the GLSL ES dialect that ES 2.0 apps ship. The
//! translation is intentionally minimal — just enough to make the test corpus
//! we have at hand compile:
//!
//! 1. Insert/replace a `#version 120` directive at the top.
//! 2. Drop top-level precision declarations (`precision lowp float;` etc.).
//! 3. Strip inline precision qualifiers (`lowp`/`mediump`/`highp`) from the
//!    rest of the source.
//!
//! This is *not* a full GLSL translator. Apps that use ES-only features (like
//! `gl_FragColor` is fine; built-in attribute names; integer textures; …) are
//! not handled. We can extend this when needed.

/// Translate a GLSL ES 1.00 shader source to GLSL 1.20.
pub fn translate_glsl_es_to_120(source: &str) -> String {
    translate_glsl_es_with_version(source, "#version 120\n")
}

/// Translate a GLSL ES 3.00 shader source to desktop GLSL 3.30 Core.
///
/// GLSL ES 3.00 and desktop GLSL 3.30 are syntactically very similar (both
/// have `in`/`out`/`uniform`, named uniform blocks, integer textures, etc.).
/// The differences this translator handles:
///
/// 1. Rewrite the `#version 300 es` directive to `#version 330 core`.
/// 2. Drop standalone `precision <qualifier> <type>;` declarations.
/// 3. Strip inline `lowp`/`mediump`/`highp` qualifiers.
///
/// Things this translator does **not** handle — pre-3.20 desktop GLSL also
/// disallows them but they require non-trivial AST work and we haven't seen
/// guest shaders that use them yet:
///
/// - Implicit conversions from `int` to `uint` that ES allows but desktop
///   doesn't (rare in hand-written shaders).
/// - The ES-only built-in `gl_FragData[]` (legacy ES 2 fallback; ES 3 apps
///   use named `out` variables).
///
/// When a guest app trips one of these, extend this function rather than
/// patching the guest shader source.
pub fn translate_glsl_es_300_to_330(source: &str) -> String {
    translate_glsl_es_with_version(source, "#version 330 core\n")
}

fn translate_glsl_es_with_version(source: &str, version_directive: &'static str) -> String {
    let mut out = String::with_capacity(source.len() + 32);
    let mut emitted_version = false;

    // Collect #extension directives to hoist them right after #version.
    // Desktop GLSL requires #extension before any non-preprocessor tokens.
    let mut extension_lines: Vec<String> = Vec::new();
    let mut body_lines: Vec<String> = Vec::new();

    for raw_line in source.lines() {
        let trimmed = raw_line.trim_start();

        if !emitted_version {
            if trimmed.starts_with("#version") {
                emitted_version = true;
                continue; // We'll emit our own version directive
            } else if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with('#') {
                emitted_version = true;
            }
        }

        // Collect #extension directives separately so we can hoist them.
        if trimmed.starts_with("#extension") {
            // Strip GL_EXT_shader_texture_lod extension — desktop GLSL 1.20/3.30
            // doesn't have this extension (texture2DLod is built-in in 1.30+).
            if trimmed.contains("GL_EXT_shader_texture_lod") {
                continue;
            }
            extension_lines.push(raw_line.to_string());
            continue;
        }

        // Drop standalone "precision <qual> <type>;" lines.
        if trimmed.starts_with("precision") {
            let rest = trimmed["precision".len()..].trim_start();
            if rest.starts_with("lowp") || rest.starts_with("mediump") || rest.starts_with("highp")
            {
                continue;
            }
        }

        let stripped = strip_precision_qualifiers(raw_line);
let stripped = strip_half_types(&stripped);
        body_lines.push(stripped);
    }

    // Emit version directive first.
    out.push_str(version_directive);

    // Emit hoisted #extension lines right after version.
    for ext in &extension_lines {
        out.push_str(ext);
        out.push('\n');
    }

    // Emit body.
    for line in &body_lines {
        out.push_str(line);
        out.push('\n');
    }

    // Replace texture*LodEXT calls with their desktop equivalents.
    // In GLSL 1.20 we have texture2DLod as a built-in (from GL_ARB_shader_texture_lod
    // which is required by GL 2.1). In GLSL 3.30 we have textureLod.
    if version_directive.contains("120") {
        out = replace_texture_lod_ext_desktop_120(&out);
    } else if version_directive.contains("330") {
        out = replace_texture_lod_ext_desktop_330(&out);
    }

    out
}

/// In desktop GLSL 1.20, `texture2DLod` is available as a built-in (via
/// GL_ARB_shader_texture_lod which is part of OpenGL 2.1 core). Replace the
/// EXT-suffixed names with the unsuffixed equivalents.
fn replace_texture_lod_ext_desktop_120(source: &str) -> String {
    source
        .replace("texture2DLodEXT", "texture2DLod")
        .replace("texture2DProjLodEXT", "texture2DProjLod")
        .replace("textureCubeLodEXT", "textureCubeLod")
        .replace("texture2DGradEXT", "texture2DGrad")
}

/// In desktop GLSL 3.30, the generic `textureLod` function replaces all
/// type-specific LOD sampling functions.
fn replace_texture_lod_ext_desktop_330(source: &str) -> String {
    source
        .replace("texture2DLodEXT", "textureLod")
        .replace("texture2DProjLodEXT", "textureProjLod")
        .replace("textureCubeLodEXT", "textureLod")
        .replace("texture2DGradEXT", "textureGrad")
}

/// Strip occurrences of `lowp`, `mediump`, and `highp` from a line, while
/// preserving identifiers that merely contain those substrings.
fn strip_precision_qualifiers(line: &str) -> String {
    const QUALIFIERS: &[&str] = &["lowp", "mediump", "highp"];

    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        let is_word_start = c.is_ascii_alphabetic() || c == b'_';
        if is_word_start && (i == 0 || !is_ident_char(bytes[i - 1])) {
            let mut matched = None;
            for q in QUALIFIERS {
                let qb = q.as_bytes();
                if i + qb.len() <= bytes.len()
                    && &bytes[i..i + qb.len()] == qb
                    && (i + qb.len() == bytes.len() || !is_ident_char(bytes[i + qb.len()]))
                {
                    matched = Some(qb.len());
                    break;
                }
            }
            if let Some(qlen) = matched {
                // Skip the qualifier and any following whitespace, but keep at
                // least one space if there was one before, to preserve
                // separation between tokens.
                let pre_was_space = !out.is_empty() && out.as_bytes()[out.len() - 1] == b' ';
                let mut j = i + qlen;
                while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                if !pre_was_space && j < bytes.len() {
                    out.push(' ');
                }
                i = j;
                continue;
            }
        }
        out.push(c as char);
        i += 1;
    }
    out
}
fn strip_half_types(line: &str) -> String {
    line.replace("f16mat4", "mat4")
        .replace("f16mat3", "mat3")
        .replace("f16mat2", "mat2")
        .replace("f16vec4", "vec4")
        .replace("f16vec3", "vec3")
        .replace("f16vec2", "vec2")
        .replace("float16_t", "float")
}
fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_version_directive() {
        let src = "#version 100\nvoid main() {}\n";
        let out = translate_glsl_es_to_120(src);
        assert!(out.starts_with("#version 120\n"));
        assert!(out.contains("void main()"));
    }

    #[test]
    fn strips_precision_lines() {
        let src = "#version 100\nprecision mediump float;\nprecision highp int;\nvoid main(){}\n";
        let out = translate_glsl_es_to_120(src);
        assert!(!out.contains("precision mediump"));
        assert!(!out.contains("precision highp"));
    }

    #[test]
    fn strips_inline_qualifiers() {
        let src = "#version 100\nvarying lowp vec4 DestinationColor;\n";
        let out = translate_glsl_es_to_120(src);
        assert!(out.contains("varying vec4 DestinationColor;"));
        assert!(!out.contains("lowp"));
    }

    #[test]
    fn preserves_non_qualifier_identifiers() {
        let src = "#version 100\nuniform float highpassCutoff;\n";
        let out = translate_glsl_es_to_120(src);
        assert!(out.contains("highpassCutoff"));
    }

    #[test]
    fn hoists_extension_directives() {
        let src = "#version 100\nvoid foo() {}\n#extension GL_OES_standard_derivatives : enable\nvoid main(){}\n";
        let out = translate_glsl_es_to_120(src);
        // Extension should come right after #version, before any code
        let version_pos = out.find("#version 120").unwrap();
        let ext_pos = out.find("#extension GL_OES_standard_derivatives").unwrap();
        let foo_pos = out.find("void foo()").unwrap();
        assert!(ext_pos < foo_pos);
        assert!(ext_pos > version_pos);
    }

    #[test]
    fn strips_texture_lod_ext_extension_and_replaces_calls() {
        let src = "#version 100\n#extension GL_EXT_shader_texture_lod : enable\nvoid main(){ gl_FragColor = texture2DLodEXT(tex, uv, 0.0); }\n";
        let out = translate_glsl_es_to_120(src);
        assert!(!out.contains("GL_EXT_shader_texture_lod"));
        assert!(!out.contains("texture2DLodEXT"));
        assert!(out.contains("texture2DLod"));
    }
}
