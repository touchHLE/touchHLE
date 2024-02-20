use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::frameworks::audio_toolbox::audio_components;
use crate::frameworks::carbon_core::OSStatus;
use crate::mem::{ConstVoidPtr, MutPtr, MutVoidPtr};

use super::audio_components::AudioComponentInstance;

type AudioUnit = AudioComponentInstance;
type AudioUnitPropertyID = u32;
type AudioUnitScope = u32;
type AudioUnitElement = u32;

const kAudioUnitProperty_SetRenderCallback: AudioUnitPropertyID = 23;
const kAudioUnitProperty_StreamFormat: AudioUnitPropertyID = 8;

fn AudioUnitInitialize(_env: &mut Environment, inUnit: AudioUnit) -> OSStatus {
    log!("TODO: AudioUnitInitialize({:?})", inUnit);
    0 // success
}

fn AudioUnitSetProperty(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_ID: AudioUnitPropertyID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    in_data: ConstVoidPtr,
    in_data_size: u32,
) -> OSStatus {
    let _host_object = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&in_unit)
        .unwrap();

    match in_ID {
        kAudioUnitProperty_SetRenderCallback => {
            log!(
                "TODO: AudioUnitSetProperty({:?}, kAudioUnitProperty_SetRenderCallback, {:?}, {:?}, {:?}, {:?})",
                in_unit,
                in_scope,
                in_element,
                in_data,
                in_data_size
            );
        }
        kAudioUnitProperty_StreamFormat => {
            log!(
                "TODO: AudioUnitSetProperty({:?}, kAudioUnitProperty_StreamFormat, {:?}, {:?}, {:?}, {:?})",
                in_unit,
                in_scope,
                in_element,
                in_data,
                in_data_size
            );
        }
        _ => unimplemented!(),
    };

    0 // success
}

fn AudioUnitGetProperty(
    _env: &mut Environment,
    in_unit: AudioUnit,
    in_ID: AudioUnitPropertyID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    out_data: MutVoidPtr,
    io_data_size: MutPtr<u32>,
) -> OSStatus {
    match in_ID {
        kAudioUnitProperty_StreamFormat => {
            log!(
                "TODO: AudioUnitGetProperty({:?}, kAudioUnitProperty_StreamFormat, {:?}, {:?}, {:?}, {:?})",
                in_unit,
                in_scope,
                in_element,
                out_data,
                io_data_size
            );
        }
        _ => unimplemented!(),
    };
    0 // success
}

fn AudioOutputUnitStart(env: &mut Environment, ci: AudioUnit) -> OSStatus {
    audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&ci)
        .unwrap()
        .started = true;
    0 // success
}

fn AudioOutputUnitStop(env: &mut Environment, ci: AudioUnit) -> OSStatus {
    audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&ci)
        .unwrap()
        .started = false;
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioUnitInitialize(_)),
    export_c_func!(AudioUnitSetProperty(_, _, _, _, _, _)),
    export_c_func!(AudioUnitGetProperty(_, _, _, _, _, _)),
    export_c_func!(AudioOutputUnitStart(_)),
    export_c_func!(AudioOutputUnitStop(_)),
];
