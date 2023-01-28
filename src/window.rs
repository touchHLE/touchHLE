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
pub use matrix::Matrix;

use crate::image::Image;
use sdl2::mouse::MouseButton;
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use std::collections::VecDeque;
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

#[derive(Debug)]
pub enum Event {
    Quit,
    TouchDown((f32, f32)),
    TouchMove((f32, f32)),
    TouchUp((f32, f32)),
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
    event_queue: VecDeque<Event>,
    #[cfg(target_os = "macos")]
    max_height: u32,
    #[cfg(target_os = "macos")]
    viewport_y_offset: u32,
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
            event_queue: VecDeque::new(),
            #[cfg(target_os = "macos")]
            max_height: height,
            #[cfg(target_os = "macos")]
            viewport_y_offset: 0,
            splash_image_and_gl_ctx,
            device_orientation: DeviceOrientation::Portrait,
            app_gl_ctx_no_longer_current: false,
        };
        window.display_splash();
        window
    }

    /// Poll for events from the OS. This needs to be done reasonably often
    /// (60Hz is probably fine) so that the host OS doesn't consider touchHLE
    /// to be unresponsive. Note that events are not returned by this function,
    /// since we often need to defer actually handling them.
    pub fn poll_for_events(&mut self) {
        fn transform_input_coords(window: &Window, (in_x, in_y): (i32, i32)) -> (f32, f32) {
            let (in_w, in_h) = window.size_in_current_orientation();
            // normalize to unit square centred on origin
            let x = in_x as f32 / in_w as f32 - 0.5;
            let y = in_y as f32 / in_h as f32 - 0.5;
            // rotate
            let matrix = window.input_rotation_matrix();
            let [x, y] = matrix.transform([x, y]);
            // back to pixels
            let (out_w, out_h) = window.size_unrotated();
            let out_x = (x + 0.5) * out_w as f32;
            let out_y = (y + 0.5) * out_h as f32;
            (out_x, out_y)
        }

        while let Some(event) = self.event_pump.poll_event() {
            use sdl2::event::Event as E;
            self.event_queue.push_back(match event {
                E::Quit { .. } => Event::Quit,
                // TODO: support for real touch inputs and multi-touch
                E::MouseButtonDown {
                    x,
                    y,
                    mouse_btn: MouseButton::Left,
                    ..
                } => Event::TouchDown(transform_input_coords(self, (x, y))),
                E::MouseMotion {
                    x, y, mousestate, ..
                } if mousestate.left() => Event::TouchMove(transform_input_coords(self, (x, y))),
                E::MouseButtonUp {
                    x,
                    y,
                    mouse_btn: MouseButton::Left,
                    ..
                } => Event::TouchUp(transform_input_coords(self, (x, y))),
                _ => continue,
            })
        }
    }

    /// Pop an event from the queue (in FIFO order)
    pub fn pop_event(&mut self) -> Option<Event> {
        self.event_queue.pop_front()
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

        let matrix = self.output_rotation_matrix();
        let viewport_size = self.size_in_current_orientation();
        let viewport_offset = (0, self.viewport_y_offset());

        self.app_gl_ctx_no_longer_current = true;

        gl::make_gl_context_current(&self.video_ctx, &self.window, gl_ctx);
        unsafe { gl::display_image(image, viewport_offset, viewport_size, &matrix) };
        self.window.gl_swap_window();

        // hold onto GL context so the image doesn't disappear, and hold
        // onto image so we can rotate later if necessary
    }

    /// Swap front-buffer and back-buffer so the result of OpenGL rendering is
    /// presented.
    pub fn swap_window(&mut self) {
        self.window.gl_swap_window();
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

        // macOS quirk: when resizing the window, the new framebuffer's size is
        // apparently max(new_size, old_size) in each dimension, but the
        // viewport is positioned wrong on the y axis for some reason, so we
        // need to apply an offset.
        // Recreating the OpenGL context was an alternative workaround, but that
        // apparently stops other OpenGL contexts drawing to the framebuffer!
        #[cfg(target_os = "macos")]
        {
            let (_old_width, old_height) = self.window.size();
            self.max_height = self.max_height.max(old_height).max(height);
            self.viewport_y_offset = self.max_height - height;
        }

        self.window.set_size(width, height).unwrap();

        self.device_orientation = new_orientation;

        if self.splash_image_and_gl_ctx.is_some() {
            self.display_splash();
        }
    }

    /// Get the size in pixels of the window with the aspect ratio reflecting
    /// rotation (see [Self::rotate_device]).
    pub fn size_in_current_orientation(&self) -> (u32, u32) {
        size_for_orientation(self.device_orientation)
    }

    /// Get the size in pixels of the window without rotation.
    pub fn size_unrotated(&self) -> (u32, u32) {
        size_for_orientation(DeviceOrientation::Portrait)
    }

    pub fn viewport_y_offset(&self) -> u32 {
        #[cfg(target_os = "macos")]
        return self.viewport_y_offset;
        #[cfg(not(target_os = "macos"))]
        return 0;
    }

    /// Transformation matrix for texture co-ordinates when sampling the
    /// framebuffer presented by the app. Rotates the framebuffer to match the
    /// window. See [Self::rotate_device].
    pub fn output_rotation_matrix(&self) -> Matrix<2> {
        match self.device_orientation {
            DeviceOrientation::Portrait => Matrix::identity(),
            DeviceOrientation::LandscapeLeft => Matrix::z_rotation(-FRAC_PI_2),
        }
    }

    /// Transformation matrix for touch inputs received by the window. Rotates
    /// them to match the app. See [Self::rotate_device].
    pub fn input_rotation_matrix(&self) -> Matrix<2> {
        match self.device_orientation {
            DeviceOrientation::Portrait => Matrix::identity(),
            DeviceOrientation::LandscapeLeft => Matrix::z_rotation(FRAC_PI_2),
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

pub fn open_url(url: &str) {
    let _ = sdl2::url::open_url(url);
}
