/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
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
use crate::objc::nil;

const kAudioUnitType_Output: u32 = fourcc(b"auou");
const kAudioUnitSubType_RemoteIO: u32 = fourcc(b"rioc");
const kAudioUnitManufacturer_Apple: u32 = fourcc(b"appl");

#[derive(Default)]
pub struct State {
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
    pub global_stream_format: AudioStreamBasicDescription,
    pub input_stream_format: Option<AudioStreamBasicDescription>,
    pub output_stream_format: Option<AudioStreamBasicDescription>,
    pub render_callback: Option<AURenderCallbackStruct>,
    pub last_render_time: Option<Instant>,
    pub al_source: Option<ALuint>,
}
impl Default for AudioComponentInstanceHostObject {
    fn default() -> Self {
        // Default values obtained from an iPod Touch 4 running iOS 6.1.6
        // through a test app built targetting iOS 2.0
        AudioComponentInstanceHostObject {
            started: false,
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
struct OpaqueAudioComponent {}
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
    let audio_comp_descr = env.mem.read(in_desc);
    assert!(audio_comp_descr.component_type == kAudioUnitType_Output);
    assert!(audio_comp_descr.component_sub_type == kAudioUnitSubType_RemoteIO);
    assert!(audio_comp_descr.component_manufacturer == kAudioUnitManufacturer_Apple);

    let out_component = nil.cast();
    log!(
        "TODO: AudioComponentFindNext({:?}, {:?}) -> {:?}",
        in_component,
        in_desc,
        out_component
    );
    out_component
}

fn AudioComponentInstanceNew(
    env: &mut Environment,
    in_component: AudioComponent,
    out_instance: MutPtr<AudioComponentInstance>,
) -> OSStatus {
    let host_object = AudioComponentInstanceHostObject::default();

    let guest_instance: AudioComponentInstance = env
        .mem
        .alloc_and_write(OpaqueAudioComponentInstance { _pad: 0 });
    State::get(&mut env.framework_state)
        .audio_component_instances
        .insert(guest_instance, host_object);

    env.mem.write(out_instance, guest_instance);

    let result = 0; // success
    log_dbg!(
        "AudioComponentInstanceNew({:?}, {:?}) -> {:?}",
        in_component,
        out_instance,
        result
    );
    result
}

fn AudioComponentInstanceDispose(
    env: &mut Environment,
    in_instance: AudioComponentInstance,
) -> OSStatus {
    let result = if in_instance.is_null() {
        paramErr
    } else {
        State::get(&mut env.framework_state)
            .audio_component_instances
            .remove(&in_instance);
        env.mem.free(in_instance.cast());
        0
    };
    log_dbg!(
        "AudioComponentInstanceDispose({:?}) -> {:?}",
        in_instance,
        result
    );
    result
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioComponentFindNext(_, _)),
    export_c_func!(AudioComponentInstanceNew(_, _)),
    export_c_func!(AudioComponentInstanceDispose(_)),
];
