/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AudioComponent.h` (Audio Component Services)

use std::collections::HashMap;
use std::time::Instant;

use crate::abi::GuestFunction;
use crate::audio::openal::al_types::ALuint;
use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::frameworks::carbon_core::{paramErr, OSStatus};
use crate::frameworks::core_audio_types::{
    fourcc, kAudioFormatFlagIsAlignedHigh, kAudioFormatFlagIsFloat, kAudioFormatFlagIsPacked,
    kAudioFormatFlagIsSignedInteger, kAudioFormatLinearPCM, AudioStreamBasicDescription,
};
use crate::mem::{ConstPtr, ConstVoidPtr, MutPtr, SafeRead};

const kAudioUnitType_Output: u32 = fourcc(b"auou");
const kAudioUnitSubType_RemoteIO: u32 = fourcc(b"rioc");
const kAudioUnitManufacturer_Apple: u32 = fourcc(b"appl");
const kAudioUnitType_Mixer: u32 = fourcc(b"aumx");
const kAudioUnitSubType_3DMixer: u32 = fourcc(b"3dem");

// --- 3D Mixer Structures ---
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MixerDistanceParams {
    pub reference_distance: f32,
    pub maximum_distance: f32,
    pub rolloff_factor: f32,
}

// ИСПРАВЛЕНИЕ: Реализация SafeRead для возможности чтения из памяти
unsafe impl SafeRead for MixerDistanceParams {}

#[derive(Clone)]
pub struct MixerBusState {
    pub al_source: Option<ALuint>,
    pub position: [f32; 3], // X, Y, Z
    pub volume: f32,
    pub distance_params: MixerDistanceParams,
    pub render_callback: Option<AURenderCallbackStruct>,
    pub stream_format: Option<AudioStreamBasicDescription>,
    pub last_render_time: Option<Instant>,
}

impl Default for MixerBusState {
    fn default() -> Self {
        Self {
            al_source: None,
            position: [0.0, 0.0, 0.0],
            volume: 1.0,
            distance_params: MixerDistanceParams {
                reference_distance: 1.0,
                maximum_distance: 100000.0,
                rolloff_factor: 1.0,
            },
            render_callback: None,
            stream_format: None,
            last_render_time: None,
        }
    }
}
// ---------------------------

#[derive(Default)]
pub struct State {
    pub audio_component: AudioComponent,
    pub audio_component_instances:
        HashMap<AudioComponentInstance, AudioComponentInstanceHostObject>,
}
impl State {
    pub fn get(framework_state: &mut crate::frameworks::State) -> &mut Self {
        &mut framework_state.audio_toolbox.audio_components
    }
}

#[derive(Clone)]
pub struct AudioComponentInstanceHostObject {
    pub started: bool,
    pub maximum_frames_per_slice: u32,
    pub global_stream_format: AudioStreamBasicDescription,
    pub input_stream_format: Option<AudioStreamBasicDescription>,
    pub output_stream_format: Option<AudioStreamBasicDescription>,
    pub render_callback: Option<AURenderCallbackStruct>,
    pub last_render_time: Option<Instant>,
    pub al_source: Option<ALuint>,
    pub is_running_handler: bool,

    // --- 3D Mixer State ---
    pub is_3d_mixer: bool,
    pub mixer_buses: HashMap<u32, MixerBusState>,
}

impl Default for AudioComponentInstanceHostObject {
    fn default() -> Self {
        AudioComponentInstanceHostObject {
            started: false,
            maximum_frames_per_slice: 1024,
            global_stream_format: AudioStreamBasicDescription {
                sample_rate: 44100.0,
                format_id: kAudioFormatLinearPCM,
                format_flags: kAudioFormatFlagIsFloat
                    | kAudioFormatFlagIsSignedInteger
                    | kAudioFormatFlagIsPacked
                    | kAudioFormatFlagIsAlignedHigh,
                bytes_per_packet: 4,
                frames_per_packet: 1,
                bytes_per_frame: 4,
                channels_per_frame: 2,
                bits_per_channel: 32,
                _reserved: 0,
            },
            input_stream_format: None,
            output_stream_format: None,
            render_callback: None,
            last_render_time: None,
            al_source: None,
            is_running_handler: false,
            is_3d_mixer: false,
            mixer_buses: HashMap::new(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
#[allow(dead_code)]
pub struct AURenderCallbackStruct {
    pub input_proc: AURenderCallback,
    pub input_proc_ref_con: ConstVoidPtr,
}
unsafe impl SafeRead for AURenderCallbackStruct {}

#[repr(C, packed)]
pub struct OpaqueAudioComponent {
    _pad: u8,
}
unsafe impl SafeRead for OpaqueAudioComponent {}

type AudioComponent = MutPtr<OpaqueAudioComponent>;
pub type AURenderCallback = GuestFunction;

#[repr(C, packed)]
pub struct OpaqueAudioComponentInstance {
    _pad: u8,
}
unsafe impl SafeRead for OpaqueAudioComponentInstance {}

pub type AudioComponentInstance = MutPtr<OpaqueAudioComponentInstance>;

#[repr(C, packed)]
struct AudioComponentDescription {
    component_type: u32,
    component_sub_type: u32,
    component_manufacturer: u32,
    component_flags: u32,
    component_flags_mask: u32,
}
unsafe impl SafeRead for AudioComponentDescription {}

fn AudioComponentFindNext(
    env: &mut Environment,
    in_component: AudioComponent,
    in_desc: ConstPtr<AudioComponentDescription>,
) -> AudioComponent {
    assert!(in_component.is_null());
    let audio_comp_descr = env.mem.read::<_, false>(in_desc);

    let comp_type = audio_comp_descr.component_type;
    let comp_sub_type = audio_comp_descr.component_sub_type;
    let comp_manufacturer = audio_comp_descr.component_manufacturer;

    let is_remote_io = comp_type == kAudioUnitType_Output
        && comp_sub_type == kAudioUnitSubType_RemoteIO
        && comp_manufacturer == kAudioUnitManufacturer_Apple;

    let is_3d_mixer = comp_type == kAudioUnitType_Mixer
        && comp_sub_type == kAudioUnitSubType_3DMixer
        && comp_manufacturer == kAudioUnitManufacturer_Apple;

    if !is_remote_io && !is_3d_mixer {
        log!(
            "AudioComponentFindNext: unsupported component type={:#010x} sub_type={:#010x} manufacturer={:#010x}, returning null",
            comp_type,
            comp_sub_type,
            comp_manufacturer,
        );
        return MutPtr::null();
    }

    let state = State::get(&mut env.framework_state);
    if state.audio_component.is_null() {
        state.audio_component = env.mem.alloc_and_write(OpaqueAudioComponent { _pad: 0 });
    }

    log_dbg!(
        "AudioComponentFindNext: matched type=0x{:08x} sub_type=0x{:08x} manuf=0x{:08x} -> {:?}",
        comp_type,
        comp_sub_type,
        comp_manufacturer,
        state.audio_component
    );
    state.audio_component
}

fn AudioComponentInstanceNew(
    env: &mut Environment,
    in_component: AudioComponent,
    out_instance: MutPtr<AudioComponentInstance>,
) -> OSStatus {
    if in_component.is_null() {
        return paramErr;
    }

    let mut host_object = AudioComponentInstanceHostObject::default();
    host_object.is_3d_mixer = true;

    let guest_instance: AudioComponentInstance = env
        .mem
        .alloc_and_write(OpaqueAudioComponentInstance { _pad: 0 });

    State::get(&mut env.framework_state)
        .audio_component_instances
        .insert(guest_instance, host_object);

    env.mem.write(out_instance, guest_instance);

    log_dbg!(
        "AudioComponentInstanceNew(component={:?}) -> instance={:?}",
        in_component,
        guest_instance
    );
    0
}

/// Создать AudioUnit instance напрямую (используется из
//`au_graph::AUGraphOpen`),
/// минуя обычный путь `AudioComponentInstanceNew`.
pub fn create_audio_unit_instance(env: &mut Environment) -> AudioComponentInstance {
    let mut host_object = AudioComponentInstanceHostObject::default();
    host_object.is_3d_mixer = true;
    let guest_instance: AudioComponentInstance = env
        .mem
        .alloc_and_write(OpaqueAudioComponentInstance { _pad: 0 });
    State::get(&mut env.framework_state)
        .audio_component_instances
        .insert(guest_instance, host_object);
    guest_instance
}

fn AudioComponentInstanceDispose(
    env: &mut Environment,
    in_instance: AudioComponentInstance,
) -> OSStatus {
    log_dbg!("AudioComponentInstanceDispose({:?})", in_instance);
    if in_instance.is_null() {
        return paramErr;
    }

    State::get(&mut env.framework_state)
        .audio_component_instances
        .remove(&in_instance);
    env.mem.free(in_instance.cast());

    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioComponentFindNext(_, _)),
    export_c_func!(AudioComponentInstanceNew(_, _)),
    export_c_func!(AudioComponentInstanceDispose(_)),
];
