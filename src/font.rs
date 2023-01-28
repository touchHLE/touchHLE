//! Text layout and font rasterization abstraction.
//!
//! This is implemented using the [rusttype] library. All usage of that library
//! should be confined to this module.

use rusttype::{Point, Rect, Scale};
use std::cmp;

pub struct Font {
    font: rusttype::Font<'static>,
}

pub enum TextAlignment {
    Left,
    Center,
    Right,
}

fn update_bounds(text_bounds: &mut Rect<i32>, glyph_bounds: &Rect<i32>) {
    text_bounds.min.x = cmp::min(text_bounds.min.x, glyph_bounds.min.x);
    text_bounds.min.y = cmp::min(text_bounds.min.y, glyph_bounds.min.y);
    text_bounds.max.x = cmp::max(text_bounds.max.x, glyph_bounds.max.x);
    text_bounds.max.y = cmp::max(text_bounds.max.y, glyph_bounds.max.y);
}

impl Font {
    fn from_file(path: &str) -> Font {
        let Ok(bytes) = std::fs::read(path) else {
            panic!("Couldn't read bundled font file {:?}. Perhaps the directory is missing?", path);
        };

        let Some(font) = rusttype::Font::try_from_vec(bytes) else {
            panic!("Couldn't parse bundled font file {:?}. This probably means the file is corrupt. Try re-downloading it.", path);
        };

        Font { font }
    }

    // TODO: add a Japanese font (for Super Monkey Ball when LANG=ja)
    pub fn sans_regular() -> Font {
        Self::from_file("touchHLE_fonts/LiberationSans-Regular.ttf")
    }
    pub fn sans_bold() -> Font {
        Self::from_file("touchHLE_fonts/LiberationSans-Bold.ttf")
    }
    pub fn sans_italic() -> Font {
        Self::from_file("touchHLE_fonts/LiberationSans-Italic.ttf")
    }

    fn line_height_and_gap(&self, font_size: f32) -> (f32, f32) {
        let v_metrics = self.font.v_metrics(Scale::uniform(font_size));
        (v_metrics.ascent - v_metrics.descent, v_metrics.line_gap)
    }

    /// Break text into lines with known widths.
    fn break_lines<'a>(&self, font_size: f32, text: &'a str) -> Vec<(f32, &'a str)> {
        let mut lines = Vec::new();

        for line in text.lines() {
            let mut line_bounds: Rect<i32> = Default::default();

            for glyph in self
                .font
                .layout(line, Scale::uniform(font_size), Default::default())
            {
                let Some(glyph_bounds) = glyph.pixel_bounding_box() else {
                    continue;
                };
                update_bounds(&mut line_bounds, &glyph_bounds);
            }

            lines.push((line_bounds.width() as f32, line));
        }

        lines
    }

    /// Calculate the on-screen width and height of text with a given font size.
    pub fn calculate_text_size(&self, font_size: f32, text: &str) -> (f32, f32) {
        let lines = self.break_lines(font_size, text);

        let width = lines
            .iter()
            .fold(0.0, |sum, &(line_width, _line)| sum + line_width);
        let (line_height, line_gap) = self.line_height_and_gap(font_size);
        let height = line_height * (lines.len() as f32) + line_gap * ((lines.len() - 1) as f32);

        (width, height)
    }

    /// Draw text. Calls the provided callback for each pixel, providing the
    /// coverage (a value between 0.0 and 1.0).
    pub fn draw<F: FnMut((i32, i32), f32)>(
        &self,
        font_size: f32,
        text: &str,
        origin: (f32, f32),
        alignment: TextAlignment,
        mut put_pixel: F,
    ) {
        let lines = self.break_lines(font_size, text);

        let (line_height, line_gap) = self.line_height_and_gap(font_size);
        let mut line_y = line_height * ((lines.len() - 1) as f32)
            + line_gap * (lines.len().saturating_sub(2) as f32)
            - self.font.v_metrics(Scale::uniform(font_size)).descent;

        for (line_width, line_text) in lines {
            let line_x_offset = match alignment {
                TextAlignment::Left => 0.0,
                TextAlignment::Center => -line_width / 2.0,
                TextAlignment::Right => -line_width,
            };
            for glyph in self.font.layout(
                line_text,
                Scale::uniform(font_size),
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
                let y_offset = ((origin.1 + line_y).round() as i32) - glyph_bounds.max.y;
                glyph.draw(|x, y, coverage| {
                    let (x, y) = (x as i32, y as i32);
                    put_pixel((x_offset + x, y_offset + (glyph_height - y)), coverage)
                });
            }
            line_y -= line_height + line_gap;
        }
    }
}
