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
mod matrix;

pub use gl::{gl21compat, gl32core, gles11, GLContext, GLVersion};
use matrix::Matrix;

use crate::image::Image;
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use std::f32::consts::FRAC_PI_2;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DeviceOrientation {
    Portrait,
    LandscapeLeft,
}
fn size_for_orientation(orientation: DeviceOrientation) -> (u32, u32) {
    match orientation {
        DeviceOrientation::Portrait => (320, 480),
        DeviceOrientation::LandscapeLeft => (480, 320),
    }
}

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
        for y in 0..height {
            for x in 0..width {
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
    splash_image_and_gl_ctx: Option<(Image, GLContext)>,
    device_orientation: DeviceOrientation,
    app_gl_ctx_no_longer_current: bool,
}
impl Window {
    pub fn new(title: &str, icon: Image, launch_image: Option<Image>) -> Window {
        let sdl_ctx = sdl2::init().unwrap();
        let video_ctx = sdl_ctx.video().unwrap();

        // SDL2 disables the screen saver by default, but iPhone OS enables
        // the idle timer that triggers sleep by default, so we turn it back on
        // here, and then the app can disable it if it wants to.
        video_ctx.enable_screen_saver();

        // TODO: some apps specify their orientation in Info.plist, we could use
        // that here.
        let device_orientation = DeviceOrientation::Portrait;

        let (width, height) = size_for_orientation(device_orientation);
        let mut window = video_ctx
            .window(title, width, height)
            .position_centered()
            .opengl()
            .build()
            .unwrap();

        window.set_icon(surface_from_image(&icon));

        let event_pump = sdl_ctx.event_pump().unwrap();

        let splash_image_and_gl_ctx = if let Some(launch_image) = launch_image {
            // Splash screen must be drawn with OpenGL (or not drawn at all)
            // because otherwise we can't later use OpenGL in the same window.
            // We are not required to use the same OpenGL version as for other
            // contexts in this window, so let's use something relatively modern
            // and compatible. OpenGL 3.2 is the baseline version of OpenGL
            // available on macOS.
            let gl_ctx = gl::create_gl_context(&video_ctx, &window, GLVersion::GL32Core);
            Some((launch_image, gl_ctx))
        } else {
            None
        };

        let mut window = Window {
            _sdl_ctx: sdl_ctx,
            video_ctx,
            window,
            event_pump,
            splash_image_and_gl_ctx,
            device_orientation: DeviceOrientation::Portrait,
            app_gl_ctx_no_longer_current: false,
        };
        window.display_splash();
        window
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

    /// Retrieve and reset the flag that indicates if the current OpenGL context
    /// was changed to one outside of the control of the guest app.
    ///
    /// This should be checked before making OpenGL calls on behalf of the guest
    /// app, so its context can be restored.
    pub fn is_app_gl_ctx_no_longer_current(&mut self) -> bool {
        let value = self.app_gl_ctx_no_longer_current;
        self.app_gl_ctx_no_longer_current = false;
        value
    }

    fn display_splash(&mut self) {
        let Some((image, gl_ctx)) = &self.splash_image_and_gl_ctx else {
            panic!();
        };

        let matrix = self.content_rotation_matrix();
        let viewport = size_for_orientation(self.device_orientation);

        self.app_gl_ctx_no_longer_current = true;

        gl::make_gl_context_current(&self.video_ctx, &self.window, gl_ctx);
        unsafe { gl::display_image(image, viewport, &matrix) };
        self.window.gl_swap_window();

        // hold onto GL context so the image doesn't disappear, and hold
        // onto image so we can rotate later if necessary
    }

    /// Consider the emulated device to be rotated to a particular orientation.
    ///
    /// On a PC or laptop, this will make the window be rotated so the app
    /// content appears upright. On a mobile device (TODO), this will do
    /// nothing because the user can physically rotate the screen.
    pub fn rotate_device(&mut self, new_orientation: DeviceOrientation) {
        if new_orientation == self.device_orientation {
            return;
        }

        let (width, height) = size_for_orientation(new_orientation);
        self.window.set_size(width, height).unwrap();

        self.device_orientation = new_orientation;

        if let Some((image, gl_ctx)) = self.splash_image_and_gl_ctx.take() {
            // macOS quirk: resizing the window makes the OpenGL framebuffer be
            // displayed in the wrong part of the window. Recreating the context
            // seems to fix this?
            std::mem::drop(gl_ctx);
            let gl_ctx = gl::create_gl_context(&self.video_ctx, &self.window, GLVersion::GL32Core);
            self.splash_image_and_gl_ctx = Some((image, gl_ctx));

            self.display_splash();
        }
    }

    /// Get a transformation matrix that can be applied to the content presented
    /// by the app to make it appear upright.
    fn content_rotation_matrix(&self) -> Matrix<2> {
        match self.device_orientation {
            DeviceOrientation::Portrait => Matrix::identity(),
            DeviceOrientation::LandscapeLeft => Matrix::z_rotation(-FRAC_PI_2),
        }
    }

    pub fn is_screen_saver_enabled(&self) -> bool {
        self.video_ctx.is_screen_saver_enabled()
    }
    pub fn set_screen_saver_enabled(&mut self, enabled: bool) {
        match enabled {
            true => self.video_ctx.enable_screen_saver(),
            false => self.video_ctx.disable_screen_saver(),
        }
    }
}
