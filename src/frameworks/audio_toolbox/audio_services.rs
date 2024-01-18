/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AudioServices.h` (Audio Services)

use std::collections::HashMap;

use crate::audio::ContextManager;
use crate::audio::openal as al;
use crate::audio::openal::al_types::*;
use crate::audio::openal::alc_types::*;

use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::carbon_core::OSStatus;
use crate::frameworks::core_audio_types::fourcc;
use crate::frameworks::core_foundation::cf_url::CFURLRef;
use crate::frameworks::foundation::ns_url::to_rust_path;
use crate::mem::{MutPtr, MutVoidPtr};
use crate::{audio, Environment};

use super::audio_queue::decode_buffer;

/// Usually a FourCC.
type AudioServicesPropertyID = u32;
type SystemSoundID = u32;

const kAudioServicesUnsupportedPropertyError: OSStatus = fourcc(b"pty?") as _;
const kSystemSoundID_Vibrate: SystemSoundID = 0x00000FFF;
const kAudioServicesSystemSoundUnspecifiedError: OSStatus = -1500;

struct SystemSoundData {
    al_source: ALuint,
    al_buffer: ALuint,
}

pub struct State {
    pub(self) sounds: HashMap<SystemSoundID, SystemSoundData>,
    pub(self) al_device_and_context: Option<(*mut ALCdevice, *mut ALCcontext)>,
    pub(self) data_top: SystemSoundID,
}

impl Default for State {
    fn default() -> Self {
        Self {
            sounds: Default::default(),
            al_device_and_context: None,
            data_top: 0x1001,
        }
    }
}

impl State {
    fn get(framework_state: &mut crate::frameworks::State) -> &mut Self {
        &mut framework_state.audio_toolbox.audio_services
    }
    fn make_al_context_current(&mut self) -> ContextManager {
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

fn AudioServicesCreateSystemSoundID(
    env: &mut Environment,
    file: CFURLRef,
    out_system_sound_id: MutPtr<SystemSoundID>,
) -> OSStatus {
    let path = to_rust_path(env, file);
    let Ok(mut audio_file) = audio::AudioFile::open_for_reading(&path, &env.fs) else {
        log!(
            "Warning: Failed to open audio file {:?} for AudioServicesCreateSystemSoundID()",
            path
        );
        return kAudioServicesSystemSoundUnspecifiedError;
    };

    let mut data = vec![0; audio_file.byte_count().try_into().unwrap()];
    let format = audio_file.audio_description().into_basic_description();
    audio_file.read_bytes(0, data.as_mut_slice()).unwrap();
    let (al_format, al_frequency, data) = decode_buffer(data.as_mut_slice(), &format);

    let state = State::get(&mut env.framework_state);
    let _ctx = state.make_al_context_current();
    // TODO: This should only support linear pcm and ima4, but also supports
    // mp3 here since AudioFile supports it. We also aren't checking for length.
    let mut al_source = 0;
    unsafe {
        al::alGenSources(1, &mut al_source);
        //al::alSourcef(al_source, al::AL_MAX_GAIN, 1.0);
        assert!(al::alGetError() == 0);
    }

    let mut al_buffer = 0;
    unsafe {
        al::alGenBuffers(1, &mut al_buffer);
        al::alBufferData(
            al_buffer,
            al_format,
            data.as_ptr() as *const ALvoid,
            data.len().try_into().unwrap(),
            al_frequency,
        );
        al::alSourcei(al_source, al::AL_BUFFER, al_buffer.try_into().unwrap());
        assert!(al::alGetError() == 0);
    }
    let sys_data = SystemSoundData {
        al_source,
        al_buffer,
    };
    state.sounds.insert(state.data_top, sys_data);
    env.mem.write(out_system_sound_id, state.data_top);
    state.data_top = state.data_top.checked_add(1).unwrap();
    0
}

fn AudioServicesGetProperty(
    _env: &mut Environment,
    in_property_id: AudioServicesPropertyID,
    _in_specifier_size: u32,
    _in_specifier: crate::mem::ConstVoidPtr,
    _io_property_data_size: MutPtr<u32>,
    _out_property_data: MutVoidPtr,
) -> OSStatus {
    // Crash Bandicoot Nitro Kart 3D tries to use this property ID, which does
    // not seem to be documented anywhere? Assuming this is a bug.
    if in_property_id == 0xfff {
        kAudioServicesUnsupportedPropertyError
    } else {
        unimplemented!();
    }
}

fn AudioServicesPlaySystemSound(env: &mut Environment, sys_sound_id: SystemSoundID) {
    let state = State::get(&mut env.framework_state);
    if sys_sound_id == kSystemSoundID_Vibrate {
        log!("TODO: vibration (AudioServicesPlaySystemSound)");
    } else if let Some(SystemSoundData {
        al_source,
        al_buffer: _,
    }) = state.sounds.get(&sys_sound_id)
    {
        unsafe {
            let al_source = *al_source;
            let _ctx = state.make_al_context_current();
            let mut al_state: i32 = 0;
            al::alGetSourcei(al_source, al::AL_SOURCE_STATE, &mut al_state as *mut i32);
            al::alSourcePlay(al_source);
            al::alGetSourcei(al_source, al::AL_SOURCE_STATE, &mut al_state as *mut i32);
            assert!(al::alGetError() == 0);
        }
    } else {
        panic!(
            "Incorrect/unsupported system sound {:x} played!",
            sys_sound_id
        );
    }
    // TODO: implement other system sounds
}

fn AudioServicesDisposeSystemSound(env: &mut Environment, sys_sound_id: SystemSoundID) -> OSStatus {
    let state = State::get(&mut env.framework_state);
    if let Some(SystemSoundData {
        al_source,
        al_buffer,
    }) = state.sounds.remove(&sys_sound_id)
    {
        unsafe {
            al::alSourceStop(al_source);
            al::alDeleteSources(1, &al_source as *const ALuint);
            al::alDeleteBuffers(1, &al_buffer as *const ALuint);
            assert!(al::alGetError() == 0);
        }
        0
    } else {
        // This is also true of kSystemSoundID_Vibrate.
        log!("Tried to dispose of invalid system sound {}!", sys_sound_id);
        kAudioServicesSystemSoundUnspecifiedError
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioServicesCreateSystemSoundID(_, _)),
    export_c_func!(AudioServicesGetProperty(_, _, _, _, _)),
    export_c_func!(AudioServicesPlaySystemSound(_)),
    export_c_func!(AudioServicesDisposeSystemSound(_)),
];
