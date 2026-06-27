/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Minimal Arabic text shaping and bidirectional reordering.
//!
//! The text layout in [`super`] relies on `rusttype`, which lays glyphs out in
//! the logical (memory) order of the string, left-to-right. That is wrong for
//! Arabic in two ways:
//!
//!  * Arabic is written right-to-left, so without reordering the letters of a
//!    word appear reversed ("backwards").
//!  * Arabic letters are cursive: each letter takes a different shape depending
//!    on whether it joins to its neighbours. `rusttype` doesn't apply the
//!    font's `GSUB` joining rules, so feeding it the base letters yields
//!    disconnected, isolated forms.
//!
//! This module pre-processes a single line of text before it is handed to
//! `rusttype::Font::layout`, converting Arabic letters to the appropriate
//! Unicode Presentation Forms (the bundled Noto Sans Arabic font maps these in
//! its `cmap`) and reordering the line into visual order using a simplified
//! version of the Unicode Bidirectional Algorithm (UAX #9). It is intentionally
//! self-contained (no new dependencies) and only covers what the emulated apps
//! realistically need; full BiDi/OpenType shaping would mean pulling in a much
//! larger stack such as `cosmic-text`.

use std::borrow::Cow;

/// Transform one logical-order line of text into the visual, left-to-right
/// order in which its glyphs should be drawn.
///
/// Lines that contain no Arabic are returned untouched (and borrowed), so the
/// behaviour of non-Arabic text is exactly as before.
pub fn shape_line_for_display(line: &str) -> Cow<'_, str> {
    if !line.chars().any(|c| is_arabic(c as u32)) {
        return Cow::Borrowed(line);
    }

    let logical: Vec<char> = line.chars().collect();
    let shaped = shape(&logical);
    let visual = reorder_visual(&shaped);
    Cow::Owned(visual.into_iter().collect())
}

/// Returns `true` for codepoints in the Arabic Unicode blocks (used to decide
/// whether a line needs the full shaping pipeline at all).
fn is_arabic(c: u32) -> bool {
    (0x0600..=0x06FF).contains(&c)
        || (0x0750..=0x077F).contains(&c)
        || (0x08A0..=0x08FF).contains(&c)
        || (0xFB50..=0xFDFF).contains(&c)
        || (0xFE70..=0xFEFF).contains(&c)
}

// =============================================================================
// MARK: - Contextual shaping (joining forms)
// =============================================================================

/// Arabic joining type of a character (a subset of the Unicode
/// `Joining_Type` property, see `ArabicShaping.txt`).
#[derive(Copy, Clone, PartialEq, Eq)]
enum Join {
    /// Non-joining (e.g. isolated letters, spaces, Latin).
    NonJoining,
    /// Right-joining (joins only to a preceding letter; e.g. alef, dal, reh).
    Right,
    /// Dual-joining (joins on both sides; e.g. beh, seen, lam).
    Dual,
    /// Join-causing (tatweel, ZWJ): joins but has no shape of its own.
    Causing,
    /// Transparent (combining marks): ignored when determining neighbours.
    Transparent,
}

fn joining_type(c: char) -> Join {
    use Join::*;
    match c as u32 {
        0x0621 => NonJoining,                        // hamza
        0x0622 | 0x0623 | 0x0624 | 0x0625 => Right,  // alef variants, waw hamza
        0x0626 => Dual,                              // yeh with hamza
        0x0627 => Right,                             // alef
        0x0628 => Dual,                              // beh
        0x0629 => Right,                             // teh marbuta
        0x062A..=0x062E => Dual,                     // teh, theh, jeem, hah, khah
        0x062F | 0x0630 | 0x0631 | 0x0632 => Right,  // dal, thal, reh, zain
        0x0633..=0x063A => Dual,                     // seen..ghain
        0x0640 => Causing,                           // tatweel (kashida)
        0x0641..=0x0647 => Dual,                     // feh, qaf, kaf, lam, meem, noon, heh
        0x0648 => Right,                             // waw
        0x0649 => Dual,                              // alef maksura
        0x064A => Dual,                              // yeh
        // Transparent: combining marks / harakat.
        0x0610..=0x061A
        | 0x064B..=0x065F
        | 0x0670
        | 0x06D6..=0x06DC
        | 0x06DF..=0x06E4
        | 0x06E7
        | 0x06E8
        | 0x06EA..=0x06ED => Transparent,
        0x200D => Causing,    // ZERO WIDTH JOINER
        0x200C => NonJoining, // ZERO WIDTH NON-JOINER
        _ => NonJoining,
    }
}

/// The four contextual forms a cursive letter can take.
#[derive(Copy, Clone)]
enum Form {
    Isolated,
    Final,
    Initial,
    Medial,
}

/// Presentation-form codepoints `[isolated, final, initial, medial]` for an
/// Arabic letter, or `None` if the character has no contextual forms. A `0`
/// entry means that form does not exist for this letter (right-joining letters
/// only have isolated and final forms).
fn presentation_forms(c: char) -> Option<[u32; 4]> {
    Some(match c as u32 {
        0x0621 => [0xFE80, 0, 0, 0],
        0x0622 => [0xFE81, 0xFE82, 0, 0],
        0x0623 => [0xFE83, 0xFE84, 0, 0],
        0x0624 => [0xFE85, 0xFE86, 0, 0],
        0x0625 => [0xFE87, 0xFE88, 0, 0],
        0x0626 => [0xFE89, 0xFE8A, 0xFE8B, 0xFE8C],
        0x0627 => [0xFE8D, 0xFE8E, 0, 0],
        0x0628 => [0xFE8F, 0xFE90, 0xFE91, 0xFE92],
        0x0629 => [0xFE93, 0xFE94, 0, 0],
        0x062A => [0xFE95, 0xFE96, 0xFE97, 0xFE98],
        0x062B => [0xFE99, 0xFE9A, 0xFE9B, 0xFE9C],
        0x062C => [0xFE9D, 0xFE9E, 0xFE9F, 0xFEA0],
        0x062D => [0xFEA1, 0xFEA2, 0xFEA3, 0xFEA4],
        0x062E => [0xFEA5, 0xFEA6, 0xFEA7, 0xFEA8],
        0x062F => [0xFEA9, 0xFEAA, 0, 0],
        0x0630 => [0xFEAB, 0xFEAC, 0, 0],
        0x0631 => [0xFEAD, 0xFEAE, 0, 0],
        0x0632 => [0xFEAF, 0xFEB0, 0, 0],
        0x0633 => [0xFEB1, 0xFEB2, 0xFEB3, 0xFEB4],
        0x0634 => [0xFEB5, 0xFEB6, 0xFEB7, 0xFEB8],
        0x0635 => [0xFEB9, 0xFEBA, 0xFEBB, 0xFEBC],
        0x0636 => [0xFEBD, 0xFEBE, 0xFEBF, 0xFEC0],
        0x0637 => [0xFEC1, 0xFEC2, 0xFEC3, 0xFEC4],
        0x0638 => [0xFEC5, 0xFEC6, 0xFEC7, 0xFEC8],
        0x0639 => [0xFEC9, 0xFECA, 0xFECB, 0xFECC],
        0x063A => [0xFECD, 0xFECE, 0xFECF, 0xFED0],
        0x0641 => [0xFED1, 0xFED2, 0xFED3, 0xFED4],
        0x0642 => [0xFED5, 0xFED6, 0xFED7, 0xFED8],
        0x0643 => [0xFED9, 0xFEDA, 0xFEDB, 0xFEDC],
        0x0644 => [0xFEDD, 0xFEDE, 0xFEDF, 0xFEE0],
        0x0645 => [0xFEE1, 0xFEE2, 0xFEE3, 0xFEE4],
        0x0646 => [0xFEE5, 0xFEE6, 0xFEE7, 0xFEE8],
        0x0647 => [0xFEE9, 0xFEEA, 0xFEEB, 0xFEEC],
        0x0648 => [0xFEED, 0xFEEE, 0, 0],
        0x0649 => [0xFEEF, 0xFEF0, 0, 0],
        0x064A => [0xFEF1, 0xFEF2, 0xFEF3, 0xFEF4],
        _ => return None,
    })
}

/// Map a letter to the presentation-form character for the requested form,
/// falling back to a simpler form when the requested one doesn't exist.
fn to_presentation_form(c: char, form: Form) -> char {
    let Some([iso, fin, ini, med]) = presentation_forms(c) else {
        return c;
    };
    let pick = |primary: u32, fallback: u32| if primary != 0 { primary } else { fallback };
    let cp = match form {
        Form::Isolated => iso,
        Form::Final => pick(fin, iso),
        Form::Initial => pick(ini, iso),
        Form::Medial => pick(med, pick(fin, iso)),
    };
    char::from_u32(cp).unwrap_or(c)
}

/// Lam-Alef ligature. When a LAM is immediately followed by an ALEF variant
/// the pair is rendered as a single mandatory ligature glyph. `joins_prev`
/// selects the final vs. isolated ligature form.
fn lam_alef_ligature(alef: char, joins_prev: bool) -> char {
    let (iso, fin) = match alef as u32 {
        0x0622 => (0xFEF5, 0xFEF6), // lam + alef madda
        0x0623 => (0xFEF7, 0xFEF8), // lam + alef hamza above
        0x0625 => (0xFEF9, 0xFEFA), // lam + alef hamza below
        0x0627 => (0xFEFB, 0xFEFC), // lam + alef
        _ => unreachable!(),
    };
    char::from_u32(if joins_prev { fin } else { iso }).unwrap()
}

fn is_lam_alef_target(c: char) -> bool {
    matches!(c as u32, 0x0622 | 0x0623 | 0x0625 | 0x0627)
}

/// Apply Arabic contextual shaping, returning a new (still logical-order)
/// sequence where letters have been replaced by their presentation forms and
/// lam-alef pairs collapsed into ligatures.
fn shape(chars: &[char]) -> Vec<char> {
    let n = chars.len();
    let joins: Vec<Join> = chars.iter().map(|&c| joining_type(c)).collect();

    // Index of the nearest non-transparent character before/after `i`.
    let prev_base = |i: usize| {
        (0..i)
            .rev()
            .find(|&k| joins[k] != Join::Transparent)
    };
    let next_base = |i: usize| {
        (i + 1..n).find(|&k| joins[k] != Join::Transparent)
    };

    let mut out = Vec::with_capacity(n);
    let mut i = 0;
    while i < n {
        let c = chars[i];

        // Lam-Alef ligature: only when the alef immediately follows the lam
        // (no intervening marks, which is the overwhelmingly common case).
        if c as u32 == 0x0644 && i + 1 < n && is_lam_alef_target(chars[i + 1]) {
            let joins_prev = prev_base(i)
                .map(|p| matches!(joins[p], Join::Dual | Join::Causing))
                .unwrap_or(false);
            out.push(lam_alef_ligature(chars[i + 1], joins_prev));
            i += 2;
            continue;
        }

        if presentation_forms(c).is_some() {
            let cur = joins[i];
            let prev = prev_base(i).map(|p| joins[p]);
            let next = next_base(i).map(|k| joins[k]);
            // A letter joins to the previous one if it can join on its right
            // and the previous letter can join on its left, and symmetrically
            // for the next letter.
            let joins_prev = matches!(cur, Join::Right | Join::Dual | Join::Causing)
                && matches!(prev, Some(Join::Dual) | Some(Join::Causing));
            let joins_next = matches!(cur, Join::Dual | Join::Causing)
                && matches!(next, Some(Join::Right | Join::Dual | Join::Causing));
            let form = match (joins_prev, joins_next) {
                (true, true) => Form::Medial,
                (true, false) => Form::Final,
                (false, true) => Form::Initial,
                (false, false) => Form::Isolated,
            };
            out.push(to_presentation_form(c, form));
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

// =============================================================================
// MARK: - Bidirectional reordering (simplified UAX #9)
// =============================================================================

#[derive(Copy, Clone, PartialEq, Eq)]
enum Dir {
    Ltr,
    Rtl,
    Neutral,
}

fn is_rtl_strong(c: u32) -> bool {
    (0x0590..=0x05FF).contains(&c) // Hebrew
        || (0x0600..=0x06FF).contains(&c) // Arabic
        || (0x0750..=0x077F).contains(&c)
        || (0x08A0..=0x08FF).contains(&c)
        || (0xFB1D..=0xFB4F).contains(&c) // Hebrew presentation forms
        || (0xFB50..=0xFDFF).contains(&c) // Arabic presentation forms A
        || (0xFE70..=0xFEFF).contains(&c) // Arabic presentation forms B
}

/// Coarse directional class of a (non-combining) character.
fn direction_class(c: char) -> Dir {
    let u = c as u32;
    if is_rtl_strong(u) {
        Dir::Rtl
    } else if c.is_alphabetic()
        || c.is_ascii_digit()
        || (0x0660..=0x0669).contains(&u) // Arabic-Indic digits
        || (0x06F0..=0x06F9).contains(&u) // extended Arabic-Indic digits
    {
        // Numbers are kept left-to-right, matching how digits read inside
        // Arabic text.
        Dir::Ltr
    } else {
        Dir::Neutral
    }
}

/// Combining marks attach to the preceding base character and travel with it.
fn is_combining_mark(c: char) -> bool {
    matches!(c as u32, 0x0300..=0x036F) || joining_type(c) == Join::Transparent
}

/// A grapheme-ish cluster: a base character plus any trailing combining marks.
/// Reordering operates on whole clusters so a mark always stays next to its
/// base in the final visual order.
struct Cluster {
    dir: Dir,
    chars: Vec<char>,
}

/// Reorder a shaped, logical-order line into visual (left-to-right) order.
fn reorder_visual(shaped: &[char]) -> Vec<char> {
    // 1. Group into clusters.
    let mut clusters: Vec<Cluster> = Vec::new();
    for &c in shaped {
        if is_combining_mark(c) && !clusters.is_empty() {
            clusters.last_mut().unwrap().chars.push(c);
        } else {
            clusters.push(Cluster {
                dir: direction_class(c),
                chars: vec![c],
            });
        }
    }
    if clusters.is_empty() {
        return Vec::new();
    }

    // 2. Base direction = direction of the first strong character (default LTR).
    let base = clusters
        .iter()
        .map(|c| c.dir)
        .find(|d| *d != Dir::Neutral)
        .unwrap_or(Dir::Ltr);

    // 3. Resolve neutral runs to the surrounding direction (UAX #9 N1/N2):
    //    a neutral run flanked by the same direction takes it, otherwise it
    //    takes the base direction.
    let n = clusters.len();
    let raw: Vec<Dir> = clusters.iter().map(|c| c.dir).collect();
    let mut resolved = raw.clone();
    let mut i = 0;
    while i < n {
        if raw[i] != Dir::Neutral {
            i += 1;
            continue;
        }
        let start = i;
        while i < n && raw[i] == Dir::Neutral {
            i += 1;
        }
        let before = if start == 0 { base } else { resolved[start - 1] };
        let after = if i == n { base } else { raw[i] };
        let dir = if before == after { before } else { base };
        for r in resolved.iter_mut().take(i).skip(start) {
            *r = dir;
        }
    }

    // 4. Assign embedding levels: characters matching the base direction sit at
    //    the base level, the opposite direction one level deeper.
    let base_level: u8 = if base == Dir::Rtl { 1 } else { 0 };
    let levels: Vec<u8> = resolved
        .iter()
        .map(|&d| base_level + if d == base { 0 } else { 1 })
        .collect();

    // 5. Apply rule L2: from the highest level down to 1, reverse every
    //    contiguous run of clusters at that level or above.
    let order = resolve_order(&levels);

    let mut out = Vec::with_capacity(shaped.len());
    for idx in order {
        out.extend_from_slice(&clusters[idx].chars);
    }
    out
}

/// UAX #9 rule L2: produce the visual order of indices from embedding levels.
fn resolve_order(levels: &[u8]) -> Vec<usize> {
    let n = levels.len();
    let mut order: Vec<usize> = (0..n).collect();
    let Some(&max) = levels.iter().max() else {
        return order;
    };
    let mut level = max;
    while level >= 1 {
        let mut i = 0;
        while i < n {
            if levels[i] >= level {
                let start = i;
                while i < n && levels[i] >= level {
                    i += 1;
                }
                order[start..i].reverse();
            } else {
                i += 1;
            }
        }
        level -= 1;
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cps(s: &str) -> Vec<u32> {
        s.chars().map(|c| c as u32).collect()
    }

    #[test]
    fn non_arabic_is_untouched() {
        assert!(matches!(
            shape_line_for_display("Hello, world!"),
            Cow::Borrowed("Hello, world!")
        ));
    }

    #[test]
    fn salaam_is_shaped_and_reversed() {
        // "سلام" = seen, lam, alef, meem (logical order).
        let out = shape_line_for_display("\u{0633}\u{0644}\u{0627}\u{0645}");
        // Expected visual (left-to-right): meem (isolated), lam-alef (final
        // ligature), seen (initial).
        assert_eq!(cps(&out), vec![0xFEE1, 0xFEFC, 0xFEB3]);
    }

    #[test]
    fn medial_form_is_used_in_the_middle() {
        // "ببب" three behs: initial, medial, final. Reversed for display, the
        // first drawn glyph is the final form, last is the initial form.
        let out = shape_line_for_display("\u{0628}\u{0628}\u{0628}");
        assert_eq!(cps(&out), vec![0xFE90, 0xFE92, 0xFE91]);
    }

    #[test]
    fn latin_run_keeps_order_inside_arabic() {
        // alef, beh, then "12": the digits stay left-to-right and land on the
        // left of the right-to-left letters.
        let out = shape_line_for_display("\u{0627}\u{0628}12");
        assert_eq!(cps(&out), vec!['1' as u32, '2' as u32, 0xFE8F, 0xFE8D]);
    }

    #[test]
    fn combining_mark_stays_with_its_base() {
        // beh + fatha (U+064E). The mark must remain immediately after the
        // (shaped) beh in the output.
        let out = shape_line_for_display("\u{0628}\u{064E}");
        assert_eq!(cps(&out), vec![0xFE8F, 0x064E]);
    }
}
