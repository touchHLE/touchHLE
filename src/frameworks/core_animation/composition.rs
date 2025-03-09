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
#![allow(clippy::zero_ptr)] // alas, as you know, opengl

use super::ca_eagl_layer::find_fullscreen_eagl_layer;
use super::ca_layer::CALayerHostObject;
use crate::frameworks::core_animation::animation;
use crate::frameworks::core_graphics::cg_color::CGColorHostObject;
use crate::frameworks::core_graphics::{cg_bitmap_context, cg_image, CGFloat, CGRect};
use crate::gles::gles11_raw as gles11; // constants only
use crate::gles::gles11_raw::types::*;
use crate::gles::present::{present_frame, FpsCounter};
use crate::gles::GLES; // constants only
use crate::image::Image;
use crate::matrix::Matrix;
use crate::mem::SafeWrite;
use crate::objc::{id, msg, msg_class, nil, ObjC};
use crate::Environment;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct State {
    texture_framebuffer: Option<(GLuint, GLuint)>,
    recomposite_next: Option<Instant>,
    fps_counter: Option<FpsCounter>,
    misc_gl_objects: Option<MiscGlObjects>,
}

struct MiscGlObjects {
    /// Texture containing a single rounded corner.
    rounded_corner_texture: GLuint,
    /// [BASIC_SQUARE_POINTS], used as both vertex and texture co-ords for
    /// drawing simple textured quads.
    basic_square_buffer: GLuint,
    /// [FLIPPED_SQUARE_POINTS], used as texture co-ords for some textured
    /// quads.
    flipped_square_buffer: GLuint,
    /// 9-patch rounded corner texture co-ords (always the same).
    rounded_vertex_buffer: GLuint,
    /// 9-patch rounded corner vertex co-ords (varies with ratio of corner
    /// radius to overall rectangle size).
    rounded_tex_coord_buffer: GLuint,
    /// Index buffer for 9-patch (first 6 elements can be used for square).
    index_buffer: GLuint,
}

unsafe fn load_matrix(gles: &mut dyn GLES, matrix: Matrix<4>) {
    gles.LoadMatrixf(matrix.columns().as_ptr() as *const _);
}

/// For use by `NSRunLoop`: call this 60 times per second. Composites the app's
/// visible layers (i.e. UI) and presents it to the screen. Does nothing if
/// composition isn't in use or it's too soon (the latter check is skipped if
/// `force` is set to [true]).
///
/// Returns the time a recomposite is due, if any.
pub fn recomposite_if_necessary(env: &mut Environment, force: bool) -> Option<Instant> {
    let mut animation_state = animation::State::default();
    let windows = env.framework_state.uikit.ui_view.ui_window.windows.clone();
    if !windows.iter().any(|&window| !msg![env; window isHidden]) {
        log_dbg!("No visible windows, skipping composition");
        return None;
    }

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
        if !force && recomposite_next > now {
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

    let window_layers: Vec<id> = windows
        .into_iter()
        .map(|window| {
            let layer: id = msg![env; window layer];
            // Ensure layer bitmaps are up to date.
            display_layers(env, layer);
            layer
        })
        .collect();

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
    let cumulative_transform = Matrix::<4>::identity();
    let opacity = 1.0;

    let window = env.window.as_mut().unwrap();
    let mut gles = window.make_internal_gl_ctx_current();

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

    // Set up various other GL objects that will be reused on every frame.
    let misc_gl_objects = env
        .framework_state
        .core_animation
        .composition
        .misc_gl_objects
        .get_or_insert_with(|| {
            let dimension = 512usize; // way larger than any reasonable corner
            let mut image = Image::from_pixel_vec(
                vec![255u8; dimension * dimension * 4],
                (dimension as _, dimension as _),
            );
            image.round_corners(dimension as _, /* four_corners: */ false, /* add_sheen: */ false);

            let mut rounded_corner_texture = 0;
            unsafe {
                gles.GenTextures(1, &mut rounded_corner_texture);
                gles.BindTexture(gles11::TEXTURE_2D, rounded_corner_texture);
                // GENERATE_MIPMAP must be set before the texture upload.
                gles.TexParameteri(
                    gles11::TEXTURE_2D,
                    gles11::GENERATE_MIPMAP,
                    gles11::TRUE as _,
                );
                upload_rgba8_pixels(gles.as_mut(), image.pixels(), (dimension as _, dimension as _));
                gles.TexParameteri(
                    gles11::TEXTURE_2D,
                    gles11::TEXTURE_MIN_FILTER,
                    gles11::LINEAR_MIPMAP_LINEAR as _,
                );
                gles.TexParameteri(
                    gles11::TEXTURE_2D,
                    gles11::TEXTURE_WRAP_S,
                    gles11::CLAMP_TO_EDGE as _,
                );
                gles.TexParameteri(
                    gles11::TEXTURE_2D,
                    gles11::TEXTURE_WRAP_T,
                    gles11::CLAMP_TO_EDGE as _,
                );
            }

            let [basic_square_buffer, flipped_square_buffer, rounded_vertex_buffer, rounded_tex_coord_buffer, index_buffer] = unsafe {
                let mut array_buffers = [0; 5];
                gles.GenBuffers(5, array_buffers.as_mut_ptr());
                array_buffers
            };
            unsafe {
                gles.BindBuffer(gles11::ARRAY_BUFFER, basic_square_buffer);
                upload_slice(gles.as_mut(), gles11::ARRAY_BUFFER, &BASIC_SQUARE_POINTS, gles11::STATIC_DRAW);
                gles.BindBuffer(gles11::ARRAY_BUFFER, flipped_square_buffer);
                upload_slice(gles.as_mut(), gles11::ARRAY_BUFFER, &FLIPPED_SQUARE_POINTS, gles11::STATIC_DRAW);
                gles.BindBuffer(gles11::ARRAY_BUFFER, rounded_vertex_buffer);
                upload_slice(gles.as_mut(), gles11::ARRAY_BUFFER, &[0f32; FLOATS_PER_9PATCH], gles11::DYNAMIC_DRAW);
                gles.BindBuffer(gles11::ARRAY_BUFFER, rounded_tex_coord_buffer);
                upload_slice(
                    gles.as_mut(),
                    gles11::ARRAY_BUFFER,
                    &make_9patch_coords([0.0, 1.0, 1.0, 0.0], [0.0, 1.0, 1.0, 0.0]),
                    gles11::STATIC_DRAW,
                );
                // Prevent accidental subsequent use.
                gles.BindBuffer(gles11::ARRAY_BUFFER, 0);

                gles.BindBuffer(gles11::ELEMENT_ARRAY_BUFFER, index_buffer);
                upload_slice(gles.as_mut(), gles11::ELEMENT_ARRAY_BUFFER, &make_9patch_indices(), gles11::STATIC_DRAW);
                // Prevent accidental subsequent use.
                gles.BindBuffer(gles11::ELEMENT_ARRAY_BUFFER, 0);
            }

            MiscGlObjects {
                rounded_corner_texture,
                basic_square_buffer,
                flipped_square_buffer,
                rounded_vertex_buffer,
                rounded_tex_coord_buffer,
                index_buffer,
            }
        });

    // Clear the framebuffer and set up state to prepare for rendering
    unsafe {
        gles.Viewport(0, 0, fb_width as _, fb_height as _);
        gles.ClearColor(0.0, 0.0, 0.0, 1.0);
        gles.Clear(gles11::COLOR_BUFFER_BIT);
        gles.Color4f(1.0, 1.0, 1.0, 1.0);

        gles.MatrixMode(gles11::PROJECTION);
        // Scale down screen-space to normalized device co-ordinates, shift the
        // origin to be at the top-left rather than the center, and flip the
        // Y axis (OpenGL's points up, Core Animation's points down).
        // Using the projection matrix for this is more convenient than adding
        // an extra multiply to composite_layer_recursive.
        load_matrix(
            gles.as_mut(),
            Matrix::from(&Matrix::scale_2d(
                2.0 / screen_bounds.size.width,
                -2.0 / screen_bounds.size.height,
            ))
            .multiply(&Matrix::translate_3d(-1.0, 1.0, 0.0)),
        );
        gles.MatrixMode(gles11::MODELVIEW);
        gles.LoadIdentity();

        // One index buffer to rule them all
        gles.BindBuffer(gles11::ELEMENT_ARRAY_BUFFER, misc_gl_objects.index_buffer);
    }
    std::mem::drop(gles);

    // Assumes the windows in the list are ordered back-to-front.
    // TODO: this may not be correct once we support windowLevel.
    for root_layer in window_layers {
        // Here's where the actual drawing happens
        unsafe {
            composite_layer_recursive(
                env,
                &mut animation_state,
                root_layer,
                cumulative_transform,
                opacity,
            );
        }
    }

    // Re-borrow
    let window = env.window.as_mut().unwrap();
    let mut gles = window.make_internal_gl_ctx_current();

    // Clean up some GL state
    unsafe {
        gles.Viewport(0, 0, fb_width as _, fb_height as _);
        gles.Color4f(1.0, 1.0, 1.0, 1.0);
        gles.Disable(gles11::BLEND);
        gles.MatrixMode(gles11::PROJECTION);
        gles.LoadIdentity();
        gles.MatrixMode(gles11::MODELVIEW);
        gles.LoadIdentity();
        gles.BindBuffer(gles11::ARRAY_BUFFER, 0);
        gles.BindBuffer(gles11::ELEMENT_ARRAY_BUFFER, 0);
        assert_eq!(gles.GetError(), 0);
    }

    // Present our rendered frame (bound to TEXTURE_2D). This copies it to the
    // default framebuffer (0) so we need to unbind our internal framebuffer.
    unsafe {
        gles.BindTexture(gles11::TEXTURE_2D, texture);
        gles.BindFramebufferOES(gles11::FRAMEBUFFER_OES, 0);
        present_frame(
            gles.as_mut(),
            present_frame_args.0,
            present_frame_args.1,
            present_frame_args.2,
        );
    }
    std::mem::drop(gles);
    window.swap_window();

    animation_state.update_started_and_finished_animations(env);

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
    env: &mut Environment,
    animation_state: &mut animation::State,
    layer: id,
    cumulative_transform: Matrix<4>,
    opacity: CGFloat,
) {
    // TODO: this can't handle zPosition among other things, but it is not
    //       supported yet :)
    // TODO: back-to-front drawing is not efficient, could we use front-to-back?

    // This is both acting as the presentationLayer and the private render layer
    // It might need to be reworked in the future into a guest presentationLayer
    let host_obj = animation_state.create_presentation_layer(env, layer);

    if host_obj.hidden {
        return;
    }

    let window = env.window.as_mut().unwrap();
    let mut gles = window.make_internal_gl_ctx_current();

    let opacity = opacity * host_obj.opacity;
    let cumulative_transform = {
        let CALayerHostObject { bounds, .. } = host_obj;

        // Update the transform to match this layer's co-ordinate space.
        let cumulative_transform =
            <Matrix<4> as From<_>>::from(host_obj.superlayer_to_layer_transform())
                .multiply(&cumulative_transform);

        // Reposition and scale the unit quad (see ARRAY_BUFFER binding)
        // so it will have the right size in this layer's co-ordinate space.
        gles.MatrixMode(gles11::MODELVIEW);
        load_matrix(
            gles.as_mut(),
            Matrix::<4>::from(&Matrix::scale_2d(bounds.size.width, bounds.size.height))
                .multiply(&Matrix::translate_3d(bounds.origin.x, bounds.origin.y, 0.0))
                .multiply(&cumulative_transform),
        );

        cumulative_transform
    };

    // Draw background color, if any
    let have_background = if let Some(background_color) = host_obj.background_color {
        let misc = env
            .framework_state
            .core_animation
            .composition
            .misc_gl_objects
            .as_ref()
            .unwrap();

        let CGColorHostObject { r, g, b, a, .. } = background_color;
        gles.Color4f(
            r * a * opacity,
            g * a * opacity,
            b * a * opacity,
            a * opacity,
        );
        gles.Enable(gles11::BLEND);
        gles.BlendFunc(gles11::ONE, gles11::ONE_MINUS_SRC_ALPHA);

        let radius = host_obj.corner_radius;
        if radius == 0.0 {
            gles.Disable(gles11::TEXTURE_2D);
            gles.DisableClientState(gles11::TEXTURE_COORD_ARRAY);

            gles.EnableClientState(gles11::VERTEX_ARRAY);
            gles.BindBuffer(gles11::ARRAY_BUFFER, misc.basic_square_buffer);
            gles.VertexPointer(2, gles11::FLOAT, 0, 0 as *const GLvoid);

            gles.DrawElements(
                gles11::TRIANGLES,
                SQUARE_INDICES.len() as _,
                gles11::UNSIGNED_BYTE,
                0 as *const GLvoid,
            );
        } else {
            gles.Enable(gles11::TEXTURE_2D);
            gles.BindTexture(gles11::TEXTURE_2D, misc.rounded_corner_texture);
            gles.EnableClientState(gles11::TEXTURE_COORD_ARRAY);
            gles.BindBuffer(gles11::ARRAY_BUFFER, misc.rounded_tex_coord_buffer);
            gles.TexCoordPointer(2, gles11::FLOAT, 0, 0 as *const GLvoid);

            gles.EnableClientState(gles11::VERTEX_ARRAY);
            gles.BindBuffer(gles11::ARRAY_BUFFER, misc.rounded_vertex_buffer);
            upload_slice(
                gles.as_mut(),
                gles11::ARRAY_BUFFER,
                &make_9patch_coords(
                    [
                        0.0,
                        (radius / host_obj.bounds.size.width).min(0.5),
                        (1.0 - radius / host_obj.bounds.size.width).max(0.5),
                        1.0,
                    ],
                    [
                        0.0,
                        (radius / host_obj.bounds.size.height).min(0.5),
                        (1.0 - radius / host_obj.bounds.size.height).max(0.5),
                        1.0,
                    ],
                ),
                gles11::DYNAMIC_DRAW,
            );
            gles.VertexPointer(2, gles11::FLOAT, 0, 0 as *const GLvoid);

            gles.DrawElements(
                gles11::TRIANGLES,
                INDICES_PER_9PATCH as _,
                gles11::UNSIGNED_BYTE,
                0 as *const GLvoid,
            );
        };

        true
    } else {
        false
    };

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
            // Update original layer texture
            env.objc.borrow_mut::<CALayerHostObject>(layer).gles_texture = Some(texture);
        }
    }

    // Update original layer texture with CAEAGLLayer pixels (slow path), if any
    if need_update {
        let original_host_obj = env.objc.borrow_mut::<CALayerHostObject>(layer);
        if let Some((ref mut pixels, width, height)) = original_host_obj.presented_pixels {
            // The pixels are always RGBA, but if the layer is opaque then the
            // alpha channel is meant to be ignored. glTexImage2D() has no
            // option to ignore it, so let's manually set them to 255.
            if original_host_obj.opaque {
                let mut i = 3;
                while i < pixels.len() {
                    pixels[i] = 255;
                    i += 4;
                }
            }

            upload_rgba8_pixels(gles.as_mut(), pixels, (width, height));
        }
    }

    // Update texture with CGImageRef or CGContextRef pixels, if any
    if need_update {
        if host_obj.contents != nil {
            let image = cg_image::borrow_image(&env.objc, host_obj.contents);

            // No special handling for opacity is needed here: the alpha channel
            // on an image is meaningful and won't be ignored.
            upload_rgba8_pixels(gles.as_mut(), image.pixels(), image.dimensions());
        } else if let Some(cg_context) = host_obj.cg_context {
            // Make sure this is in sync with the code in ca_layer.rs that
            // sets up the context!
            let (width, height, data) = cg_bitmap_context::get_data(&env.objc, cg_context);
            let size = width * height * 4;
            let pixels = env.mem.bytes_at(data.cast(), size);
            upload_rgba8_pixels(gles.as_mut(), pixels, (width, height));
        }
    }

    if need_update {
        // Update original layer field
        env.objc
            .borrow_mut::<CALayerHostObject>(layer)
            .gles_texture_is_up_to_date = true;
    }

    // Draw texture, if any
    if need_texture {
        let misc = env
            .framework_state
            .core_animation
            .composition
            .misc_gl_objects
            .as_ref()
            .unwrap();

        gles.Color4f(opacity, opacity, opacity, opacity);
        if opacity == 1.0 && host_obj.opaque && !have_background {
            gles.Disable(gles11::BLEND);
        } else {
            gles.Enable(gles11::BLEND);
            gles.BlendFunc(gles11::ONE, gles11::ONE_MINUS_SRC_ALPHA);
        }

        gles.EnableClientState(gles11::VERTEX_ARRAY);
        gles.BindBuffer(gles11::ARRAY_BUFFER, misc.basic_square_buffer);
        gles.VertexPointer(2, gles11::FLOAT, 0, 0 as *const GLvoid);

        gles.EnableClientState(gles11::TEXTURE_COORD_ARRAY);
        // Normal images will have top-to-bottom row order, but OpenGL ES
        // expects bottom-to-top, so flip the UVs in that case.
        gles.BindBuffer(
            gles11::ARRAY_BUFFER,
            if host_obj.contents != nil {
                misc.basic_square_buffer
            } else {
                misc.flipped_square_buffer
            },
        );
        gles.TexCoordPointer(2, gles11::FLOAT, 0, 0 as *const GLvoid);
        gles.Enable(gles11::TEXTURE_2D);
        gles.DrawElements(
            gles11::TRIANGLES,
            SQUARE_INDICES.len() as _,
            gles11::UNSIGNED_BYTE,
            0 as *const GLvoid,
        );
    }
    std::mem::drop(gles);

    // avoid holding mutable borrow while recursing
    let original_host_obj = env.objc.borrow_mut::<CALayerHostObject>(layer);
    for &child_layer in &original_host_obj.sublayers.clone() {
        // TODO: clipping/masksToBounds support
        composite_layer_recursive(
            env,
            animation_state,
            child_layer,
            cumulative_transform,
            opacity,
        )
    }
}

const FLOATS_PER_POINT: usize = 2;
const BASIC_SQUARE_POINTS: [f32; 4 * FLOATS_PER_POINT] = [0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0];
const SQUARE_INDICES: [u8; 6] = [0, 1, 2, 2, 1, 3];
const FLIPPED_SQUARE_POINTS: [f32; 4 * FLOATS_PER_POINT] = [0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0];
const FLOATS_PER_9PATCH: usize = BASIC_SQUARE_POINTS.len() * 3 * 3;
const INDICES_PER_9PATCH: usize = SQUARE_INDICES.len() * 3 * 3;

fn make_9patch_coords(x_edges: [f32; 4], y_edges: [f32; 4]) -> [f32; FLOATS_PER_9PATCH] {
    let mut out_points = [0.0; FLOATS_PER_9PATCH];
    for (i, out_points_chunk) in out_points
        .chunks_exact_mut(BASIC_SQUARE_POINTS.len())
        .enumerate()
    {
        let (x, y) = (i % 3, i / 3);

        for (dst_xy, src_xy) in out_points_chunk
            .chunks_exact_mut(2)
            .zip(BASIC_SQUARE_POINTS.chunks_exact(2))
        {
            let (x1, x2) = (x_edges[x], x_edges[x + 1]);
            let (y1, y2) = (y_edges[y], y_edges[y + 1]);
            dst_xy[0] = x1 + src_xy[0] * (x2 - x1);
            dst_xy[1] = y1 + src_xy[1] * (y2 - y1);
        }
    }
    out_points
}

fn make_9patch_indices() -> [u8; INDICES_PER_9PATCH] {
    let mut out_indices = [0; SQUARE_INDICES.len() * 3 * 3];
    for (i, out_indices_chunk) in out_indices
        .chunks_exact_mut(SQUARE_INDICES.len())
        .enumerate()
    {
        for (out_index, in_index) in out_indices_chunk
            .iter_mut()
            .zip(SQUARE_INDICES.iter().copied())
        {
            *out_index = in_index + i as u8 * (BASIC_SQUARE_POINTS.len() / FLOATS_PER_POINT) as u8;
        }
    }
    out_indices
}

unsafe fn upload_slice<T: SafeWrite>(
    gles: &mut dyn GLES,
    target: GLenum,
    data: &[T],
    usage: GLenum,
) {
    gles.BufferData(
        target,
        std::mem::size_of_val(data) as _,
        data.as_ptr() as *const _,
        usage,
    )
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
