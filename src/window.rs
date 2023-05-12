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
use crate::options::Options;
use sdl2::mouse::MouseButton;
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use std::collections::VecDeque;
use std::env;
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
fn rotate_fullscreen_size(orientation: DeviceOrientation, screen_size: (u32, u32)) -> (u32, u32) {
    let (short_side, long_side) = if screen_size.0 < screen_size.1 {
        (screen_size.0, screen_size.1)
    } else {
        (screen_size.1, screen_size.0)
    };
    match orientation {
        DeviceOrientation::Portrait => (short_side, long_side),
        DeviceOrientation::LandscapeLeft | DeviceOrientation::LandscapeRight => {
            (long_side, short_side)
        }
    }
}
/// Tell SDL2 what orientation we want. Only useful on Android.
fn set_sdl2_orientation(orientation: DeviceOrientation) {
    // Despite the name, this hint works on Android too.
    sdl2::hint::set(
        "SDL_IOS_ORIENTATIONS",
        match orientation {
            DeviceOrientation::Portrait => "Portrait",
            // The inversion is deliberate. These probably correspond to iPhone OS
            // content orientations?
            DeviceOrientation::LandscapeLeft => "LandscapeRight",
            DeviceOrientation::LandscapeRight => "LandscapeLeft",
        },
    );
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

/// Display a message box with custom buttons. Each button has an associated
/// ID, and the ID of the clicked button (if any) will be returned.
pub fn show_message_with_options(
    title: &str,
    message: &str,
    is_error: bool,
    options: &[(i32, &str)],
) -> Option<i32> {
    use sdl2::messagebox::{
        show_message_box, ButtonData, ClickedButton, MessageBoxButtonFlag, MessageBoxFlag,
    };

    let buttons: Vec<_> = options
        .iter()
        .map(|&(button_id, text)| ButtonData {
            flags: MessageBoxButtonFlag::NOTHING,
            button_id,
            text,
        })
        .collect();
    let clicked_button = show_message_box(
        if is_error {
            MessageBoxFlag::ERROR
        } else {
            MessageBoxFlag::INFORMATION
        },
        &buttons,
        title,
        message,
        None,
        None,
    )
    .unwrap();
    match clicked_button {
        ClickedButton::CloseButton => None,
        ClickedButton::CustomButton(data) => Some(data.button_id),
    }
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
    /// Copy of `fullscreen` on [Options]. Note that this is meaningless when
    /// [Self::rotatable_fullscreen] returns [true].
    fullscreen: bool,
    scale_hack: NonZeroU32,
    splash_image_and_gl_ctx: Option<(Image, GLContext)>,
    device_orientation: DeviceOrientation,
    app_gl_ctx_no_longer_current: bool,
    controller_ctx: sdl2::GameControllerSubsystem,
    controllers: Vec<sdl2::controller::GameController>,
    _sensor_ctx: sdl2::SensorSubsystem,
    accelerometer: Option<sdl2::sensor::Sensor>,
    virtual_cursor_last: Option<(f32, f32, bool, bool)>,
}
impl Window {
    /// Returns [true] if touchHLE is running on a device where we should always
    /// display fullscreen, but SDL2 will let us control the orientation, i.e.
    /// Android devices.
    fn rotatable_fullscreen() -> bool {
        env::consts::OS == "android"
    }

    pub fn new(
        title: &str,
        icon: Option<Image>,
        launch_image: Option<Image>,
        options: &Options,
    ) -> Window {
        let sdl_ctx = sdl2::init().unwrap();
        let video_ctx = sdl_ctx.video().unwrap();

        // The "hidapi" feature of rust-sdl2 is enabled so that sdl2::sensor
        // is available, but we don't want to enable SDL's HIDAPI controller
        // drivers because they cause duplicated controllers on macOS
        // (https://github.com/libsdl-org/SDL/issues/7479). Once that's fixed,
        // remove this (https://github.com/hikari-no-yume/touchHLE/issues/85).
        sdl2::hint::set("SDL_JOYSTICK_HIDAPI", "0");

        if env::consts::OS == "android" {
            // It's important to set context version BEFORE window creation
            // ref. https://wiki.libsdl.org/SDL2/SDL_GLattr
            let attr = video_ctx.gl_attr();
            attr.set_context_version(1, 1);
            attr.set_context_profile(sdl2::video::GLProfile::GLES);
        }

        // SDL2 disables the screen saver by default, but iPhone OS enables
        // the idle timer that triggers sleep by default, so we turn it back on
        // here, and then the app can disable it if it wants to.
        video_ctx.enable_screen_saver();

        let scale_hack = options.scale_hack;
        // TODO: some apps specify their orientation in Info.plist, we could use
        // that here.
        let device_orientation = options.initial_orientation;
        let fullscreen = options.fullscreen;

        let mut window = if Self::rotatable_fullscreen() {
            // Without this, SDL will force fullscreen mode to be portrait.
            set_sdl2_orientation(device_orientation);
            let screen_size = video_ctx.display_bounds(0).unwrap().size();
            let (width, height) = rotate_fullscreen_size(device_orientation, screen_size);
            let window = video_ctx
                .window(title, width, height)
                .fullscreen()
                .opengl()
                .build()
                .unwrap();
            window
        } else if fullscreen {
            let (width, height) = video_ctx.display_bounds(0).unwrap().size();
            let window = video_ctx
                .window(title, width, height)
                .fullscreen_desktop()
                .opengl()
                .build()
                .unwrap();
            window
        } else {
            let (width, height) = size_for_orientation(device_orientation, scale_hack);
            let window = video_ctx
                .window(title, width, height)
                .position_centered()
                .opengl()
                .build()
                .unwrap();
            window
        };

        if env::consts::OS == "android" {
            // Sanity check
            let gl_attr = video_ctx.gl_attr();
            debug_assert_eq!(gl_attr.context_profile(), sdl2::video::GLProfile::GLES);
            debug_assert_eq!(gl_attr.context_version(), (1, 1));
        }

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
            match gl::create_gl_context(&video_ctx, &window, GLVersion::GL32Core) {
                Ok(gl_ctx) => Some((launch_image, gl_ctx)),
                Err(err) => {
                    log!("Couldn't create OpenGL context for splash image: {}", err);
                    None
                }
            }
        } else {
            None
        };

        let controller_ctx = sdl_ctx.game_controller().unwrap();

        let sensor_ctx = sdl_ctx.sensor().unwrap();
        let mut accelerometer: Option<sdl2::sensor::Sensor> = None;
        if let Ok(num_sensors) = sensor_ctx.num_sensors() {
            for sensor_idx in 0..num_sensors {
                if let Ok(sensor) = sensor_ctx.open(sensor_idx) {
                    if sensor.sensor_type() == sdl2::sensor::SensorType::Accelerometer {
                        log!("Accelerometer detected: {}.", sensor.name());
                        accelerometer = Some(sensor);
                        break;
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        let max_height = window.size().1;

        let mut window = Window {
            _sdl_ctx: sdl_ctx,
            video_ctx,
            window,
            event_pump,
            event_queue: VecDeque::new(),
            #[cfg(target_os = "macos")]
            max_height,
            #[cfg(target_os = "macos")]
            viewport_y_offset: 0,
            fullscreen,
            scale_hack,
            splash_image_and_gl_ctx,
            device_orientation,
            app_gl_ctx_no_longer_current: false,
            controller_ctx,
            controllers: Vec::new(),
            _sensor_ctx: sensor_ctx,
            accelerometer,
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
        fn transform_input_coords(
            window: &Window,
            (in_x, in_y): (f32, f32),
            independent_of_viewport: bool,
        ) -> (f32, f32) {
            let (vx, vy, vw, vh) = if independent_of_viewport {
                let (width, height) =
                    size_for_orientation(window.device_orientation, NonZeroU32::new(1).unwrap());
                (0, 0, width, height)
            } else {
                window.viewport()
            };
            // normalize to unit square centred on origin
            let x = (in_x - vx as f32) / vw as f32 - 0.5;
            let y = (in_y - vy as f32) / vh as f32 - 0.5;
            // rotate
            let matrix = window.input_rotation_matrix();
            let [x, y] = matrix.transform([x, y]);
            // back to pixels
            let (out_w, out_h) = window.size_unrotated_unscaled();
            let out_x = (x + 0.5) * out_w as f32;
            let out_y = (y + 0.5) * out_h as f32;
            (out_x, out_y)
        }
        fn translate_button(button: sdl2::controller::Button) -> Option<crate::options::Button> {
            match button {
                sdl2::controller::Button::A => Some(crate::options::Button::A),
                sdl2::controller::Button::B => Some(crate::options::Button::B),
                sdl2::controller::Button::X => Some(crate::options::Button::X),
                sdl2::controller::Button::Y => Some(crate::options::Button::Y),
                _ => None,
            }
        }

        let mut controller_updated = false;
        while let Some(event) = self.event_pump.poll_event() {
            use sdl2::event::Event as E;
            self.event_queue.push_back(match event {
                E::Quit { .. } => Event::Quit,
                // TODO: support for multi-touch
                E::MouseButtonDown {
                    x,
                    y,
                    mouse_btn: MouseButton::Left,
                    ..
                } => Event::TouchDown(transform_input_coords(self, (x as f32, y as f32), false)),
                E::MouseMotion {
                    x, y, mousestate, ..
                } if mousestate.left() => {
                    Event::TouchMove(transform_input_coords(self, (x as f32, y as f32), false))
                }
                E::MouseButtonUp {
                    x,
                    y,
                    mouse_btn: MouseButton::Left,
                    ..
                } => Event::TouchUp(transform_input_coords(self, (x as f32, y as f32), false)),
                E::ControllerDeviceAdded { which, .. } => {
                    self.controller_added(which);
                    continue;
                }
                E::ControllerDeviceRemoved { which, .. } => {
                    self.controller_removed(which);
                    continue;
                }
                // Note that accelerometer simulation with analog sticks is
                // handled with polling, rather than being event-based.
                E::ControllerButtonUp { button, .. } | E::ControllerButtonDown { button, .. } => {
                    controller_updated = true;
                    let Some(button) = translate_button(button) else {
                        continue;
                    };
                    let Some(&(x, y)) = options.button_to_touch.get(&button) else {
                        continue;
                    };
                    match event {
                        E::ControllerButtonUp { .. } => {
                            Event::TouchUp(transform_input_coords(self, (x, y), true))
                        }
                        E::ControllerButtonDown { .. } => {
                            Event::TouchDown(transform_input_coords(self, (x, y), true))
                        }
                        _ => unreachable!(),
                    }
                }
                E::ControllerAxisMotion { .. } => {
                    controller_updated = true;
                    continue;
                }
                _ => continue,
            })
        }

        if controller_updated {
            let (new_x, new_y, new_pressed, visible) = self.get_virtual_cursor(options);
            let (old_x, old_y, old_pressed, _) = self.virtual_cursor_last.unwrap_or_default();
            self.virtual_cursor_last = Some((new_x, new_y, new_pressed, visible));
            self.event_queue
                .push_back(match (old_pressed, new_pressed) {
                    (false, true) => {
                        Event::TouchDown(transform_input_coords(self, (new_x, new_y), false))
                    }
                    (true, false) => {
                        Event::TouchUp(transform_input_coords(self, (new_x, new_y), false))
                    }
                    _ if (new_x, new_y) != (old_x, old_y) && new_pressed => {
                        Event::TouchMove(transform_input_coords(self, (new_x, new_y), false))
                    }
                    _ => return,
                });
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
        if !self.controllers.is_empty() {
            log!("Your connected controller's left analog stick will be used for accelerometer simulation.");
            if self.accelerometer.is_some() {
                log!("Disconnect the controller if you want to use your device's accelerometer.");
            }
        } else if self.accelerometer.is_some() {
            log!("Your device's accelerometer will be used for accelerometer simulation.");
            log!("Connect a controller if you would prefer to use an analog stick.");
        } else if self.controllers.is_empty() {
            log!("Connect a controller to get accelerometer simulation.");
        }
    }

    /// Get the real or simulated accelerometer output.
    /// See also [crate::frameworks::uikit::ui_accelerometer].
    pub fn get_acceleration(&self, options: &Options) -> (f32, f32, f32) {
        if self.controllers.is_empty() {
            if let Some(ref accelerometer) = self.accelerometer {
                let data = accelerometer.get_data().unwrap();
                let sdl2::sensor::SensorData::Accel(data) = data else { panic!(); };
                let [x, y, z] = data;
                // UIAcceleration reports acceleration towards gravity, but SDL2
                // reports acceleration away from gravity.
                let (x, y, z) = (-x, -y, -z);
                // UIAcceleration reports acceleration in units of g-force, but SDL2
                // reports acceleration in units of m/s^2.
                let gravity: f32 = 9.80665; // SDL_STANDARD_GRAVITY
                let (x, y, z) = (x / gravity, y / gravity, z / gravity);
                return (x, y, z);
            }
        }

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
        // is usually a circle enclosed by the square. So we need to cut out the
        // rectangular shape of the screen from that circle within the square.
        let (vx, vy, vw, vh) = self.viewport();
        let (vx, vy, vw, vh) = (vx as f32, vy as f32, vw as f32, vh as f32);

        let (x, y) = {
            // Use Pythagoras's theorem to find the largest size the rectangle
            // can have within the circle.
            let ratio = vw / vh;
            let rect_height = (ratio * ratio + 1.0).powf(-0.5);
            let rect_width = ratio * rect_height;

            let x_abs = x.abs().min(rect_width) / rect_width;
            let y_abs = y.abs().min(rect_height) / rect_height;
            (x_abs.copysign(x), y_abs.copysign(y))
        };

        // Convert to on-screen window co-ordinates
        let x = (x / 2.0 + 0.5) * vw + vx;
        let y = (y / 2.0 + 0.5) * vh + vy;

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

    pub fn create_gl_context(&mut self, version: GLVersion) -> Result<GLContext, String> {
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
        let (vx, vy, vw, vh) = self.viewport();
        let viewport_offset = (vx, vy + self.viewport_y_offset());
        let viewport_size = (vw, vh);

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
    /// content appears upright. On a mobile device, this might do something
    /// else, because the user can physically rotate the screen.
    pub fn rotate_device(&mut self, new_orientation: DeviceOrientation) {
        if new_orientation == self.device_orientation {
            return;
        }

        if !self.fullscreen && !Self::rotatable_fullscreen() {
            let (width, height) = if Self::rotatable_fullscreen() {
                set_sdl2_orientation(new_orientation);
                rotate_fullscreen_size(new_orientation, self.window.size())
            } else {
                size_for_orientation(new_orientation, self.scale_hack)
            };

            // macOS quirk: when resizing the window, the new framebuffer's size
            // is apparently max(new_size, old_size) in each dimension, but the
            // viewport is positioned wrong on the y axis for some reason, so we
            // need to apply an offset.
            // Recreating the OpenGL context was an alternative workaround, but
            // that apparently stops other OpenGL contexts drawing to the
            // framebuffer!
            #[cfg(target_os = "macos")]
            {
                let (_old_width, old_height) = self.window.size();
                self.max_height = self.max_height.max(old_height).max(height);
                self.viewport_y_offset = self.max_height - height;
            }

            self.window.set_size(width, height).unwrap();
        }

        if Self::rotatable_fullscreen() {
            set_sdl2_orientation(new_orientation);
            // Hack: from reading SDL2's source code, it seems that SDL2 will
            // only re-do the orientation when changing whether a window is
            // "resizeable" (can be rotated). You can't set the resizeable state
            // on a fullscreen window, so it must be temporarily stop being
            // fulscreen.
            // Apparently, doing this does result in resizing the window.
            self.window
                .set_fullscreen(sdl2::video::FullscreenType::Off)
                .unwrap();
            unsafe {
                let window_raw = self.window.raw();
                sdl2_sys::SDL_SetWindowResizable(window_raw, sdl2_sys::SDL_bool::SDL_FALSE);
                sdl2_sys::SDL_SetWindowResizable(window_raw, sdl2_sys::SDL_bool::SDL_TRUE);
            }
            self.window
                .set_fullscreen(sdl2::video::FullscreenType::True)
                .unwrap();
        }

        self.device_orientation = new_orientation;

        if self.splash_image_and_gl_ctx.is_some() {
            self.display_splash();
        }
    }

    /// Get the size in pixels of the window without rotation or scaling.
    ///
    /// The aspect ratio, scale and orientation reflect the guest app's view of
    /// the world.
    pub fn size_unrotated_unscaled(&self) -> (u32, u32) {
        size_for_orientation(DeviceOrientation::Portrait, NonZeroU32::new(1).unwrap())
    }

    /// Get the size in pixels of the window without rotation but with the
    /// scale hack. Scaling caused by fullscreen mode is not included.
    ///
    /// Only the aspect ratio and orientation reflect the guest app's view of
    /// the world.
    pub fn size_unrotated_scalehacked(&self) -> (u32, u32) {
        size_for_orientation(DeviceOrientation::Portrait, self.scale_hack)
    }

    /// Get the region of the on-screen window (x, y, width, height) used to
    /// display the app content.
    ///
    /// The aspect ratio of this region always reflects the guest app's view of
    /// the world, but the scale and orientation might not.
    pub fn viewport(&self) -> (u32, u32, u32, u32) {
        let (app_width, app_height) =
            size_for_orientation(self.device_orientation, self.scale_hack);
        if !self.fullscreen && !Self::rotatable_fullscreen() {
            return (0, 0, app_width, app_height);
        }

        let (screen_width, screen_height) = self.window.drawable_size();

        let app_aspect = app_width as f32 / app_height as f32;
        let screen_aspect = screen_width as f32 / screen_height as f32;
        let (scaled_width, scaled_height) = if app_aspect < screen_aspect {
            (
                (screen_height as f32 * app_aspect).round() as u32,
                screen_height,
            )
        } else {
            (
                screen_width,
                (screen_width as f32 / app_aspect).round() as u32,
            )
        };
        let x = (screen_width - scaled_width) / 2;
        let y = (screen_height - scaled_height) / 2;
        (x, y, scaled_width, scaled_height)
    }

    /// Special offset to add to y co-ordinates, only when drawing to screen.
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
