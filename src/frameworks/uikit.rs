/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The UIKit framework.
//!
//! For the time being the focus of this project is on running games, which are
//! likely to use UIKit in very simple and limited ways, so this implementation
//! will probably take a lot of shortcuts.

use crate::Environment;
use std::time::Instant;

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
    ui_device: ui_device::State,
    ui_font: ui_font::State,
    ui_graphics: ui_graphics::State,
    ui_screen: ui_screen::State,
    ui_touch: ui_touch::State,
    ui_view: ui_view::State,
}

/// For use by `NSRunLoop`: handles any events that have queued up.
///
/// Returns the next time this function must be called, if any, e.g. the next
/// time an accelerometer input is due.
pub fn handle_events(env: &mut Environment) -> Option<Instant> {
    use crate::window::Event;

    loop {
        let Some(event) = env.window.pop_event() else {
            break;
        };

        match event {
            Event::Quit => {
                println!("User requested quit, exiting.");
                ui_application::exit(env);
            }
            Event::TouchDown(..) | Event::TouchMove(..) | Event::TouchUp(..) => {
                ui_touch::handle_event(env, event)
            }
        }
    }

    ui_accelerometer::handle_accelerometer(env)
}
