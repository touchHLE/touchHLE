/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AudioSession.h` (Audio Session Services)
//!
//! Deprecated since iOS 7 in favour of `AVAudioSession`, but every 32-bit app
//! and every piece of audio middleware (CRI Mana, FMOD, Bink, ...) still uses
//! it, so the properties they set have to be remembered and reported back.

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::carbon_core::{paramErr, OSStatus};
use crate::frameworks::core_audio_types::{debug_fourcc, fourcc};
use crate::frameworks::core_foundation::cf_run_loop::{CFRunLoopMode, CFRunLoopRef};
use crate::mem::{guest_size_of, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr};
use crate::Environment;

type AudioSessionInterruptionListener = GuestFunction;
type AudioSessionPropertyListener = GuestFunction;

/// `'!ini'`
const kAudioSessionNotInitialized: OSStatus = fourcc(b"!ini") as _;
/// `'!siz'`
const kAudioSessionBadPropertySizeError: OSStatus = fourcc(b"!siz") as _;
/// `'pty?'`
const kAudioSessionUnsupportedPropertyError: OSStatus = fourcc(b"pty?") as _;

/// Usually a FourCC.
type AudioSessionPropertyID = u32;
const kAudioSessionProperty_OtherAudioIsPlaying: AudioSessionPropertyID = fourcc(b"othr");
const kAudioSessionProperty_AudioCategory: AudioSessionPropertyID = fourcc(b"acat");
const kAudioSessionProperty_CurrentHardwareSampleRate: AudioSessionPropertyID = fourcc(b"chsr");
const kAudioSessionProperty_CurrentHardwareOutputNumberChannels: AudioSessionPropertyID =
    fourcc(b"choc");
const kAudioSessionProperty_CurrentHardwareOutputVolume: AudioSessionPropertyID = fourcc(b"chov");
const kAudioSessionProperty_CurrentHardwareInputNumberChannels: AudioSessionPropertyID =
    fourcc(b"chic");
const kAudioSessionProperty_PreferredHardwareIOBufferDuration: AudioSessionPropertyID =
    fourcc(b"iobd");
const kAudioSessionProperty_PreferredHardwareSampleRate: AudioSessionPropertyID = fourcc(b"hwsr");
const kAudioSessionProperty_CurrentHardwareIOBufferDuration: AudioSessionPropertyID =
    fourcc(b"chbd");
const kAudioSessionProperty_Mode: AudioSessionPropertyID = fourcc(b"mode");
const kAudioSessionProperty_InputAvailable: AudioSessionPropertyID = fourcc(b"inav");
const kAudioSessionProperty_OverrideCategoryMixWithOthers: AudioSessionPropertyID =
    fourcc(b"mixe");
const kAudioSessionProperty_OtherMixableAudioShouldDuck: AudioSessionPropertyID = fourcc(b"duck");
const kAudioSessionProperty_OverrideCategoryDefaultToSpeaker: AudioSessionPropertyID =
    fourcc(b"spkr");
const kAudioSessionProperty_OverrideCategoryEnableBluetoothInput: AudioSessionPropertyID =
    fourcc(b"blni");

const kAudioSessionCategory_SoloAmbientSound: u32 = fourcc(b"solo");
const kAudioSessionMode_Default: u32 = fourcc(b"dflt");

const kAudioSessionOverrideAudioRoute_None: u32 = 0;

#[derive(Clone, Copy)]
struct PropertyListener {
    property_id: AudioSessionPropertyID,
    listener: AudioSessionPropertyListener,
    client_data: MutVoidPtr,
}

pub struct State {
    initialized: bool,
    active: bool,
    interruption_listener: Option<AudioSessionInterruptionListener>,
    interruption_client_data: MutVoidPtr,
    property_listeners: Vec<PropertyListener>,
    audio_session_category: u32,
    audio_session_mode: u32,
    pub current_hardware_sample_rate: f64,
    pub current_hardware_output_number_channels: u32,
    current_hardware_output_volume: f32,
    current_hardware_io_buffer_duration: f32,
    preferred_hardware_sample_rate: Option<f64>,
    preferred_hardware_io_buffer_duration: Option<f32>,
    audio_route_override: u32,
    mix_with_others: bool,
    duck_others: bool,
    bluetooth_input: bool,
}
impl Default for State {
    fn default() -> Self {
        // TODO: Check values from a real device
        State {
            initialized: false,
            active: false,
            interruption_listener: None,
            interruption_client_data: MutVoidPtr::null(),
            property_listeners: Vec::new(),
            // This is the default value.
            audio_session_category: kAudioSessionCategory_SoloAmbientSound,
            audio_session_mode: kAudioSessionMode_Default,
            // Values taken from an iOS 2 simulator
            current_hardware_sample_rate: 44100.0,
            current_hardware_output_number_channels: 2,
            current_hardware_output_volume: 1.0,
            // Value was checked on both iOS Simulator and iPhone 3GS
            current_hardware_io_buffer_duration: 0.023220,
            preferred_hardware_sample_rate: None,
            preferred_hardware_io_buffer_duration: None,
            audio_route_override: kAudioSessionOverrideAudioRoute_None,
            mix_with_others: false,
            duck_others: false,
            bluetooth_input: false,
        }
    }
}
impl State {
    pub fn category(&self) -> u32 {
        self.audio_session_category
    }
    pub fn is_active(&self) -> bool {
        self.active
    }
}

fn AudioSessionInitialize(
    env: &mut Environment,
    in_run_loop: CFRunLoopRef,
    in_run_loop_mode: CFRunLoopMode,
    in_interruption_listener: AudioSessionInterruptionListener,
    in_client_data: MutVoidPtr,
) -> OSStatus {
    let state = &mut env.framework_state.audio_toolbox.audio_session;
    // Re-initialising an already initialised session is allowed and just
    // replaces the listener, matching what a real device does.
    state.initialized = true;
    if in_interruption_listener.to_ptr().is_null() {
        state.interruption_listener = None;
    } else {
        state.interruption_listener = Some(in_interruption_listener);
    }
    state.interruption_client_data = in_client_data;

    let result = 0; // success
    log_dbg!(
        "AudioSessionInitialize({:?}, {:?}, {:?}, {:?}) -> {:?}",
        in_run_loop,
        in_run_loop_mode,
        in_interruption_listener,
        in_client_data,
        result
    );
    result
}

fn AudioSessionGetPropertySize(
    env: &mut Environment,
    in_ID: AudioSessionPropertyID,
    out_data_size: MutPtr<u32>,
) -> OSStatus {
    match get_audio_session_property_size(in_ID) {
        Some(size) => {
            env.mem.write(out_data_size, size);
            log_dbg!(
                "AudioSessionGetPropertySize({}) -> 0 ({})",
                debug_fourcc(in_ID),
                size
            );
            0 // Success
        }
        None => {
            env.mem.write(out_data_size, 0);
            log_dbg!(
                "AudioSessionGetPropertySize({}) -> unsupported property",
                debug_fourcc(in_ID)
            );
            kAudioSessionUnsupportedPropertyError
        }
    }
}

fn AudioSessionGetProperty(
    env: &mut Environment,
    in_ID: AudioSessionPropertyID,
    io_data_size: MutPtr<u32>,
    out_data: MutVoidPtr,
) -> OSStatus {
    let required_size = match get_audio_session_property_size(in_ID) {
        Some(size) => size,
        None => {
            log_dbg!(
                "AudioSessionGetProperty({}) -> unsupported property",
                debug_fourcc(in_ID)
            );
            return kAudioSessionUnsupportedPropertyError;
        }
    };
    let io_data_size_value = env.mem.read(io_data_size);
    if io_data_size_value != required_size {
        log_dbg!(
            "AudioSessionGetProperty({}): bad size {} (expected {})",
            debug_fourcc(in_ID),
            io_data_size_value,
            required_size
        );
        return kAudioSessionBadPropertySizeError;
    }

    let state = &env.framework_state.audio_toolbox.audio_session;
    match in_ID {
        kAudioSessionProperty_OtherAudioIsPlaying => {
            let value: u32 = 0;
            env.mem.write(out_data.cast(), value);
        }
        kAudioSessionProperty_AudioCategory => {
            let value: u32 = state.audio_session_category;
            env.mem.write(out_data.cast(), value);
        }
        kAudioSessionProperty_Mode => {
            let value: u32 = state.audio_session_mode;
            env.mem.write(out_data.cast(), value);
        }
        kAudioSessionProperty_CurrentHardwareSampleRate => {
            let value: f64 = state.current_hardware_sample_rate;
            env.mem.write(out_data.cast(), value);
        }
        kAudioSessionProperty_CurrentHardwareOutputNumberChannels => {
            let value: u32 = state.current_hardware_output_number_channels;
            env.mem.write(out_data.cast(), value);
        }
        kAudioSessionProperty_CurrentHardwareInputNumberChannels => {
            let value: u32 = 0;
            env.mem.write(out_data.cast(), value);
        }
        kAudioSessionProperty_CurrentHardwareOutputVolume => {
            let value: f32 = state.current_hardware_output_volume;
            env.mem.write(out_data.cast(), value);
        }
        kAudioSessionProperty_CurrentHardwareIOBufferDuration => {
            let value: f32 = state.current_hardware_io_buffer_duration;
            env.mem.write(out_data.cast(), value);
        }
        kAudioSessionProperty_InputAvailable => {
            let value: u32 = 0;
            env.mem.write(out_data.cast(), value);
        }
        kAudioSessionProperty_OverrideCategoryMixWithOthers => {
            let value: u32 = state.mix_with_others as u32;
            env.mem.write(out_data.cast(), value);
        }
        kAudioSessionProperty_OtherMixableAudioShouldDuck => {
            let value: u32 = state.duck_others as u32;
            env.mem.write(out_data.cast(), value);
        }
        kAudioSessionProperty_OverrideCategoryDefaultToSpeaker => {
            let value: u32 = (state.audio_route_override != kAudioSessionOverrideAudioRoute_None)
                as u32;
            env.mem.write(out_data.cast(), value);
        }
        kAudioSessionProperty_OverrideCategoryEnableBluetoothInput => {
            let value: u32 = state.bluetooth_input as u32;
            env.mem.write(out_data.cast(), value);
        }
        _ => return kAudioSessionUnsupportedPropertyError,
    }

    let result = 0; // success
    log_dbg!(
        "AudioSessionGetProperty({}, {:?} ({:?}), {:?} ({:?})) -> {:?})",
        debug_fourcc(in_ID),
        io_data_size,
        io_data_size_value,
        out_data,
        env.mem.bytes_at(out_data.cast(), io_data_size_value),
        result
    );
    result
}

fn AudioSessionSetProperty(
    env: &mut Environment,
    in_ID: AudioSessionPropertyID,
    in_data_size: u32,
    in_data: ConstVoidPtr,
) -> OSStatus {
    if in_data.is_null() {
        return paramErr;
    }

    let state = &mut env.framework_state.audio_toolbox.audio_session;
    // Read-only properties: a real device silently ignores attempts to set
    // them, so do the same rather than failing the whole audio setup.
    let required_size: GuestUSize = match in_ID {
        kAudioSessionProperty_AudioCategory => guest_size_of::<u32>(),
        kAudioSessionProperty_Mode => guest_size_of::<u32>(),
        kAudioSessionProperty_OverrideCategoryMixWithOthers => guest_size_of::<u32>(),
        kAudioSessionProperty_OtherMixableAudioShouldDuck => guest_size_of::<u32>(),
        kAudioSessionProperty_OverrideCategoryDefaultToSpeaker => guest_size_of::<u32>(),
        kAudioSessionProperty_OverrideCategoryEnableBluetoothInput => guest_size_of::<u32>(),
        kAudioSessionProperty_PreferredHardwareIOBufferDuration => guest_size_of::<f32>(),
        kAudioSessionProperty_PreferredHardwareSampleRate => guest_size_of::<f64>(),
        kAudioSessionProperty_OtherAudioIsPlaying
        | kAudioSessionProperty_CurrentHardwareSampleRate
        | kAudioSessionProperty_CurrentHardwareOutputNumberChannels
        | kAudioSessionProperty_CurrentHardwareInputNumberChannels
        | kAudioSessionProperty_CurrentHardwareOutputVolume
        | kAudioSessionProperty_CurrentHardwareIOBufferDuration
        | kAudioSessionProperty_InputAvailable => {
            log_dbg!(
                "AudioSessionSetProperty({}): property is read-only, ignoring",
                debug_fourcc(in_ID)
            );
            return 0;
        }
        _ => {
            // Unknown properties are accepted rather than rejected: apps and
            // middleware probe for properties that only exist on some iOS
            // versions, and returning an error here aborts audio setup.
            log_dbg!(
                "AudioSessionSetProperty({}): unknown property, accepting {} byte(s)",
                debug_fourcc(in_ID),
                in_data_size
            );
            return 0;
        }
    };
    if in_data_size != required_size {
        log_dbg!(
            "AudioSessionSetProperty({}): bad size {} (expected {})",
            debug_fourcc(in_ID),
            in_data_size,
            required_size
        );
        return kAudioSessionBadPropertySizeError;
    }

    match in_ID {
        kAudioSessionProperty_AudioCategory => {
            state.audio_session_category = env.mem.read(in_data.cast::<u32>());
        }
        kAudioSessionProperty_Mode => {
            state.audio_session_mode = env.mem.read(in_data.cast::<u32>());
        }
        kAudioSessionProperty_OverrideCategoryMixWithOthers => {
            state.mix_with_others = env.mem.read(in_data.cast::<u32>()) != 0;
        }
        kAudioSessionProperty_OtherMixableAudioShouldDuck => {
            state.duck_others = env.mem.read(in_data.cast::<u32>()) != 0;
        }
        kAudioSessionProperty_OverrideCategoryDefaultToSpeaker => {
            state.audio_route_override = env.mem.read(in_data.cast::<u32>());
        }
        kAudioSessionProperty_OverrideCategoryEnableBluetoothInput => {
            state.bluetooth_input = env.mem.read(in_data.cast::<u32>()) != 0;
        }
        kAudioSessionProperty_PreferredHardwareSampleRate => {
            let rate = env.mem.read(in_data.cast::<f64>());
            state.preferred_hardware_sample_rate = Some(rate);
            state.current_hardware_sample_rate = rate;
        }
        kAudioSessionProperty_PreferredHardwareIOBufferDuration => {
            let duration = env.mem.read(in_data.cast::<f32>());
            state.preferred_hardware_io_buffer_duration = Some(duration);
            state.current_hardware_io_buffer_duration = duration;
        }
        _ => unreachable!(),
    }

    let result = 0; // success
    log_dbg!(
        "AudioSessionSetProperty({}, {:?}, {:?} ({:?})) -> {:?}",
        debug_fourcc(in_ID),
        in_data_size,
        in_data,
        env.mem.bytes_at(in_data.cast(), in_data_size),
        result
    );
    result
}

fn AudioSessionSetActive(env: &mut Environment, active: bool) -> OSStatus {
    let state = &mut env.framework_state.audio_toolbox.audio_session;
    if !state.initialized {
        log_dbg!("AudioSessionSetActive({:?}) -> not initialized", active);
        return kAudioSessionNotInitialized;
    }
    if state.active != active {
        state.active = active;
        log_dbg!("AudioSessionSetActive({:?}) -> 0", active);
    }
    0
}

fn AudioSessionAddPropertyListener(
    env: &mut Environment,
    inID: AudioSessionPropertyID,
    inProc: AudioSessionPropertyListener,
    inClientData: MutVoidPtr,
) -> OSStatus {
    if inProc.to_ptr().is_null() {
        return paramErr;
    }
    let state = &mut env.framework_state.audio_toolbox.audio_session;
    if !state
        .property_listeners
        .iter()
        .any(|l| l.property_id == inID && l.listener == inProc && l.client_data == inClientData)
    {
        state.property_listeners.push(PropertyListener {
            property_id: inID,
            listener: inProc,
            client_data: inClientData,
        });
    }
    let result = 0; // success
    log_dbg!(
        "AudioSessionAddPropertyListener({}, {:?}, {:?}) -> {}",
        debug_fourcc(inID),
        inProc,
        inClientData,
        result
    );
    result
}

fn AudioSessionRemovePropertyListenerWithUserData(
    env: &mut Environment,
    in_property_id: AudioSessionPropertyID,
    in_listener: AudioSessionPropertyListener,
    in_client_data: MutVoidPtr,
) -> OSStatus {
    let state = &mut env.framework_state.audio_toolbox.audio_session;
    state.property_listeners.retain(|l| {
        !(l.property_id == in_property_id
            && l.listener == in_listener
            && l.client_data == in_client_data)
    });
    let result = 0; // success
    log_dbg!(
        "AudioSessionRemovePropertyListenerWithUserData({}, {:?}, {:?}) -> {}",
        debug_fourcc(in_property_id),
        in_listener,
        in_client_data,
        result
    );
    result
}

/// Helper function to get Audio Session Property size by id.
/// `None` means the property is not supported by this implementation.
fn get_audio_session_property_size(in_ID: AudioSessionPropertyID) -> Option<GuestUSize> {
    match in_ID {
        kAudioSessionProperty_OtherAudioIsPlaying
        | kAudioSessionProperty_InputAvailable => Some(guest_size_of::<u32>()),
        kAudioSessionProperty_AudioCategory => Some(guest_size_of::<u32>()),
        kAudioSessionProperty_Mode => Some(guest_size_of::<u32>()),
        kAudioSessionProperty_CurrentHardwareSampleRate => Some(guest_size_of::<f64>()),
        kAudioSessionProperty_CurrentHardwareOutputNumberChannels
        | kAudioSessionProperty_CurrentHardwareInputNumberChannels => Some(guest_size_of::<u32>()),
        kAudioSessionProperty_CurrentHardwareOutputVolume => Some(guest_size_of::<f32>()),
        kAudioSessionProperty_CurrentHardwareIOBufferDuration => Some(guest_size_of::<f32>()),
        kAudioSessionProperty_OverrideCategoryMixWithOthers
        | kAudioSessionProperty_OtherMixableAudioShouldDuck
        | kAudioSessionProperty_OverrideCategoryDefaultToSpeaker
        | kAudioSessionProperty_OverrideCategoryEnableBluetoothInput => Some(guest_size_of::<u32>()),
        _ => None,
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioSessionInitialize(_, _, _, _)),
    export_c_func!(AudioSessionGetProperty(_, _, _)),
    export_c_func!(AudioSessionGetPropertySize(_, _)),
    export_c_func!(AudioSessionSetProperty(_, _, _)),
    export_c_func!(AudioSessionSetActive(_)),
    export_c_func!(AudioSessionAddPropertyListener(_, _, _)),
    export_c_func!(AudioSessionRemovePropertyListenerWithUserData(_, _, _)),
];
