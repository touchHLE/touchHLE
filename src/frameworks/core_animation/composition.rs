/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The implementation of layer compositing.
//!
//! This is completely original; I don't think Apple document how this works and
//! I haven't attempted to reverse-engineer the details. As such, it probably
//! diverges wildly from what the real iPhone OS does.

use super::ca_eagl_layer::find_fullscreen_eagl_layer;
use super::ca_layer::CALayerHostObject;
use crate::frameworks::core_graphics::{
    cg_bitmap_context, cg_image, CGFloat, CGPoint, CGRect, CGSize,
};
use crate::frameworks::uikit::ui_color;
use crate::gles::gles11_raw as gles11; // constants only
use crate::gles::gles11_raw::types::*;
use crate::gles::present::{present_frame, FpsCounter};
use crate::gles::GLES;
use crate::mem::Mem;
use crate::objc::{id, msg, msg_class, nil, ObjC};
use crate::Environment;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct State {
    texture_framebuffer: Option<(GLuint, GLuint)>,
    recomposite_next: Option<Instant>,
    fps_counter: Option<FpsCounter>,
}

/// For use by `NSRunLoop`: call this 60 times per second. Composites the app's
/// visible layers (i.e. UI) and presents it to the screen. Does nothing if
/// composition isn't in use or it's too soon.
///
/// Returns the time a recomposite is due, if any.
pub fn recomposite_if_necessary(env: &mut Environment) -> Option<Instant> {
    // Assumes the last window in the list is the one on top.
    // TODO: this is not correct once we support zPosition.
    // TODO: can there be windows smaller than the screen? If so we need to draw
    //       all of them.
    let Some(&top_window) = env
        .framework_state
        .uikit
        .ui_view
        .ui_window
        .visible_windows
        .last()
    else {
        log_dbg!("No visible window, skipping composition");
        return None;
    };

    if find_fullscreen_eagl_layer(env) != nil {
        // No composition done, EAGLContext will present directly.
        log_dbg!("Using CAEAGLLayer fast path, skipping composition");
        return None;
    }

    if env.options.print_fps {
        env.framework_state
            .core_animation
            .composition
            .fps_counter
            .get_or_insert_with(FpsCounter::start)
            .count_frame(format_args!("Core Animation compositor"));
    }

    let now = Instant::now();
    let interval = 1.0 / 60.0; // 60Hz
    let new_recomposite_next = if let Some(recomposite_next) = env
        .framework_state
        .core_animation
        .composition
        .recomposite_next
    {
        if recomposite_next > now {
            log_dbg!("Not recompositing yet, wait {:?}", recomposite_next - now);
            return Some(recomposite_next);
        }

        // See NSTimer implementation for a discussion of what this does.
        let overdue_by = now.duration_since(recomposite_next);
        log_dbg!("Recompositing, overdue by {:?}", overdue_by);
        // TODO: Use `.div_duration_f64()` once that is stabilized.
        let advance_by = (overdue_by.as_secs_f64() / interval).max(1.0).ceil();
        assert!(advance_by == (advance_by as u32) as f64);
        let advance_by = advance_by as u32;
        if advance_by > 1 {
            log_dbg!("Warning: compositor is lagging. It is overdue by {}s and has missed {} interval(s)!", overdue_by.as_secs_f64(), advance_by - 1);
        }
        let advance_by = Duration::from_secs_f64(interval)
            .checked_mul(advance_by)
            .unwrap();
        Some(recomposite_next.checked_add(advance_by).unwrap())
    } else {
        Some(now.checked_add(Duration::from_secs_f64(interval)).unwrap())
    };
    env.framework_state
        .core_animation
        .composition
        .recomposite_next = new_recomposite_next;

    let root_layer: id = msg![env; top_window layer];

    // Ensure layer bitmaps are up to date.
    display_layers(env, root_layer);

    let screen_bounds: CGRect = {
        let screen: id = msg_class![env; UIScreen mainScreen];
        msg![env; screen bounds]
    };
    let scale_hack: u32 = env.options.scale_hack.get();
    let fb_width = screen_bounds.size.width as u32 * scale_hack;
    let fb_height = screen_bounds.size.height as u32 * scale_hack;
    let present_frame_args = (
        env.window().viewport(),
        env.window().rotation_matrix(),
        env.window().virtual_cursor_visible_at(),
    );

    // TODO: draw status bar if it's not hidden

    // Initial state for layer tree traversal (see composite_layer_recursive)
    let origin = CGPoint { x: 0.0, y: 0.0 };
    let clip_to = CGRect {
        origin,
        size: screen_bounds.size,
    };
    let opacity = 1.0;

    let window = env.window.as_mut().unwrap();
    window.make_internal_gl_ctx_current();
    let gles = window.get_internal_gl_ctx();

    // Set up GL objects needed for render-to-texture. We could draw directly
    // to the screen instead, but this way we can reuse the code for scaling and
    // rotating the screen and drawing the virtual cursor.
    let texture = if let Some((texture, framebuffer)) = env
        .framework_state
        .core_animation
        .composition
        .texture_framebuffer
    {
        unsafe {
            gles.BindFramebufferOES(gles11::FRAMEBUFFER_OES, framebuffer);
        };
        texture
    } else {
        let mut texture = 0;
        let mut framebuffer = 0;
        unsafe {
            gles.GenTextures(1, &mut texture);
            gles.BindTexture(gles11::TEXTURE_2D, texture);
            gles.TexImage2D(
                gles11::TEXTURE_2D,
                0,
                gles11::RGBA as _,
                fb_width as _,
                fb_height as _,
                0,
                gles11::RGBA,
                gles11::UNSIGNED_BYTE,
                std::ptr::null(),
            );
            gles.TexParameteri(
                gles11::TEXTURE_2D,
                gles11::TEXTURE_MIN_FILTER,
                gles11::LINEAR as _,
            );
            gles.TexParameteri(
                gles11::TEXTURE_2D,
                gles11::TEXTURE_MAG_FILTER,
                gles11::LINEAR as _,
            );

            gles.GenFramebuffersOES(1, &mut framebuffer);
            gles.BindFramebufferOES(gles11::FRAMEBUFFER_OES, framebuffer);
            gles.FramebufferTexture2DOES(
                gles11::FRAMEBUFFER_OES,
                gles11::COLOR_ATTACHMENT0_OES,
                gles11::TEXTURE_2D,
                texture,
                0,
            );
            assert_eq!(gles.GetError(), 0);
            assert_eq!(
                gles.CheckFramebufferStatusOES(gles11::FRAMEBUFFER_OES),
                gles11::FRAMEBUFFER_COMPLETE_OES
            );
        }
        env.framework_state
            .core_animation
            .composition
            .texture_framebuffer = Some((texture, framebuffer));
        texture
    };

    // Clear the framebuffer and set up state to prepare for rendering
    unsafe {
        gles.Viewport(0, 0, fb_width as _, fb_height as _);
        gles.ClearColor(0.0, 0.0, 0.0, 1.0);
        gles.Clear(gles11::COLOR_BUFFER_BIT);
        gles.Enable(gles11::SCISSOR_TEST);
        gles.Scissor(0, 0, fb_width as _, fb_height as _);
        gles.Color4f(1.0, 1.0, 1.0, 1.0);
    }

    // Here's where the actual drawing happens
    unsafe {
        composite_layer_recursive(
            gles,
            &mut env.objc,
            &env.mem,
            root_layer,
            origin,
            clip_to,
            opacity,
            scale_hack,
            fb_height,
        );
    }

    // Clean up some GL state
    unsafe {
        gles.Viewport(0, 0, fb_width as _, fb_height as _);
        gles.Disable(gles11::SCISSOR_TEST);
        gles.Color4f(1.0, 1.0, 1.0, 1.0);
        gles.Disable(gles11::BLEND);
        assert_eq!(gles.GetError(), 0);
    }

    // Present our rendered frame (bound to TEXTURE_2D). This copies it to the
    // default framebuffer (0) so we need to unbind our internal framebuffer.
    unsafe {
        gles.BindTexture(gles11::TEXTURE_2D, texture);
        gles.BindFramebufferOES(gles11::FRAMEBUFFER_OES, 0);
        present_frame(
            gles,
            present_frame_args.0,
            present_frame_args.1,
            present_frame_args.2,
        );
    }
    env.window().swap_window();

    new_recomposite_next
}

/// Call `displayIfNeeded` on all relevant layers in the tree, so their bitmaps
/// are up to date before compositing.
fn display_layers(env: &mut Environment, root_layer: id) {
    // Tell layers to redraw themselves if needed.

    fn traverse(objc: &ObjC, layer: id, layers_needing_display: &mut Vec<id>) {
        let host_obj = objc.borrow::<CALayerHostObject>(layer);
        if host_obj.hidden {
            return;
        }
        if host_obj.needs_display {
            layers_needing_display.push(layer);
        }
        for &layer in &host_obj.sublayers {
            traverse(objc, layer, layers_needing_display);
        }
    }

    let mut layers_needing_display = Vec::new();
    traverse(&env.objc, root_layer, &mut layers_needing_display);

    for layer in layers_needing_display {
        () = msg![env; layer displayIfNeeded];
    }
}

/// Traverses the layer tree and draws each layer.
unsafe fn composite_layer_recursive(
    gles: &mut dyn GLES,
    objc: &mut ObjC,
    mem: &Mem,
    layer: id,
    origin: CGPoint,
    clip_to: CGRect,
    opacity: CGFloat,
    scale_hack: u32,
    fb_height: u32,
) {
    // TODO: this can't handle zPosition, non-AABB layer transforms, rounded
    // corners, and many other things, but none of these are supported yet :)
    // TODO: back-to-front drawing is not efficient, could we use front-to-back?

    let host_obj = objc.borrow::<CALayerHostObject>(layer);

    if host_obj.hidden {
        return;
    }

    let opacity = opacity * host_obj.opacity;
    let bounds = host_obj.bounds;
    let absolute_frame = {
        let position = host_obj.position;
        let anchor_point = host_obj.anchor_point;
        CGRect {
            origin: CGPoint {
                x: origin.x + position.x - bounds.size.width * anchor_point.x,
                y: origin.y + position.y - bounds.size.height * anchor_point.y,
            },
            size: bounds.size,
        }
    };
    let absolute_frame_clipped = clip_rects(clip_to, absolute_frame);

    // Draw background color, if any
    let have_background = if host_obj.background_color == nil {
        false
    } else {
        let (r, g, b, a) = ui_color::get_rgba(objc, host_obj.background_color);
        // TODO: fully support alpha transparency for backgrounds
        if a == 0.0 || opacity == 0.0 {
            false
        } else {
            gles.ClearColor(r * opacity, g * opacity, b * opacity, a * opacity);
            let (x, y, w, h) = gl_rect_from_cg_rect(absolute_frame_clipped, scale_hack, fb_height);
            gles.Scissor(x, y, w, h);
            gles.Clear(gles11::COLOR_BUFFER_BIT);
            true
        }
    };

    // re-borrow mutably
    let host_obj = objc.borrow_mut::<CALayerHostObject>(layer);

    let need_texture = host_obj.presented_pixels.is_some()
        || host_obj.contents != nil
        || host_obj.cg_context.is_some();
    let need_update = need_texture && !host_obj.gles_texture_is_up_to_date;

    if need_texture {
        if let Some(texture) = host_obj.gles_texture {
            gles.BindTexture(gles11::TEXTURE_2D, texture);
        } else {
            assert!(!host_obj.gles_texture_is_up_to_date);
            let mut texture = 0;
            gles.GenTextures(1, &mut texture);
            gles.BindTexture(gles11::TEXTURE_2D, texture);
            host_obj.gles_texture = Some(texture);
        }
    }

    // Update texture with CAEAGLLayer pixels (slow path), if any
    if need_update {
        if let Some((ref mut pixels, width, height)) = host_obj.presented_pixels {
            // The pixels are always RGBA, but if the layer is opaque then the
            // alpha channel is meant to be ignored. glTexImage2D() has no
            // option to ignore it, so let's manually set them to 255.
            if host_obj.opaque {
                let mut i = 3;
                while i < pixels.len() {
                    pixels[i] = 255;
                    i += 4;
                }
            }

            upload_rgba8_pixels(gles, pixels, (width, height));
        }
    }

    // re-borrow immutably
    let host_obj = objc.borrow::<CALayerHostObject>(layer);

    // Update texture with CGImageRef or CGContextRef pixels, if any
    if need_update {
        if host_obj.contents != nil {
            let image = cg_image::borrow_image(objc, host_obj.contents);

            // No special handling for opacity is needed here: the alpha channel
            // on an image is meaningful and won't be ignored.
            upload_rgba8_pixels(gles, image.pixels(), image.dimensions());
        } else if let Some(cg_context) = host_obj.cg_context {
            // Make sure this is in sync with the code in ca_layer.rs that
            // sets up the context!
            let (width, height, data) = cg_bitmap_context::get_data(objc, cg_context);
            let size = width * height * 4;
            let pixels = mem.bytes_at(data.cast(), size);
            upload_rgba8_pixels(gles, pixels, (width, height));
        }
    }

    // re-borrow mutably
    let host_obj = objc.borrow_mut::<CALayerHostObject>(layer);

    if need_update {
        host_obj.gles_texture_is_up_to_date = true;
    }

    // Draw texture, if any
    if need_texture {
        gles.Color4f(opacity, opacity, opacity, opacity);
        if opacity == 1.0 && host_obj.opaque && !have_background {
            gles.Disable(gles11::BLEND);
        } else {
            gles.Enable(gles11::BLEND);
            gles.BlendFunc(gles11::ONE, gles11::ONE_MINUS_SRC_ALPHA);
        }

        let (x, y, w, h) = gl_rect_from_cg_rect(absolute_frame_clipped, scale_hack, fb_height);
        gles.Scissor(x, y, w, h);
        gles.Viewport(x, y, w, h);

        gles.BindBuffer(gles11::ARRAY_BUFFER, 0);
        let vertices: [f32; 12] = [
            -1.0, -1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0, 1.0, 1.0, 1.0,
        ];
        gles.EnableClientState(gles11::VERTEX_ARRAY);
        gles.VertexPointer(2, gles11::FLOAT, 0, vertices.as_ptr() as *const GLvoid);

        // Normal images will have top-to-bottom row order, but OpenGL ES
        // expects bottom-to-top, so flip the UVs in that case.
        let tex_coords: [f32; 12] = if host_obj.contents != nil {
            [0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0]
        };
        gles.EnableClientState(gles11::TEXTURE_COORD_ARRAY);
        gles.TexCoordPointer(2, gles11::FLOAT, 0, tex_coords.as_ptr() as *const GLvoid);
        gles.Enable(gles11::TEXTURE_2D);
        gles.DrawArrays(gles11::TRIANGLES, 0, 6);
    }

    // avoid holding mutable borrow while recursing
    let sublayers = std::mem::take(&mut host_obj.sublayers);
    for &child_layer in &sublayers {
        composite_layer_recursive(
            gles,
            objc,
            mem,
            child_layer,
            /* origin: */
            CGPoint {
                x: absolute_frame.origin.x - bounds.origin.x,
                y: absolute_frame.origin.y - bounds.origin.y,
            },
            // TODO: clipping goes here (when masksToBounds is implemented)
            clip_to,
            opacity,
            scale_hack,
            fb_height,
        )
    }
    objc.borrow_mut::<CALayerHostObject>(layer).sublayers = sublayers;
}

unsafe fn upload_rgba8_pixels(gles: &mut dyn GLES, pixels: &[u8], dimensions: (u32, u32)) {
    gles.TexImage2D(
        gles11::TEXTURE_2D,
        0,
        gles11::RGBA as _,
        dimensions.0 as _,
        dimensions.1 as _,
        0,
        gles11::RGBA,
        gles11::UNSIGNED_BYTE,
        pixels.as_ptr() as *const _,
    );
    gles.TexParameteri(
        gles11::TEXTURE_2D,
        gles11::TEXTURE_MIN_FILTER,
        gles11::LINEAR as _,
    );
    gles.TexParameteri(
        gles11::TEXTURE_2D,
        gles11::TEXTURE_MAG_FILTER,
        gles11::LINEAR as _,
    );
}

fn clip_rects(a_clip: CGRect, b_clip: CGRect) -> CGRect {
    let a_x1 = a_clip.origin.x;
    let a_y1 = a_clip.origin.y;
    let a_x2 = a_x1 + a_clip.size.width;
    let a_y2 = a_y1 + a_clip.size.height;

    let b_x1 = b_clip.origin.x;
    let b_y1 = b_clip.origin.y;
    let b_x2 = b_x1 + b_clip.size.width;
    let b_y2 = b_y1 + b_clip.size.height;

    let x1 = b_x1.max(a_x1);
    let y1 = b_y1.max(a_y1);
    let x2 = b_x2.min(a_x2);
    let y2 = b_y2.min(a_y2);
    CGRect {
        origin: CGPoint { x: x1, y: y1 },
        size: CGSize {
            width: (x2 - x1).max(0.0),
            height: (y2 - y1).max(0.0),
        },
    }
}

fn gl_rect_from_cg_rect(
    rect: CGRect,
    scale_hack: u32,
    fb_height: u32,
) -> (GLint, GLint, GLint, GLint) {
    let x = (rect.origin.x * scale_hack as f32).round() as GLint;
    let y = (rect.origin.y * scale_hack as f32).round() as GLint;
    let w = (rect.size.width * scale_hack as f32).round() as GLint;
    let h = (rect.size.height * scale_hack as f32).round() as GLint;
    // y points up in OpenGL ES, but down in UIKit and Core Animation
    (x, fb_height as GLint - h - y, w, h)
}
