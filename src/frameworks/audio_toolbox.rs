/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The Audio Toolbox framework.

use crate::audio::openal as al;
use crate::audio::openal::alc_types::{ALCcontext, ALCdevice};

/// Macro for checking if an argument is null and returning `paramErr` if so.
/// This seems to be what the real Audio Toolbox does, and some apps rely on it.
macro_rules! return_if_null {
    ($param:ident) => {
        if $param.is_null() {
            log_dbg!(
                "Got NULL parameter {}, returning paramErr in {} on line {}",
                stringify!($param),
                file!(),
                line!()
            );
            return crate::frameworks::carbon_core::paramErr;
        }
    };
}

pub mod audio_components;
pub mod audio_file;
pub mod audio_queue;
pub mod audio_services;
pub mod audio_session;
pub mod audio_unit;

#[derive(Default)]
pub struct State {
    audio_file: audio_file::State,
    audio_queue: audio_queue::State,
    audio_components: audio_components::State,
    audio_session: audio_session::State,
    al_device_and_context: Option<(*mut ALCdevice, *mut ALCcontext)>,
}
impl State {
    pub fn make_al_context_current(&mut self) -> ContextManager {
        if self.al_device_and_context.is_none() {
            let device = unsafe { al::alcOpenDevice(std::ptr::null()) };
            assert!(!device.is_null());
            let context = unsafe { al::alcCreateContext(device, std::ptr::null()) };
            assert!(!context.is_null());
            log_dbg!(
                "New internal OpenAL device ({:?}) and context ({:?})",
                device,
                context
            );
            self.al_device_and_context = Some((device, context));
        }
        let (device, context) = self.al_device_and_context.unwrap();
        assert!(!device.is_null() && !context.is_null());

        // This object will make sure the existing context, which will belong
        // to the guest app, is restored once we're done.
        ContextManager::make_active(context)
    }
}

#[must_use]
pub struct ContextManager(*mut ALCcontext);
impl ContextManager {
    pub fn make_active(new_context: *mut ALCcontext) -> ContextManager {
        let old_context = unsafe { al::alcGetCurrentContext() };
        assert!(unsafe { al::alcMakeContextCurrent(new_context) } == al::ALC_TRUE);
        ContextManager(old_context)
    }
}
impl Drop for ContextManager {
    fn drop(&mut self) {
        assert!(unsafe { al::alcMakeContextCurrent(self.0) } == al::ALC_TRUE)
    }
}
