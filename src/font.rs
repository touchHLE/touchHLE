//! Text layout and font rasterization abstraction.
//!
//! This is implemented using the [rusttype] library. All usage of that library
//! should be confined to this module.

use rusttype::{Point, Rect, Scale};
use std::cmp;

pub struct Font {
    font: rusttype::Font<'static>,
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

    /// Calculate the on-screen width and height of text with a given font size.
    pub fn calculate_text_size(&self, font_size: f32, text: &str) -> (f32, f32) {
        let mut text_bounds: Rect<i32> = Default::default();

        for glyph in self
            .font
            .layout(text, Scale::uniform(font_size), Default::default())
        {
            let Some(glyph_bounds) = glyph.pixel_bounding_box() else {
                continue;
            };
            text_bounds.min.x = cmp::min(text_bounds.min.x, glyph_bounds.min.x);
            text_bounds.min.y = cmp::min(text_bounds.min.y, glyph_bounds.min.y);
            text_bounds.max.x = cmp::max(text_bounds.max.x, glyph_bounds.max.x);
            text_bounds.max.y = cmp::max(text_bounds.max.y, glyph_bounds.max.y);
        }

        (text_bounds.width() as f32, text_bounds.height() as f32)
    }

    /// Draw text. Calls the provided callback for each pixel, providing the
    /// coverage (a value between 0.0 and 1.0).
    pub fn draw<F: FnMut((i32, i32), f32)>(
        &self,
        font_size: f32,
        text: &str,
        origin: (f32, f32),
        mut put_pixel: F,
    ) {
        for glyph in self.font.layout(
            text,
            Scale::uniform(font_size),
            Point {
                x: origin.0,
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
            let y_offset = (origin.1.round() as i32) - glyph_bounds.max.y;
            glyph.draw(|x, y, coverage| {
                let (x, y) = (x as i32, y as i32);
                put_pixel((x_offset + x, y_offset + (glyph_height - y)), coverage)
            });
        }
    }
}
