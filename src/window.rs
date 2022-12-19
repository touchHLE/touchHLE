//! Abstraction of window setup and event handling.
//!
//! Implemented using the sdl2 crate (a Rust wrapper for SDL2). All usage of
//! SDL should be confined to this module.
//!
//! There is currently no separation of concerns between a single window and
//! window system interaction in general, because it is assumed only one window
//! will be needed for the runtime of the app.

use crate::image::Image;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
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
    _video_ctx: sdl2::VideoSubsystem,
    _window: sdl2::video::Window,
    event_pump: sdl2::EventPump,
}
impl Window {
    pub fn new(title: &str, icon: Image, launch_image: Option<Image>) -> Window {
        let sdl_ctx = sdl2::init().unwrap();
        let video_ctx = sdl_ctx.video().unwrap();

        let mut window = video_ctx
            .window(title, 320, 480)
            .position_centered()
            .build()
            .unwrap();

        window.set_icon(surface_from_image(&icon));

        let event_pump = sdl_ctx.event_pump().unwrap();

        if let Some(launch_image) = launch_image {
            let mut window_surface = window.surface(&event_pump).unwrap();
            surface_from_image(&launch_image)
                .blit(
                    Rect::new(0, 0, 320, 480),
                    &mut window_surface,
                    Rect::new(0, 0, 320, 480),
                )
                .unwrap();
            window_surface.finish().unwrap();
        }

        Window {
            _sdl_ctx: sdl_ctx,
            _video_ctx: video_ctx,
            _window: window,
            event_pump,
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
}
