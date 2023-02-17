/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
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
use crate::Options;
use sdl2::mouse::MouseButton;
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use std::collections::VecDeque;
use std::f32::consts::FRAC_PI_2;
use std::num::NonZeroU32;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DeviceOrientation {
    Portrait,
    LandscapeLeft,
    LandscapeRight,
}
fn size_for_orientation(orientation: DeviceOrientation, scale_hack: NonZeroU32) -> (u32, u32) {
    let scale_hack = scale_hack.get();
    match orientation {
        DeviceOrientation::Portrait => (320 * scale_hack, 480 * scale_hack),
        DeviceOrientation::LandscapeLeft => (480 * scale_hack, 320 * scale_hack),
        DeviceOrientation::LandscapeRight => (480 * scale_hack, 320 * scale_hack),
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
    scale_hack: NonZeroU32,
    splash_image_and_gl_ctx: Option<(Image, GLContext)>,
    device_orientation: DeviceOrientation,
    app_gl_ctx_no_longer_current: bool,
    controller_ctx: sdl2::GameControllerSubsystem,
    controllers: Vec<sdl2::controller::GameController>,
    virtual_cursor_last: Option<(f32, f32, bool, bool)>,
}
impl Window {
    pub fn new(
        title: &str,
        icon: Option<Image>,
        launch_image: Option<Image>,
        options: &Options,
    ) -> Window {
        let sdl_ctx = sdl2::init().unwrap();
        let video_ctx = sdl_ctx.video().unwrap();

        // SDL2 disables the screen saver by default, but iPhone OS enables
        // the idle timer that triggers sleep by default, so we turn it back on
        // here, and then the app can disable it if it wants to.
        video_ctx.enable_screen_saver();

        let scale_hack = options.scale_hack;

        // TODO: some apps specify their orientation in Info.plist, we could use
        // that here.
        let device_orientation = options.initial_orientation;

        let (width, height) = size_for_orientation(device_orientation, scale_hack);
        let mut window = video_ctx
            .window(title, width, height)
            .position_centered()
            .opengl()
            .build()
            .unwrap();

        if let Some(icon) = icon {
            window.set_icon(surface_from_image(&icon));
        }

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

        let controller_ctx = sdl_ctx.game_controller().unwrap();

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
            scale_hack,
            splash_image_and_gl_ctx,
            device_orientation,
            app_gl_ctx_no_longer_current: false,
            controller_ctx,
            controllers: Vec::new(),
            virtual_cursor_last: None,
        };
        if window.splash_image_and_gl_ctx.is_some() {
            window.display_splash();
        }
        window
    }

    /// Poll for events from the OS. This needs to be done reasonably often
    /// (60Hz is probably fine) so that the host OS doesn't consider touchHLE
    /// to be unresponsive. Note that events are not returned by this function,
    /// since we often need to defer actually handling them.
    pub fn poll_for_events(&mut self, options: &Options) {
        fn transform_input_coords(window: &Window, (in_x, in_y): (f32, f32)) -> (f32, f32) {
            let (in_w, in_h) = window.size_in_current_orientation();
            // normalize to unit square centred on origin
            let x = in_x / in_w as f32 - 0.5;
            let y = in_y / in_h as f32 - 0.5;
            // rotate
            let matrix = window.input_rotation_matrix();
            let [x, y] = matrix.transform([x, y]);
            // back to pixels
            let (out_w, out_h) = window.size_unrotated_unscaled();
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
                } => Event::TouchDown(transform_input_coords(self, (x as f32, y as f32))),
                E::MouseMotion {
                    x, y, mousestate, ..
                } if mousestate.left() => {
                    Event::TouchMove(transform_input_coords(self, (x as f32, y as f32)))
                }
                E::MouseButtonUp {
                    x,
                    y,
                    mouse_btn: MouseButton::Left,
                    ..
                } => Event::TouchUp(transform_input_coords(self, (x as f32, y as f32))),
                E::ControllerDeviceAdded { which, .. } => {
                    self.controller_added(which);
                    continue;
                }
                E::ControllerDeviceRemoved { which, .. } => {
                    self.controller_removed(which);
                    continue;
                }
                // Virtual cursor handling only. Accelerometer handling uses
                // polling.
                E::ControllerButtonUp { .. }
                | E::ControllerButtonDown { .. }
                | E::ControllerAxisMotion { .. } => {
                    let (new_x, new_y, new_pressed, visible) = self.get_virtual_cursor(options);
                    let (old_x, old_y, old_pressed, _) =
                        self.virtual_cursor_last.unwrap_or_default();
                    self.virtual_cursor_last = Some((new_x, new_y, new_pressed, visible));
                    match (old_pressed, new_pressed) {
                        (false, true) => {
                            Event::TouchDown(transform_input_coords(self, (new_x, new_y)))
                        }
                        (true, false) => {
                            Event::TouchUp(transform_input_coords(self, (new_x, new_y)))
                        }
                        _ if (new_x, new_y) != (old_x, old_y) && new_pressed => {
                            Event::TouchMove(transform_input_coords(self, (new_x, new_y)))
                        }
                        _ => continue,
                    }
                }
                _ => continue,
            })
        }
    }

    /// Pop an event from the queue (in FIFO order)
    pub fn pop_event(&mut self) -> Option<Event> {
        self.event_queue.pop_front()
    }

    fn controller_added(&mut self, joystick_idx: u32) {
        let Ok(controller) = self.controller_ctx.open(joystick_idx) else {
            log!("Warning: A new controller was connected, but it couldn't be accessed!");
            return;
        };
        log!(
            "New controller connected: {}. Left stick = device tilt. Right stick = touch input (press the stick or shoulder button to tap/hold).",
            controller.name()
        );
        self.controllers.push(controller);
    }
    fn controller_removed(&mut self, instance_id: u32) {
        let Some(idx) = self.controllers.iter().position(|controller| controller.instance_id() == instance_id) else {
            return;
        };
        let controller = self.controllers.remove(idx);
        log!("Warning: Controller disconnected: {}", controller.name());
    }
    pub fn print_accelerometer_notice(&self) {
        log!("This app uses the accelerometer.");
        if self.controllers.is_empty() {
            log!("Connect a controller to get accelerometer simulation.");
        } else {
            log!("Your connected controller's left analog stick will be used for accelerometer simulation.");
        }
    }

    /// Get the real (TODO) or simulated accelerometer output.
    /// See also [crate::frameworks::uikit::ui_accelerometer].
    pub fn get_acceleration(&self, options: &Options) -> (f32, f32, f32) {
        // Get left analog stick input. The range is [-1, 1] on each axis.
        let (x, y, _) = self.get_controller_stick(options, true);

        // Correct for window rotation
        let [x, y] = self.input_rotation_matrix().transform([x, y]);
        let (x, y) = (x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0)); // just in case

        // Let's simulate tilting the device based on the analog stick inputs.
        //
        // If an iPhone is lying flat on its back, level with the ground, and it
        // is on Earth, the accelerometer will report approximately (0, 0, -1).
        // The acceleration x and y axes are aligned with the screen's x and y
        // axes. +x points to the right of the screen, +y points to the top of
        // the screen, and +z points away from the screen. In the example
        // scenario, the z axis is parallel to gravity.

        let gravity: [f32; 3] = [0.0, 0.0, -1.0];

        let neutral_x = options.x_tilt_offset.to_radians();
        let neutral_y = options.y_tilt_offset.to_radians();
        let x_rotation_range = options.x_tilt_range.to_radians() / 2.0;
        let y_rotation_range = options.y_tilt_range.to_radians() / 2.0;
        // (x, y) are swapped and inverted because the controller Y axis usually
        // corresponds to forward/backward movement, but rotating about the Y
        // axis means tilting the device left/right, and gravity points in the
        // opposite direction of the device's tilt.
        let x_rotation = neutral_x - x_rotation_range * y;
        let y_rotation = neutral_y - y_rotation_range * x;

        let matrix =
            Matrix::<3>::y_rotation(y_rotation).multiply(&Matrix::<3>::x_rotation(x_rotation));
        let [x, y, z] = matrix.transform(gravity);

        (x, y, z)
    }

    /// For use when redrawing the screen: Get the cached on-screen position and
    /// press state of the analog stick-controlled virtual cursor, if it is
    /// visible.
    pub fn virtual_cursor_visible_at(&self) -> Option<(f32, f32, bool)> {
        let (x, y, pressed, visible) = self.virtual_cursor_last?;
        if visible {
            Some((x, y, pressed))
        } else {
            None
        }
    }

    /// Get the new  on-screen position, click state and visibility of the
    /// analog stick-controlled virtual cursor.
    fn get_virtual_cursor(&self, options: &Options) -> (f32, f32, bool, bool) {
        // Get right analog stick input. The range is [-1, 1] on each axis.
        let (x, y, pressed) = self.get_controller_stick(options, false);

        // The cursor is intended to only show up once you move the analog stick
        // out of its deadzone, or while the button is held.
        let visible = pressed || x != 0.0 || y != 0.0;

        // Though the analog stick output fits within a square, its actual range
        // is usually a circle enclosed by the square. So we need to cut out
        // a square within that circle.
        let (x, y) = {
            let limit = std::f32::consts::FRAC_PI_4.sin();
            let x_abs = x.abs().min(limit) / limit;
            let y_abs = y.abs().min(limit) / limit;
            (x_abs.copysign(x), y_abs.copysign(y))
        };

        // Aspect ratio handling: cut the square down to a rectangle
        // TODO: It would be better to directly cut out a rectangle from the
        // circle.
        let (width, height) = self.size_in_current_orientation();
        let (width, height) = (width as f32, height as f32);
        let (x, y) = {
            let (x_abs, y_abs) = if width < height {
                (x.abs().min(width / height) / (width / height), y.abs())
            } else {
                (x.abs(), y.abs().min(height / width) / (height / width))
            };
            (x_abs.copysign(x), y_abs.copysign(y))
        };

        // Convert to window co-ordinates
        let x = (x / 2.0 + 0.5) * width;
        let y = (y / 2.0 + 0.5) * height;

        (x, y, pressed, visible)
    }

    /// Get the summed X and Y positions and button state of the left or right
    /// analog stick of the game controllers. Each axis value is in the range
    /// [-1, 1].
    fn get_controller_stick(&self, options: &Options, left: bool) -> (f32, f32, bool) {
        fn convert_axis(axis: i16, deadzone: f32) -> f32 {
            assert!(deadzone >= 0.0);
            let axis = ((axis as f32) / (i16::MAX as f32)).clamp(-1.0, 1.0);
            let abs_axis = (axis.abs().max(deadzone) - deadzone) / (1.0 - deadzone);
            abs_axis.copysign(axis)
        }

        let (mut x, mut y) = (0.0, 0.0);
        let mut pressed = false;
        for controller in &self.controllers {
            use sdl2::controller::{Axis, Button};
            let (x_axis, y_axis, button1, button2) = if left {
                (
                    Axis::LeftX,
                    Axis::LeftY,
                    Button::LeftStick,
                    Button::LeftShoulder,
                )
            } else {
                (
                    Axis::RightX,
                    Axis::RightY,
                    Button::RightStick,
                    Button::RightShoulder,
                )
            };
            x += convert_axis(controller.axis(x_axis), options.deadzone);
            y += convert_axis(controller.axis(y_axis), options.deadzone);
            pressed |= controller.button(button1);
            pressed |= controller.button(button2);
        }
        let (x, y) = (x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0));

        (x, y, pressed)
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

        let (width, height) = size_for_orientation(new_orientation, self.scale_hack);

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
    /// rotation (see [Self::rotate_device]). This also has the scale hack
    /// applied.
    pub fn size_in_current_orientation(&self) -> (u32, u32) {
        size_for_orientation(self.device_orientation, self.scale_hack)
    }

    /// Get the size in pixels of the window without rotation or scaling.
    pub fn size_unrotated_unscaled(&self) -> (u32, u32) {
        size_for_orientation(DeviceOrientation::Portrait, NonZeroU32::new(1).unwrap())
    }

    /// Get the size in pixels of the window without rotation but with the
    /// scale hack.
    pub fn size_unrotated_scalehacked(&self) -> (u32, u32) {
        size_for_orientation(DeviceOrientation::Portrait, self.scale_hack)
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
            DeviceOrientation::LandscapeRight => Matrix::z_rotation(FRAC_PI_2),
        }
    }

    /// Transformation matrix for touch inputs received by the window. Rotates
    /// them to match the app. See [Self::rotate_device].
    pub fn input_rotation_matrix(&self) -> Matrix<2> {
        match self.device_orientation {
            DeviceOrientation::Portrait => Matrix::identity(),
            DeviceOrientation::LandscapeLeft => Matrix::z_rotation(FRAC_PI_2),
            DeviceOrientation::LandscapeRight => Matrix::z_rotation(-FRAC_PI_2),
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
