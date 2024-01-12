use std::mem::size_of;
use std::time::Instant;

use touchHLE_openal_soft_wrapper::al_types::{ALuint, ALvoid};
use touchHLE_openal_soft_wrapper::{
    alBufferData, alDeleteBuffers, alDeleteSources, alGenBuffers, alGenSources, alGetError,
    alGetSourcei, alSourcePlay, alSourceQueueBuffers, alSourceUnqueueBuffers, AL_BUFFERS_PROCESSED,
    AL_FORMAT_MONO16, AL_FORMAT_MONO8, AL_FORMAT_STEREO16, AL_FORMAT_STEREO8, AL_PLAYING,
    AL_SOURCE_STATE,
};

use crate::abi::CallFromHost;
use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::frameworks::audio_toolbox::audio_components;
use crate::frameworks::carbon_core::OSStatus;
use crate::frameworks::core_audio_types::{kAudioFormatLinearPCM, AudioStreamBasicDescription};
use crate::frameworks::core_foundation::cf_run_loop::CFRunLoopGetMain;
use crate::frameworks::foundation::ns_run_loop;
use crate::mem::{ConstVoidPtr, MutPtr, MutVoidPtr, SafeRead};
use crate::objc::nil;

use super::audio_components::{AURenderCallbackStruct, AudioComponentInstance};
use super::audio_session;

pub type AudioUnit = AudioComponentInstance;
type AudioUnitPropertyID = u32;
type AudioUnitScope = u32;
type AudioUnitElement = u32;

#[repr(C, packed)]
struct AudioBufferList<const COUNT: usize> {
    mNumberBuffers: u32,
    mBuffers: [AudioBuffer; COUNT],
}
unsafe impl SafeRead for AudioBufferList<2> {}

#[repr(C, packed)]
struct AudioBuffer {
    mNumberChannels: u32,
    mDataByteSize: u32,
    mData: MutVoidPtr,
}

// TODO: Other scopes
const kAudioUnitScope_Global: AudioUnitScope = 0;
const kAudioUnitScope_Output: AudioUnitScope = 2;

const kAudioUnitProperty_SetRenderCallback: AudioUnitPropertyID = 23;
const kAudioUnitProperty_StreamFormat: AudioUnitPropertyID = 8;

fn AudioUnitInitialize(env: &mut Environment, inUnit: AudioUnit) -> OSStatus {
    let run_loop = CFRunLoopGetMain(env);
    ns_run_loop::add_audio_unit(env, run_loop, inUnit);
    0 // success
}

fn AudioUnitUninitialize(env: &mut Environment, inUnit: AudioUnit) -> OSStatus {
    let run_loop = CFRunLoopGetMain(env);
    ns_run_loop::remove_audio_unit(env, run_loop, inUnit);
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
    env: &mut Environment,
    inUnit: AudioUnit,
    inID: AudioUnitPropertyID,
    inScope: AudioUnitScope,
    inElement: AudioUnitElement,
    outData: MutVoidPtr,
    ioDataSize: MutPtr<u32>,
) -> OSStatus {
    assert!(inElement == 0);

    let host_object = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&inUnit)
        .unwrap();

    match inID {
        kAudioUnitProperty_StreamFormat => {
            assert!(
                env.mem.read(ioDataSize)
                    == size_of::<AudioStreamBasicDescription>().try_into().unwrap()
            );
            let stream_format = match inScope {
                kAudioUnitScope_Global => host_object.global_stream_format,
                kAudioUnitScope_Output => host_object.output_stream_format.unwrap(),
                _ => unimplemented!(),
            };
            env.mem.write(outData.cast(), stream_format);
            env.mem.write(
                ioDataSize.cast(),
                u32::try_from(size_of::<AudioStreamBasicDescription>()).unwrap(),
            );
        }
        _ => unimplemented!(),
    };
    0 // success
}

fn AudioOutputUnitStart(env: &mut Environment, ci: AudioUnit) -> OSStatus {
    let _context_manager = env.framework_state.audio_toolbox.make_al_context_current();

    let mut source: ALuint = 0;
    unsafe {
        alGenSources(1, &mut source);
        alSourcePlay(source);
        assert_eq!(alGetError(), 0);
    }

    let audio_components_state = audio_components::State::get(&mut env.framework_state);
    let audio_unit_state = audio_components_state
        .audio_component_instances
        .get_mut(&ci)
        .unwrap();
    audio_unit_state.al_source = Some(source);
    audio_unit_state.last_render_time = Some(Instant::now());
    audio_unit_state.started = true;

    let result = 0; // Success
    log_dbg!("AudioOutputUnitStart({:?}) -> {:?}", ci, result);
    result
}

fn AudioOutputUnitStop(env: &mut Environment, ci: AudioUnit) -> OSStatus {
    let _context_manager = env.framework_state.audio_toolbox.make_al_context_current();

    let audio_components_state = audio_components::State::get(&mut env.framework_state);
    let audio_unit_state = audio_components_state
        .audio_component_instances
        .get_mut(&ci)
        .unwrap();
    audio_unit_state.started = false;
    audio_unit_state.last_render_time = None;

    if let Some(al_source) = audio_unit_state.al_source {
        unsafe {
            alDeleteSources(1, &al_source);
            assert_eq!(alGetError(), 0);
        }
    }
    audio_unit_state.al_source = None;

    let result = 0; // Success
    log_dbg!("AudioOutputUnitStop({:?}) -> {:?}", ci, result);
    result
}

pub fn render_audio_unit(env: &mut Environment, audio_unit: AudioUnit) {
    let _context_manager = env.framework_state.audio_toolbox.make_al_context_current();

    let audio_session::State {
        current_hardware_sample_rate,
        ..
    } = env.framework_state.audio_toolbox.audio_session;

    let audio_components_state = audio_components::State::get(&mut env.framework_state);
    let audio_unit_host_object = audio_components_state
        .audio_component_instances
        .get(&audio_unit)
        .unwrap();

    if !audio_unit_host_object.started {
        return;
    }

    let al_source = audio_unit_host_object.al_source.unwrap();
    let mut al_buffers = Vec::new();
    unsafe {
        let mut buffers_processed = 0;
        alGetSourcei(al_source, AL_BUFFERS_PROCESSED, &mut buffers_processed);
        while buffers_processed > 0 {
            let mut al_buffer = 0;
            alSourceUnqueueBuffers(al_source, 1, &mut al_buffer);
            al_buffers.push(al_buffer);
            alGetSourcei(al_source, AL_BUFFERS_PROCESSED, &mut buffers_processed);
        }
        assert_eq!(alGetError(), 0);
    }

    let stream_format = audio_unit_host_object
        .output_stream_format
        .unwrap_or(audio_unit_host_object.global_stream_format);

    // TODO: Unify with audio_queue and support more formats
    assert!(stream_format.format_id == kAudioFormatLinearPCM);

    let bytes_per_channel = stream_format.bits_per_channel / 8;
    let actual_bytes_per_frame = stream_format.channels_per_frame * bytes_per_channel;

    let now = Instant::now();
    // Calculate number of frames by checking how much time passed since
    // the last render. Limit to 100ms to prevent delay from adding up
    // if it's been too long since the last render.
    // TODO: Verify if this behavior is right
    let elapsed_time = now.duration_since(audio_unit_host_object.last_render_time.unwrap());
    let number_frames = (elapsed_time.as_secs_f64().min(0.1) * current_hardware_sample_rate) as u32;
    let buffer_size = number_frames * actual_bytes_per_frame;

    // Alloc callback arguments
    let action_flags = env.mem.alloc_and_write(0);

    let buffer1Data = env.mem.alloc(buffer_size);
    let buffer2Data = env.mem.alloc(buffer_size);
    let audio_buffer_list: AudioBufferList<2> = AudioBufferList {
        mNumberBuffers: 2,
        mBuffers: [
            AudioBuffer {
                mNumberChannels: 2,
                mDataByteSize: buffer_size,
                mData: buffer1Data,
            },
            AudioBuffer {
                mNumberChannels: 2,
                mDataByteSize: buffer_size,
                mData: buffer2Data,
            },
        ],
    };
    let audio_buffer_list = env.mem.alloc_and_write(audio_buffer_list);

    // Run render callback
    let AURenderCallbackStruct {
        inputProc,
        inputProcRefCon,
    } = audio_unit_host_object.render_callback.unwrap();
    let () = inputProc.call_from_host(
        env,
        (
            inputProcRefCon,
            action_flags,
            nil.cast_void().cast_const(),
            0u32,
            number_frames,
            audio_buffer_list.cast_void(),
        ),
    );

    // Read and play data written by the callback
    // TODO: Figure out why the buffers come in this way
    let data: Vec<u8> = env
        .mem
        .bytes_at(buffer1Data.cast(), buffer_size)
        .chunks(actual_bytes_per_frame as usize)
        .flat_map(|frame| {
            let mut frame = frame.to_owned();
            // Change from big to little endian
            frame.reverse();
            // Fetch only frame bytes
            frame[0..(stream_format.bytes_per_frame as usize)].to_owned()
        })
        .collect();

    let al_format = match (
        stream_format.bytes_per_frame / bytes_per_channel,
        stream_format.bits_per_channel,
    ) {
        (1, 8) => AL_FORMAT_MONO8,
        (1, 16) => AL_FORMAT_MONO16,
        (2, 8) => AL_FORMAT_STEREO8,
        (2, 16) => AL_FORMAT_STEREO16,
        _ => unreachable!(),
    };

    unsafe {
        // Get an unqueued buffer or create a new one
        let al_buffer = al_buffers.pop().unwrap_or_else(|| {
            let mut al_buffer = 0;
            alGenBuffers(1, &mut al_buffer);
            al_buffer
        });

        alBufferData(
            al_buffer,
            al_format,
            data.as_ptr() as *const ALvoid,
            data.len().try_into().unwrap(),
            current_hardware_sample_rate as i32,
        );
        alSourceQueueBuffers(al_source, 1, &al_buffer);

        let mut al_source_state = 0;
        alGetSourcei(al_source, AL_SOURCE_STATE, &mut al_source_state);
        if al_source_state != AL_PLAYING {
            alSourcePlay(al_source);
        }

        // TODO: Play buffer 2 (In RE4 its the same as buffer 1 though)

        // Clear unused buffers
        if !al_buffers.is_empty() {
            alDeleteBuffers(al_buffers.len() as i32, al_buffers.as_ptr());
        }

        assert_eq!(alGetError(), 0);
    }

    // TODO: Do something with the action flags?
    env.mem.free(action_flags.cast_void());

    env.mem.free(buffer1Data.cast_void());
    env.mem.free(buffer2Data.cast_void());

    env.mem.free(audio_buffer_list.cast_void());

    // Reborrow as mutable to update the last render time
    audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&audio_unit)
        .unwrap()
        .last_render_time = Some(now);
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioUnitInitialize(_)),
    export_c_func!(AudioUnitUninitialize(_)),
    export_c_func!(AudioUnitSetProperty(_, _, _, _, _, _)),
    export_c_func!(AudioUnitGetProperty(_, _, _, _, _, _)),
    export_c_func!(AudioOutputUnitStart(_)),
    export_c_func!(AudioOutputUnitStop(_)),
];
