//! The UIKit framework.
//!
//! For the time being the focus of this project is on running games, which are
//! likely to use UIKit in very simple and limited ways, so this implementation
//! will probably take a lot of shortcuts.

use crate::Environment;

pub mod ui_accelerometer;
pub mod ui_application;
pub mod ui_device;
pub mod ui_event;
pub mod ui_font;
pub mod ui_graphics;
pub mod ui_nib;
pub mod ui_responder;
pub mod ui_screen;
pub mod ui_touch;
pub mod ui_view;
pub mod ui_window;

#[derive(Default)]
pub struct State {
    ui_accelerometer: ui_accelerometer::State,
    ui_application: ui_application::State,
    ui_font: ui_font::State,
    ui_graphics: ui_graphics::State,
    ui_screen: ui_screen::State,
    ui_touch: ui_touch::State,
    ui_view: ui_view::State,
}

/// For use by `NSRunLoop`: handles any events that have queued up.
pub fn handle_events(env: &mut Environment) {
    use crate::window::Event;

    loop {
        let Some(event) = env.window.pop_event() else {
            return;
        };

        match event {
            // FIXME: tell the app when we're about to quit
            Event::Quit => {
                println!("User requested quit, exiting.");
                std::process::exit(0);
            }
            Event::TouchDown(..) | Event::TouchMove(..) | Event::TouchUp(..) => {
                ui_touch::handle_event(env, event)
            }
        }
    }
}
