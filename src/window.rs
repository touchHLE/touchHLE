//! Abstraction of window setup, OpenGL context creation and event handling.
//! Also provides OpenGL bindings.
//!
//! Implemented using the sdl2 crate (a Rust wrapper for SDL2). All usage of
//! SDL should be confined to this module.
//!
//! There is currently no separation of concerns between a single window and
//! window system interaction in general, because it is assumed only one window
//! will be needed for the runtime of the app.

mod gl;

pub use gl::{gl21compat, gl32core, gles11, GLContext, GLVersion};

use crate::image::Image;
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;

pub enum Event {
    Quit,
}

fn surface_from_image(image: &Image) -> Surface {
    let src_pixels = image.pixels();
    let (width, height) = image.dimensions();

    let mut surface = Surface::new(width, height, PixelFormatEnum::RGBA32).unwrap();
    let (width, height) = (width as usize, height as usize);
    let pitch = surface.pitch() as usize;
    surface.with_lock_mut(|dst_pixels| {
        for y in 0..(height as usize) {
            for x in 0..(width as usize) {
                for channel in 0..4 {
                    let src_idx = y * width * 4 + x * 4 + channel;
                    let dst_idx = y * pitch + x * 4 + channel;
                    dst_pixels[dst_idx] = src_pixels[src_idx];
                }
            }
        }
    });
    surface
}

pub struct Window {
    _sdl_ctx: sdl2::Sdl,
    video_ctx: sdl2::VideoSubsystem,
    window: sdl2::video::Window,
    event_pump: sdl2::EventPump,
    _splash_gl_ctx: Option<GLContext>,
}
impl Window {
    pub fn new(title: &str, icon: Image, launch_image: Option<Image>) -> Window {
        let sdl_ctx = sdl2::init().unwrap();
        let video_ctx = sdl_ctx.video().unwrap();

        let mut window = video_ctx
            .window(title, 320, 480)
            .position_centered()
            .opengl()
            .build()
            .unwrap();

        window.set_icon(surface_from_image(&icon));

        let event_pump = sdl_ctx.event_pump().unwrap();

        let splash_gl_ctx = if let Some(launch_image) = launch_image {
            // Splash screen must be drawn with OpenGL (or not drawn at all)
            // because otherwise we can't later use OpenGL in the same window.
            // We are not required to use the same OpenGL version as for other
            // contexts in this window, so let's use something relatively modern
            // and compatible. OpenGL 3.2 is the baseline version of OpenGL
            // available on macOS.
            let gl_ctx = gl::create_gl_context(&video_ctx, &window, GLVersion::GL32Core);
            gl::make_gl_context_current(&video_ctx, &window, &gl_ctx);

            unsafe { gl::display_image(&launch_image) };
            window.gl_swap_window();

            Some(gl_ctx) // hold onto GL context so the image doesn't disappear
        } else {
            None
        };

        Window {
            _sdl_ctx: sdl_ctx,
            video_ctx,
            window,
            event_pump,
            _splash_gl_ctx: splash_gl_ctx,
        }
    }

    pub fn poll_for_events(&mut self, events: &mut Vec<Event>) {
        for event in self.event_pump.poll_iter() {
            use sdl2::event::Event as E;
            #[allow(clippy::single_match)]
            match event {
                E::Quit { .. } => events.push(Event::Quit),
                _ => (),
            }
        }
    }

    pub fn create_gl_context(&mut self, version: GLVersion) -> GLContext {
        gl::create_gl_context(&self.video_ctx, &self.window, version)
    }

    pub fn make_gl_context_current(&mut self, gl_ctx: &GLContext) {
        gl::make_gl_context_current(&self.video_ctx, &self.window, gl_ctx);
    }
}
