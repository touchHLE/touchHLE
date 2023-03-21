/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Text layout and font rasterization abstraction.
//!
//! This is implemented using the [rusttype] library. All usage of that library
//! should be confined to this module.
//!
//! TODO: Less terrible text layout. RustType doesn't do text layout so this
//! code has its own, not particularly good implementation. We might want to
//! switch to something like cosmic-text in future, but that has a _lot_ more
//! dependencies.

use rusttype::{Point, Rect, Scale};
use std::cmp;
use std::env;

pub struct Font {
    font: rusttype::Font<'static>,
}

pub enum TextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Copy, Clone)]
pub enum WrapMode {
    Word,
    Char,
}

fn update_bounds(text_bounds: &mut Rect<i32>, glyph_bounds: &Rect<i32>) {
    text_bounds.min.x = cmp::min(text_bounds.min.x, glyph_bounds.min.x);
    text_bounds.min.y = cmp::min(text_bounds.min.y, glyph_bounds.min.y);
    text_bounds.max.x = cmp::max(text_bounds.max.x, glyph_bounds.max.x);
    text_bounds.max.y = cmp::max(text_bounds.max.y, glyph_bounds.max.y);
}

fn scale(font_size: f32) -> Scale {
    // iPhone OS's interpretation of font size is slightly different, reason
    // unknown. This is not the same as the Windows pt vs Mac pt issue.
    // This scale factor has been eyeball'd, it's not exact.
    Scale::uniform(font_size * 1.125)
}

impl Font {
    fn from_file(path: &str) -> Font {
        let prefix = if env::consts::OS == "android" {
            "/data/data/org.touch.hle/files/"
        } else {
            ""
        };

        let Ok(bytes) = std::fs::read(prefix.to_owned() + path) else {
            panic!("Couldn't read bundled font file {:?}. Perhaps the directory is missing?", path);
        };

        let Some(font) = rusttype::Font::try_from_vec(bytes) else {
            panic!("Couldn't parse bundled font file {:?}. This probably means the file is corrupt. Try re-downloading it.", path);
        };

        Font { font }
    }

    pub fn sans_regular() -> Font {
        Self::from_file("touchHLE_fonts/LiberationSans-Regular.ttf")
    }
    pub fn sans_bold() -> Font {
        Self::from_file("touchHLE_fonts/LiberationSans-Bold.ttf")
    }
    pub fn sans_italic() -> Font {
        Self::from_file("touchHLE_fonts/LiberationSans-Italic.ttf")
    }
    pub fn sans_regular_ja() -> Font {
        Self::from_file("touchHLE_fonts/NotoSansJP-Regular.otf")
    }
    pub fn sans_bold_ja() -> Font {
        Self::from_file("touchHLE_fonts/NotoSansJP-Bold.otf")
    }

    fn line_height_and_gap(&self, font_size: f32) -> (f32, f32) {
        let v_metrics = self.font.v_metrics(scale(font_size));
        (v_metrics.ascent - v_metrics.descent, v_metrics.line_gap)
    }

    /// Calculate the width of a line. This does not handle newlines!
    fn calculate_line_width(&self, font_size: f32, line: &str) -> f32 {
        let mut line_bounds: Rect<i32> = Default::default();

        for glyph in self.font.layout(line, scale(font_size), Default::default()) {
            let Some(glyph_bounds) = glyph.pixel_bounding_box() else {
                continue;
            };
            update_bounds(&mut line_bounds, &glyph_bounds);
        }

        line_bounds.width() as f32
    }

    /// Break text into lines with known widths.
    fn break_lines<'a>(
        &self,
        font_size: f32,
        text: &'a str,
        wrap: Option<(f32, WrapMode)>,
    ) -> Vec<(f32, &'a str)> {
        let mut lines = Vec::new();

        for line in text.lines() {
            let Some((wrap_width, wrap_mode)) = wrap else {
                lines.push((self.calculate_line_width(font_size, line), line));
                continue;
            };

            // Find points at which the line could be wrapped
            let mut wrap_points = Vec::new();
            match wrap_mode {
                WrapMode::Word => {
                    let mut word_start = 0;

                    loop {
                        if let Some(i) = line[word_start..].find(|c: char| c.is_whitespace()) {
                            let word_end = word_start + i;
                            // Include any additional whitespace in the word,
                            // so that the next word begins with non-whitespace.
                            if let Some(i) = line[word_end..].find(|c: char| !c.is_whitespace()) {
                                wrap_points.push(word_end + i);
                                word_start = word_end + i;
                            } else {
                                wrap_points.push(line.len());
                                break;
                            }
                        } else {
                            wrap_points.push(line.len());
                            break;
                        }
                    }
                }
                WrapMode::Char => {
                    let mut char_end = 1.min(line.len());

                    while char_end <= line.len() {
                        if line.is_char_boundary(char_end) {
                            wrap_points.push(char_end);
                        }
                        char_end += 1;
                    }
                }
            };

            let mut next_wrap_point_idx = 0;
            let mut line_start = 0;

            while next_wrap_point_idx < wrap_points.len() {
                // Find optimal line wrapping by binary search.
                // `binary_search_by` returns Err when there's no exactly
                // matching line length, which is usually going to be the case.
                let wrap_search_result =
                    wrap_points[next_wrap_point_idx..].binary_search_by(|&wrap_point| {
                        let line = &line[line_start..wrap_point];
                        let line_width = self.calculate_line_width(font_size, line);
                        line_width.partial_cmp(&wrap_width).unwrap()
                    });
                let wrap_point_idx = match wrap_search_result {
                    Ok(i) => next_wrap_point_idx + i,
                    Err(i) => (next_wrap_point_idx + i).wrapping_sub(1),
                };

                let line_end = wrap_points[wrap_point_idx];
                let line = &line[line_start..line_end];
                lines.push((self.calculate_line_width(font_size, line), line));

                next_wrap_point_idx = wrap_point_idx + 1;
                line_start = line_end;
            }
        }

        lines
    }

    /// Calculate the on-screen width and height of text with a given font size.
    pub fn calculate_text_size(
        &self,
        font_size: f32,
        text: &str,
        wrap: Option<(f32, WrapMode)>,
    ) -> (f32, f32) {
        let lines = self.break_lines(font_size, text, wrap);

        let width = lines
            .iter()
            .fold(0f32, |widest, &(line_width, _line)| widest.max(line_width));
        let (line_height, line_gap) = self.line_height_and_gap(font_size);
        let height = line_height * (lines.len() as f32) + line_gap * ((lines.len() - 1) as f32);

        (width, height)
    }

    /// Draw text. Calls the provided callback for each pixel, providing the
    /// coverage (a value between 0.0 and 1.0). Assumes y starts at the
    /// bottom-left corner and points upwards.
    pub fn draw<F: FnMut((i32, i32), f32)>(
        &self,
        font_size: f32,
        text: &str,
        origin: (f32, f32),
        wrap: Option<(f32, WrapMode)>,
        alignment: TextAlignment,
        mut put_pixel: F,
    ) {
        // TODO: This code has gone through a rather traumatic series of y sign
        //       flips and might benefit from refactoring for clarity?

        let lines = self.break_lines(font_size, text, wrap);

        let mut line_y = self.font.v_metrics(scale(font_size)).ascent;
        let (line_height, line_gap) = self.line_height_and_gap(font_size);

        for (line_width, line_text) in lines {
            let line_x_offset = match alignment {
                TextAlignment::Left => 0.0,
                TextAlignment::Center => -line_width / 2.0,
                TextAlignment::Right => -line_width,
            };
            for glyph in self.font.layout(
                line_text,
                scale(font_size),
                Point {
                    x: origin.0 + line_x_offset,
                    y: 0.0,
                },
            ) {
                let Some(glyph_bounds) = glyph.pixel_bounding_box() else {
                    continue;
                };
                // y needs to be flipped to point up
                // FIXME: blending
                let glyph_height = glyph_bounds.height();
                let x_offset = glyph_bounds.min.x;
                let y_offset = ((origin.1 + line_y).round() as i32) + glyph_bounds.max.y;
                glyph.draw(|x, y, coverage| {
                    let (x, y) = (x as i32, y as i32);
                    put_pixel((x_offset + x, y_offset - (glyph_height - y)), coverage)
                });
            }
            line_y += line_height + line_gap;
        }
    }
}
