/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `AudioSession.h` (Audio Session Services)

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::carbon_core::OSStatus;
use crate::frameworks::core_audio_types::{debug_fourcc, fourcc};
use crate::frameworks::core_foundation::cf_run_loop::{CFRunLoopMode, CFRunLoopRef};
use crate::mem::{guest_size_of, ConstVoidPtr, MutPtr, MutVoidPtr};
use crate::Environment;

type AudioSessionInterruptionListener = GuestFunction;
type AudioSessionPropertyListener = GuestFunction;

const kAudioSessionBadPropertySizeError: OSStatus = fourcc(b"!siz") as _;
const kAudioSessionNoErr: OSStatus = 0;

type AudioSessionPropertyID = u32;

// --- Fourcc-коды Apple из AudioToolbox/AudioSession.h ---
// ВНИМАНИЕ: в предыдущей версии этого файла у двух свойств были НЕПРАВИЛЬНЫЕ
// fourcc'и ('pbuf' и 'cbuf'). Реальные (из SDK) — 'iobd' и 'chbd'.
// Именно из-за этого в touchHLE_log.txt появлялось
//   "TODO: AudioSessionSetProperty UNIMPLEMENTED 'iobd'"
// и игра (RE VS. / biovsus и др.) не могла корректно настроить аудио-сессию.
const kAudioSessionProperty_PreferredHardwareSampleRate: AudioSessionPropertyID = fourcc(b"hwsr");
const kAudioSessionProperty_PreferredHardwareIOBufferDuration: AudioSessionPropertyID =
    fourcc(b"iobd");
const kAudioSessionProperty_AudioCategory: AudioSessionPropertyID = fourcc(b"acat");
const kAudioSessionProperty_AudioRoute: AudioSessionPropertyID = fourcc(b"rout");
const kAudioSessionProperty_CurrentHardwareSampleRate: AudioSessionPropertyID = fourcc(b"chsr");
const kAudioSessionProperty_CurrentHardwareInputNumberChannels: AudioSessionPropertyID =
    fourcc(b"chic");
const kAudioSessionProperty_CurrentHardwareOutputNumberChannels: AudioSessionPropertyID =
    fourcc(b"choc");
const kAudioSessionProperty_CurrentHardwareOutputVolume: AudioSessionPropertyID = fourcc(b"chov");
const kAudioSessionProperty_CurrentHardwareIOBufferDuration: AudioSessionPropertyID =
    fourcc(b"chbd");
const kAudioSessionProperty_OtherAudioIsPlaying: AudioSessionPropertyID = fourcc(b"othr");
const kAudioSessionProperty_OverrideAudioRoute: AudioSessionPropertyID = fourcc(b"ovrd");
const kAudioSessionProperty_AudioInputAvailable: AudioSessionPropertyID = fourcc(b"aiav");
const kAudioSessionProperty_OverrideCategoryMixWithOthers: AudioSessionPropertyID = fourcc(b"cmix");
const kAudioSessionProperty_OverrideCategoryDefaultToSpeaker: AudioSessionPropertyID =
    fourcc(b"cspk");
const kAudioSessionProperty_OverrideCategoryEnableBluetoothInput: AudioSessionPropertyID =
    fourcc(b"cblu");
const kAudioSessionProperty_OtherMixableAudioShouldDuck: AudioSessionPropertyID =
    fourcc(b"duck");

// Значения категорий (из AudioSession.h)
const kAudioSessionCategory_AmbientSound: u32 = fourcc(b"ambi");
const kAudioSessionCategory_SoloAmbientSound: u32 = fourcc(b"solo");

/// Состояние аудио-сессии. Расширено полями, которые игры обычно читают/пишут,
/// чтобы не возвращать им мусор и не ломать их внутреннюю логику.
pub struct State {
    pub active: bool,
    pub category: u32,
    pub current_hardware_sample_rate: f64,
    pub current_hardware_output_number_channels: u32,
    pub current_hardware_output_volume: f32,
    pub current_hardware_io_buffer_duration: f32,
    pub preferred_hardware_sample_rate: f64,
    pub preferred_hardware_io_buffer_duration: f32,
    pub mix_with_others: u32,
    pub default_to_speaker: u32,
    pub bluetooth_input: u32,
    pub duck_others: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            active: false,
            // По умолчанию в iOS — SoloAmbientSound (звук игры заглушает
            // Music.app,
            // но сам слышен). 'ambi' подошла бы, только если игра хочет микс с
            // музыкой.
            category: kAudioSessionCategory_SoloAmbientSound,
            // Значения, ожидаемые большинством iOS-игр того периода
            current_hardware_sample_rate: 44100.0,
            current_hardware_output_number_channels: 2,
            current_hardware_output_volume: 1.0,
            // Реальный буфер на iPhone 3GS / Simulator — ~23 мс
            current_hardware_io_buffer_duration: 0.023_220,
            preferred_hardware_sample_rate: 44100.0,
            preferred_hardware_io_buffer_duration: 0.023_220,
            mix_with_others: 0,
            default_to_speaker: 0,
            bluetooth_input: 0,
            duck_others: 0,
        }
    }
}

fn get_audio_session_property_size(in_id: AudioSessionPropertyID) -> u32 {
    match in_id {
        kAudioSessionProperty_OtherAudioIsPlaying
        | kAudioSessionProperty_AudioCategory
        | kAudioSessionProperty_CurrentHardwareInputNumberChannels
        | kAudioSessionProperty_CurrentHardwareOutputNumberChannels
        | kAudioSessionProperty_AudioInputAvailable
        | kAudioSessionProperty_AudioRoute
        | kAudioSessionProperty_OverrideAudioRoute
        | kAudioSessionProperty_OverrideCategoryMixWithOthers
        | kAudioSessionProperty_OverrideCategoryDefaultToSpeaker
        | kAudioSessionProperty_OverrideCategoryEnableBluetoothInput
        | kAudioSessionProperty_OtherMixableAudioShouldDuck => guest_size_of::<u32>(),

        kAudioSessionProperty_CurrentHardwareOutputVolume
        | kAudioSessionProperty_CurrentHardwareIOBufferDuration
        | kAudioSessionProperty_PreferredHardwareIOBufferDuration => guest_size_of::<f32>(),

        kAudioSessionProperty_CurrentHardwareSampleRate
        | kAudioSessionProperty_PreferredHardwareSampleRate => guest_size_of::<f64>(),

        _ => {
            log!(
                "TODO: get_audio_session_property_size unknown property: {} -> 4",
                debug_fourcc(in_id)
            );
            guest_size_of::<u32>()
        }
    }
}

pub fn AudioSessionInitialize(
    _env: &mut Environment,
    _in_run_loop: CFRunLoopRef,
    _in_run_loop_mode: CFRunLoopMode,
    _in_interruption_listener: AudioSessionInterruptionListener,
    _in_client_data: ConstVoidPtr,
) -> OSStatus {
    log_dbg!("AudioSessionInitialize");
    kAudioSessionNoErr
}

pub fn AudioSessionSetActive(env: &mut Environment, active: u32) -> OSStatus {
    log_dbg!("AudioSessionSetActive({})", active != 0);
    env.framework_state.audio_toolbox.audio_session.active = active != 0;
    kAudioSessionNoErr
}

pub fn AudioSessionGetPropertySize(
    env: &mut Environment,
    in_id: AudioSessionPropertyID,
    out_data_size: MutPtr<u32>,
) -> OSStatus {
    log_dbg!("AudioSessionGetPropertySize for {}", debug_fourcc(in_id));

    let size = get_audio_session_property_size(in_id);

    if !out_data_size.is_null() {
        env.mem.write(out_data_size, size);
    }

    kAudioSessionNoErr
}

pub fn AudioSessionGetProperty(
    env: &mut Environment,
    in_id: AudioSessionPropertyID,
    io_data_size: MutPtr<u32>,
    out_data: MutVoidPtr,
) -> OSStatus {
    log_dbg!("AudioSessionGetProperty {}", debug_fourcc(in_id));

    if out_data.is_null() {
        return crate::frameworks::carbon_core::paramErr;
    }

    let size = get_audio_session_property_size(in_id);

    if !io_data_size.is_null() {
        let provided_size = env.mem.read(io_data_size);
        if provided_size < size {
            return kAudioSessionBadPropertySizeError;
        }
        env.mem.write(io_data_size, size);
    }

    let session = &env.framework_state.audio_toolbox.audio_session;

    match in_id {
        kAudioSessionProperty_OtherAudioIsPlaying => {
            env.mem.write(out_data.cast::<u32>(), 0);
        }
        kAudioSessionProperty_AudioCategory => {
            env.mem.write(out_data.cast::<u32>(), session.category);
        }
        kAudioSessionProperty_CurrentHardwareSampleRate => {
            env.mem
                .write(out_data.cast::<f64>(), session.current_hardware_sample_rate);
        }
        kAudioSessionProperty_PreferredHardwareSampleRate => {
            env.mem.write(
                out_data.cast::<f64>(),
                session.preferred_hardware_sample_rate,
            );
        }
        kAudioSessionProperty_CurrentHardwareInputNumberChannels => {
            env.mem.write(out_data.cast::<u32>(), 0);
        }
        kAudioSessionProperty_CurrentHardwareOutputNumberChannels => {
            env.mem.write(
                out_data.cast::<u32>(),
                session.current_hardware_output_number_channels,
            );
        }
        kAudioSessionProperty_CurrentHardwareOutputVolume => {
            env.mem.write(
                out_data.cast::<f32>(),
                session.current_hardware_output_volume,
            );
        }
        kAudioSessionProperty_CurrentHardwareIOBufferDuration => {
            env.mem.write(
                out_data.cast::<f32>(),
                session.current_hardware_io_buffer_duration,
            );
        }
        kAudioSessionProperty_PreferredHardwareIOBufferDuration => {
            env.mem.write(
                out_data.cast::<f32>(),
                session.preferred_hardware_io_buffer_duration,
            );
        }
        kAudioSessionProperty_AudioInputAvailable => {
            env.mem.write(out_data.cast::<u32>(), 0);
        }
        kAudioSessionProperty_AudioRoute => {
            env.mem.write(out_data.cast::<u32>(), 0);
        }
        kAudioSessionProperty_OverrideCategoryMixWithOthers => {
            env.mem
                .write(out_data.cast::<u32>(), session.mix_with_others);
        }
        kAudioSessionProperty_OverrideCategoryDefaultToSpeaker => {
            env.mem
                .write(out_data.cast::<u32>(), session.default_to_speaker);
        }
        kAudioSessionProperty_OverrideCategoryEnableBluetoothInput => {
            env.mem
                .write(out_data.cast::<u32>(), session.bluetooth_input);
        }
        kAudioSessionProperty_OtherMixableAudioShouldDuck => {
            env.mem
                .write(out_data.cast::<u32>(), session.duck_others);
        }
        _ => {
            log!(
                "TODO: AudioSessionGetProperty UNIMPLEMENTED write for {}",
                debug_fourcc(in_id)
            );
            if size == 4 {
                env.mem.write(out_data.cast::<u32>(), 0);
            } else if size == 8 {
                env.mem.write(out_data.cast::<u64>(), 0);
            }
        }
    }

    kAudioSessionNoErr
}

pub fn AudioSessionSetProperty(
    env: &mut Environment,
    in_id: AudioSessionPropertyID,
    in_data_size: u32,
    in_data: ConstVoidPtr,
) -> OSStatus {
    log_dbg!(
        "AudioSessionSetProperty {} ({} bytes)",
        debug_fourcc(in_id),
        in_data_size
    );

    if in_data.is_null() {
        return crate::frameworks::carbon_core::paramErr;
    }

    let expected_size = get_audio_session_property_size(in_id);
    if in_data_size < expected_size {
        return kAudioSessionBadPropertySizeError;
    }

    let session = &mut env.framework_state.audio_toolbox.audio_session;

    match in_id {
        kAudioSessionProperty_AudioCategory => {
            let category = env.mem.read(in_data.cast::<u32>());
            log_dbg!(
                "AudioSessionSetProperty AudioCategory -> {}",
                debug_fourcc(category)
            );
            session.category = category;
        }
        kAudioSessionProperty_PreferredHardwareSampleRate => {
            let rate = env.mem.read(in_data.cast::<f64>());
            log_dbg!(
                "AudioSessionSetProperty PreferredHardwareSampleRate -> {}",
                rate
            );
            session.preferred_hardware_sample_rate = rate;
        }
        kAudioSessionProperty_PreferredHardwareIOBufferDuration => {
            let dur = env.mem.read(in_data.cast::<f32>());
            log_dbg!(
                "AudioSessionSetProperty PreferredHardwareIOBufferDuration -> {}",
                dur
            );
            session.preferred_hardware_io_buffer_duration = dur;
        }
        kAudioSessionProperty_OverrideAudioRoute => {
            // Значение игнорируем (у нас один фиксированный маршрут), но не
            // жалуемся.
        }
        kAudioSessionProperty_OverrideCategoryMixWithOthers => {
            session.mix_with_others = env.mem.read(in_data.cast::<u32>());
        }
        kAudioSessionProperty_OverrideCategoryDefaultToSpeaker => {
            session.default_to_speaker = env.mem.read(in_data.cast::<u32>());
        }
        kAudioSessionProperty_OverrideCategoryEnableBluetoothInput => {
            session.bluetooth_input = env.mem.read(in_data.cast::<u32>());
        }
        kAudioSessionProperty_OtherMixableAudioShouldDuck => {
            session.duck_others = env.mem.read(in_data.cast::<u32>());
            log_dbg!(
                "AudioSessionSetProperty OtherMixableAudioShouldDuck -> {}",
                session.duck_others
            );
        }
        _ => {
            log!(
                "TODO: AudioSessionSetProperty UNIMPLEMENTED {}",
                debug_fourcc(in_id)
            );
        }
    }

    kAudioSessionNoErr
}

/// Per Apple Audio Session Services Reference:
/// AudioSessionAddPropertyListener registers a callback to be invoked when
/// an audio session property changes (e.g., route change, interruption).
/// Since the emulator has a fixed audio route and no interruptions,
/// we store the listener but never invoke it. Return success so apps
/// proceed with their initialization.
pub fn AudioSessionAddPropertyListener(
    _env: &mut Environment,
    in_id: AudioSessionPropertyID,
    _in_proc: AudioSessionPropertyListener,
    _in_client_data: ConstVoidPtr,
) -> OSStatus {
    log_dbg!(
        "AudioSessionAddPropertyListener({}) — registered (will not fire)",
        debug_fourcc(in_id)
    );
    kAudioSessionNoErr
}

pub fn AudioSessionRemovePropertyListener(
    _env: &mut Environment,
    in_id: AudioSessionPropertyID,
) -> OSStatus {
    log_dbg!(
        "TODO: AudioSessionRemovePropertyListener {}",
        debug_fourcc(in_id)
    );
    kAudioSessionNoErr
}

// Recommended replacement for AudioSessionAddPropertyListener /
// -RemovePropertyListener,
// see Apple's `Audio Session Support` (`AudioToolbox/AudioServices.h`).
//
// The signature mirrors the other `…WithUserData` Audio Toolbox APIs
// (e.g. `AudioUnitAddPropertyListenerWithUserData`):
//
//     extern OSStatus
//     AudioSessionAddPropertyListenerWithUserData(
//         AudioSessionPropertyID         inID,
//         AudioSessionPropertyListener   inProc,
//         void                          *inUserData);
//
// PvZ (com.popcap.PvZ) uses the `WithUserData` remove variant for cleanup;
// without an export the load-time non-lazy bind fails and any subsequent
// invocation jumps to a NULL pointer. Provide a no-op stub matching the
// existing listener stubs so the linker is satisfied.
pub fn AudioSessionAddPropertyListenerWithUserData(
    _env: &mut Environment,
    in_id: AudioSessionPropertyID,
    _in_proc: AudioSessionPropertyListener,
    _in_user_data: ConstVoidPtr,
) -> OSStatus {
    log_dbg!(
        "TODO: AudioSessionAddPropertyListenerWithUserData {}",
        debug_fourcc(in_id)
    );
    kAudioSessionNoErr
}

pub fn AudioSessionRemovePropertyListenerWithUserData(
    _env: &mut Environment,
    in_id: AudioSessionPropertyID,
    _in_proc: AudioSessionPropertyListener,
    _in_user_data: ConstVoidPtr,
) -> OSStatus {
    log_dbg!(
        "TODO: AudioSessionRemovePropertyListenerWithUserData {}",
        debug_fourcc(in_id)
    );
    kAudioSessionNoErr
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioSessionInitialize(_, _, _, _)),
    export_c_func!(AudioSessionGetProperty(_, _, _)),
    export_c_func!(AudioSessionGetPropertySize(_, _)),
    export_c_func!(AudioSessionSetProperty(_, _, _)),
    export_c_func!(AudioSessionSetActive(_)),
    export_c_func!(AudioSessionAddPropertyListener(_, _, _)),
    export_c_func!(AudioSessionRemovePropertyListener(_)),
    export_c_func!(AudioSessionAddPropertyListenerWithUserData(_, _, _)),
    export_c_func!(AudioSessionRemovePropertyListenerWithUserData(_, _, _)),
];
