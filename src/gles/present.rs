/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Utilities for presenting frames to the window using an abstract OpenGL ES
//! implementation.

use super::gles11_raw as gles11; // constants and types only
use super::GLES;
use crate::matrix::Matrix;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

/// Set by the launcher when the in-game FPS readout ("Metal HUD" for 32-bit
/// apps, which render through OpenGL ES rather than Metal) is enabled.
static HUD_ENABLED: LazyLock<bool> =
    LazyLock::new(|| std::env::var_os("PEAR_FPS_HUD").is_some_and(|value| value != "0"));

/// Most recent FPS measurement, stored as `f32` bits; zero means "none yet".
static LAST_FPS: AtomicU32 = AtomicU32::new(0);

pub fn hud_enabled() -> bool {
    *HUD_ENABLED
}

pub struct FpsCounter {
    time: std::time::Instant,
    frames: u32,
}
impl FpsCounter {
    pub fn start() -> Self {
        FpsCounter {
            time: Instant::now(),
            frames: 0,
        }
    }

    pub fn count_frame(&mut self, label: std::fmt::Arguments<'_>) {
        self.frames += 1;
        let now = Instant::now();
        let duration = now - self.time;
        if duration >= Duration::from_secs(1) {
            self.time = now;
            let fps = std::mem::take(&mut self.frames) as f32 / duration.as_secs_f32();
            LAST_FPS.store(fps.to_bits(), Ordering::Relaxed);
            echo!("{} FPS: {:.2}", label, fps);
        }
    }
}

/// Present the the latest frame (e.g. the app's splash screen or rendering
/// output), provided as a texture bound to `GL_TEXTURE_2D`, by drawing it on
/// the window. It may be rotated, scaled and/or letterboxed as necessary. The
/// virtual cursor is also drawn if it should be currently visible.
///
/// The provided context must be current.
pub unsafe fn present_frame(
    gles: &mut dyn GLES,
    viewport: (u32, u32, u32, u32),
    rotation_matrix: Matrix<2>,
    virtual_cursor_visible_at: Option<(f32, f32, bool)>,
) {
    // While this is a generic utility, it is closely tied to
    // crate::frameworks::opengles::eagl::present_renderbuffer, which handles
    // backing up and restoring OpenGL ES state that this function might touch,
    // so these need to be updated in tandem.

    use gles11::types::*;

    // Draw the quad
    gles.Viewport(
        viewport.0 as _,
        viewport.1 as _,
        viewport.2 as _,
        viewport.3 as _,
    );
    gles.ClearColor(0.0, 0.0, 0.0, 1.0);
    gles.Clear(gles11::COLOR_BUFFER_BIT | gles11::DEPTH_BUFFER_BIT | gles11::STENCIL_BUFFER_BIT);
    gles.BindBuffer(gles11::ARRAY_BUFFER, 0);
    let vertices: [f32; 12] = [
        -1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0,
    ];
    gles.EnableClientState(gles11::VERTEX_ARRAY);
    gles.VertexPointer(2, gles11::FLOAT, 0, vertices.as_ptr() as *const GLvoid);
    let tex_coords: [f32; 12] = [0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    gles.EnableClientState(gles11::TEXTURE_COORD_ARRAY);
    gles.TexCoordPointer(2, gles11::FLOAT, 0, tex_coords.as_ptr() as *const GLvoid);
    let matrix = Matrix::<4>::from(&rotation_matrix);
    gles.MatrixMode(gles11::TEXTURE);
    gles.LoadMatrixf(matrix.columns().as_ptr() as *const _);
    gles.Enable(gles11::TEXTURE_2D);
    gles.DrawArrays(gles11::TRIANGLES, 0, 6);
    // clean this up so we don't need to worry about it in e.g. Core Animation
    gles.LoadIdentity();

    // Display virtual cursor
    if let Some((x, y, pressed)) = virtual_cursor_visible_at {
        let (vx, vy, vw, vh) = viewport;
        let x = x - vx as f32;
        let y = y - vy as f32;

        gles.DisableClientState(gles11::TEXTURE_COORD_ARRAY);
        gles.Disable(gles11::TEXTURE_2D);

        gles.Enable(gles11::BLEND);
        gles.BlendFunc(gles11::ONE, gles11::ONE_MINUS_SRC_ALPHA);
        gles.Color4f(0.0, 0.0, 0.0, if pressed { 2.0 / 3.0 } else { 1.0 / 3.0 });

        let radius = 10.0;

        let mut vertices = vertices;
        for i in (0..vertices.len()).step_by(2) {
            vertices[i] = (vertices[i] * radius + x) / (vw as f32 / 2.0) - 1.0;
            vertices[i + 1] = 1.0 - (vertices[i + 1] * radius + y) / (vh as f32 / 2.0);
        }
        gles.VertexPointer(2, gles11::FLOAT, 0, vertices.as_ptr() as *const GLvoid);
        gles.DrawArrays(gles11::TRIANGLES, 0, 6);
    }

    if hud_enabled() {
        draw_fps_hud(gles, viewport);
        self_test_hud(gles, viewport);
    }
}

/// One-shot verification aid: reads the drawn overlay back and reports how
/// many pixels of HUD colour it found. Enabled only by `PEAR_HUD_SELFTEST`.
unsafe fn self_test_hud(gles: &mut dyn GLES, viewport: (u32, u32, u32, u32)) {
    use gles11::types::*;

    static SELFTEST: LazyLock<bool> =
        LazyLock::new(|| std::env::var_os("PEAR_HUD_SELFTEST").is_some());
    static DONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !*SELFTEST || DONE.load(Ordering::Relaxed) {
        return;
    }
    // The overlay only appears once a frame rate has been measured.
    if f32::from_bits(LAST_FPS.load(Ordering::Relaxed)) <= 0.0 {
        return;
    }
    DONE.store(true, Ordering::Relaxed);
    let (_, _, width, height) = viewport;
    if width == 0 || height == 0 {
        return;
    }
    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
    gles.PixelStorei(gles11::PACK_ALIGNMENT, 1);
    gles.ReadPixels(
        0,
        0,
        width as GLsizei,
        height as GLsizei,
        gles11::RGBA,
        gles11::UNSIGNED_BYTE,
        pixels.as_mut_ptr() as *mut GLvoid,
    );
    // The overlay is drawn in the window's top-left corner; OpenGL's origin is
    // bottom-left, so that is the last rows of the read-back buffer.
    let mut found = 0usize;
    let region_height = (height / 6).max(1);
    for row in 0..region_height {
        let y = height - 1 - row;
        for x in 0..(width / 3).max(1) {
            let offset = ((y * width + x) * 4) as usize;
            let (r, g, b) = (pixels[offset], pixels[offset + 1], pixels[offset + 2]);
            // Any strongly saturated pixel belongs to the overlay; the game
            // underneath is rendered in dim greys at this point in the boot.
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            if max > 120 && max - min > 60 {
                found += 1;
            }
        }
    }
    log!("HUD self-test: {} overlay pixels in {}x{}", found, width, height);
}

/// Draws the FPS readout in the top-left corner of the window.
///
/// The engine presents through OpenGL ES, so Metal's own HUD cannot appear;
/// this seven-segment overlay is the equivalent for emulated 32-bit apps. It
/// deliberately shows no engine name, only the frame rate.
unsafe fn draw_fps_hud(gles: &mut dyn GLES, viewport: (u32, u32, u32, u32)) {
    use gles11::types::*;

    let fps = f32::from_bits(LAST_FPS.load(Ordering::Relaxed));
    if fps <= 0.0 {
        return;
    }
    let (_, _, vw, vh) = viewport;
    if vw == 0 || vh == 0 {
        return;
    }

    let scale = (vh as f32 / 180.0).max(1.5);
    let thickness = (2.0 * scale).max(2.0);
    let digit_w = 7.0 * scale;
    let digit_h = 14.0 * scale;
    let gap = 2.5 * scale;
    let padding = 4.0 * scale;

    let text = format!("FPS {}", fps.round().max(0.0) as u32);
    let text_w = text
        .chars()
        .map(|c| if c == ' ' { digit_w * 0.6 } else { digit_w + gap })
        .sum::<f32>();
    let origin_x = padding;
    let origin_y = padding;

    let mut quads = |rects: &mut Vec<f32>, x: f32, y: f32, w: f32, h: f32| {
        let x0 = x / (vw as f32 / 2.0) - 1.0;
        let x1 = (x + w) / (vw as f32 / 2.0) - 1.0;
        let y0 = 1.0 - y / (vh as f32 / 2.0);
        let y1 = 1.0 - (y + h) / (vh as f32 / 2.0);
        rects.extend_from_slice(&[x0, y0, x0, y1, x1, y0, x1, y0, x0, y1, x1, y1]);
    };

    // Background plate for legibility over bright scenes.
    let mut background = Vec::with_capacity(12);
    quads(
        &mut background,
        0.0,
        0.0,
        text_w + padding * 2.0,
        digit_h + padding * 2.0,
    );

    let mut segments: Vec<f32> = Vec::new();
    let mut pen_x = origin_x;
    for character in text.chars() {
        if character == ' ' {
            pen_x += digit_w * 0.6 + gap;
            continue;
        }
        let mask = segment_mask(character);
        let (x, y, w, h, t) = (pen_x, origin_y, digit_w, digit_h, thickness);
        let half = h / 2.0;
        let vertical_h = half - t * 1.5;
        let rects = [
            (0b0000001, x + t, y, w - 2.0 * t, t),                        // a
            (0b0000010, x + w - t, y + t, t, vertical_h),                  // b
            (0b0000100, x + w - t, y + half + t / 2.0, t, vertical_h),     // c
            (0b0001000, x + t, y + h - t, w - 2.0 * t, t),                 // d
            (0b0010000, x, y + half + t / 2.0, t, vertical_h),             // e
            (0b0100000, x, y + t, t, vertical_h),                          // f
            (0b1000000, x + t, y + half - t / 2.0, w - 2.0 * t, t),        // g
        ];
        for (bit, rx, ry, rw, rh) in rects {
            if mask & bit != 0 {
                quads(&mut segments, rx, ry, rw, rh);
            }
        }
        pen_x += digit_w + gap;
    }

    gles.Disable(gles11::TEXTURE_2D);
    gles.DisableClientState(gles11::TEXTURE_COORD_ARRAY);
    gles.Enable(gles11::BLEND);
    gles.BlendFunc(gles11::ONE, gles11::ONE_MINUS_SRC_ALPHA);
    gles.EnableClientState(gles11::VERTEX_ARRAY);

    gles.Color4f(0.0, 0.0, 0.0, 0.55);
    gles.VertexPointer(2, gles11::FLOAT, 0, background.as_ptr() as *const GLvoid);
    gles.DrawArrays(gles11::TRIANGLES, 0, (background.len() / 2) as _);

    // Green while smooth, amber when dropping, red when clearly struggling.
    let (r, g, b) = match fps {
        fps if fps >= 50.0 => (0.45, 1.0, 0.45),
        fps if fps >= 28.0 => (1.0, 0.85, 0.35),
        _ => (1.0, 0.45, 0.45),
    };
    gles.Color4f(r, g, b, 1.0);
    gles.VertexPointer(2, gles11::FLOAT, 0, segments.as_ptr() as *const GLvoid);
    gles.DrawArrays(gles11::TRIANGLES, 0, (segments.len() / 2) as _);
}

/// Segments for one character: bit 0 top, then clockwise, bit 6 middle.
fn segment_mask(character: char) -> u8 {
    match character {
        '0' => 0b011_1111,
        '1' => 0b000_0110,
        '2' => 0b101_1011,
        '3' => 0b100_1111,
        '4' => 0b110_0110,
        '5' => 0b110_1101,
        '6' => 0b111_1101,
        '7' => 0b000_0111,
        '8' => 0b111_1111,
        '9' => 0b110_1111,
        'F' => 0b111_0001,
        'P' => 0b111_0011,
        'S' => 0b110_1101,
        _ => 0,
    }
}
