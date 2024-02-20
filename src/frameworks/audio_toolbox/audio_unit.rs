use std::mem::size_of;

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::frameworks::audio_toolbox::audio_components;
use crate::frameworks::carbon_core::OSStatus;
use crate::frameworks::core_audio_types::AudioStreamBasicDescription;
use crate::mem::{ConstVoidPtr, MutPtr, MutVoidPtr};

use super::audio_components::{AURenderCallbackStruct, AudioComponentInstance};

type AudioUnit = AudioComponentInstance;
type AudioUnitPropertyID = u32;
type AudioUnitScope = u32;
type AudioUnitElement = u32;

// TODO: Other scopes
const kAudioUnitScope_Global: AudioUnitScope = 0;
const kAudioUnitScope_Output: AudioUnitScope = 2;

const kAudioUnitProperty_SetRenderCallback: AudioUnitPropertyID = 23;
const kAudioUnitProperty_StreamFormat: AudioUnitPropertyID = 8;

fn AudioUnitInitialize(_env: &mut Environment, inUnit: AudioUnit) -> OSStatus {
    log!("TODO: AudioUnitInitialize({:?})", inUnit);
    0 // success
}

fn AudioUnitSetProperty(
    env: &mut Environment,
    inUnit: AudioUnit,
    inID: AudioUnitPropertyID,
    inScope: AudioUnitScope,
    inElement: AudioUnitElement,
    inData: ConstVoidPtr,
    inDataSize: u32,
) -> OSStatus {
    assert!(inElement == 0);

    let host_object = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&inUnit)
        .unwrap();

    let result;
    match inID {
        kAudioUnitProperty_SetRenderCallback => {
            assert!(inScope == kAudioUnitScope_Global);
            assert!(inDataSize == size_of::<AURenderCallbackStruct>().try_into().unwrap());
            let render_callback = env.mem.read(inData.cast::<AURenderCallbackStruct>());
            host_object.render_callback = Some(render_callback);
            result = 0;
            log_dbg!("AudioUnitSetProperty({:?}, kAudioUnitProperty_SetRenderCallback, {:?}, {:?}, {:?}, {:?}) -> {:?}", inUnit, inScope, inElement, render_callback, inDataSize, result);
        }
        kAudioUnitProperty_StreamFormat => {
            assert!(inDataSize == size_of::<AudioStreamBasicDescription>().try_into().unwrap());
            let stream_format = env.mem.read(inData.cast::<AudioStreamBasicDescription>());
            let bytes_per_channel = stream_format.bits_per_channel / 8;
            let actual_bytes_per_frame = stream_format.channels_per_frame * bytes_per_channel;
            if actual_bytes_per_frame != stream_format.bytes_per_packet {
                log!(
                    "Warning: Stream format has non-sensical values: {:?}",
                    stream_format
                );
            }
            match inScope {
                kAudioUnitScope_Global => host_object.global_stream_format = stream_format,
                kAudioUnitScope_Output => host_object.output_stream_format = Some(stream_format),
                _ => unimplemented!(),
            };
            result = 0;
            log_dbg!("AudioUnitSetProperty({:?}, kAudioUnitProperty_StreamFormat, {:?}, {:?}, {:?}, {:?}) -> {:?}", inUnit, inScope, inElement, stream_format, inDataSize, result);
        }
        _ => unimplemented!(),
    };

    result
}

fn AudioUnitGetProperty(
    _env: &mut Environment,
    inUnit: AudioUnit,
    inID: AudioUnitPropertyID,
    inScope: AudioUnitScope,
    inElement: AudioUnitElement,
    outData: MutVoidPtr,
    ioDataSize: MutPtr<u32>,
) -> OSStatus {
    match inID {
        kAudioUnitProperty_StreamFormat => {
            log!(
                "TODO: AudioUnitGetProperty({:?}, kAudioUnitProperty_StreamFormat, {:?}, {:?}, {:?}, {:?})",
                inUnit,
                inScope,
                inElement,
                outData,
                ioDataSize
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
