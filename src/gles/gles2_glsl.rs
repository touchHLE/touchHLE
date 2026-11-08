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
    let mut out = String::with_capacity(source.len() + 32);
    let mut emitted_version = false;

    for raw_line in source.lines() {
        let trimmed = raw_line.trim_start();

        if !emitted_version {
            if trimmed.starts_with("#version") {
                out.push_str("#version 120\n");
                emitted_version = true;
                continue;
            } else if !trimmed.is_empty() && !trimmed.starts_with("//") {
                out.push_str("#version 120\n");
                emitted_version = true;
            }
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
        out.push_str(&stripped);
        out.push('\n');
    }

    if !emitted_version {
        let mut prepended = String::from("#version 120\n");
        prepended.push_str(&out);
        return prepended;
    }
    out
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
        let src = "precision mediump float;\nprecision highp int;\nvoid main(){}\n";
        let out = translate_glsl_es_to_120(src);
        assert!(!out.contains("precision mediump"));
        assert!(!out.contains("precision highp"));
    }

    #[test]
    fn strips_inline_qualifiers() {
        let src = "varying lowp vec4 DestinationColor;\n";
        let out = translate_glsl_es_to_120(src);
        assert!(out.contains("varying vec4 DestinationColor;"));
        assert!(!out.contains("lowp"));
    }

    #[test]
    fn preserves_non_qualifier_identifiers() {
        let src = "uniform float highpassCutoff;\n";
        let out = translate_glsl_es_to_120(src);
        assert!(out.contains("highpassCutoff"));
    }
}
