/*
 * Эта лицензия Source Code Form подпадает под условия Mozilla Public
 * License, v. 2.0.
 * Если копия MPL не распространялась вместе с этим
 * файлом, вы можете получить ее на https://mozilla.org/MPL/2.0/.
 */
//! `AudioUnit.h` (Audio Unit Services)

use std::time::Instant;

use crate::audio::openal::al_types::{ALuint, ALvoid};
use crate::audio::openal::{
    AL_BUFFERS_PROCESSED, AL_BUFFERS_QUEUED, AL_PLAYING, AL_SOURCE_STATE,
};

const AL_POSITION: i32 = 0x1004;
const AL_REFERENCE_DISTANCE: i32 = 0x1020;
const AL_ROLLOFF_FACTOR: i32 = 0x1021;
const AL_MAX_DISTANCE: i32 = 0x1023;

use crate::abi::CallFromHost;
use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::frameworks::audio_toolbox::audio_components;
use crate::frameworks::audio_toolbox::audio_queue::log_if_broken_audio_format;
use crate::frameworks::carbon_core::{paramErr, OSStatus};
use crate::frameworks::core_audio_types::{fourcc, AudioStreamBasicDescription};
use crate::frameworks::core_foundation::cf_run_loop::CFRunLoopGetMain;
use crate::frameworks::foundation::ns_run_loop;
use crate::mem::{guest_size_of, ConstVoidPtr, MutPtr, MutVoidPtr, SafeRead};
use crate::objc::nil;

use super::audio_components::{AURenderCallbackStruct, AudioComponentInstance};
use super::audio_queue::decode_buffer;

pub type AudioUnit = AudioComponentInstance;

type AudioUnitPropertyID = u32;
type AudioUnitScope = u32;
type AudioUnitElement = u32;
type AudioUnitParameterID = u32;
type AudioUnitParameterValue = f32;

// =========================================================================
// MARK: - Структуры
// =========================================================================

#[repr(C, packed)]
pub struct AudioBufferList<const COUNT: usize> {
    pub number_buffers: u32,
    pub buffers: [AudioBuffer; COUNT],
}
unsafe impl SafeRead for AudioBufferList<1> {}
unsafe impl SafeRead for AudioBufferList<2> {}

#[repr(C, packed)]
pub struct AudioBuffer {
    pub number_channels: u32,
    pub data_byte_size: u32,
    pub data: MutVoidPtr,
}

/// `AudioUnitConnection` — используется для kAudioUnitProperty_MakeConnection.
#[repr(C, packed)]
#[derive(Copy, Clone)]
struct AudioUnitConnection {
    source_audio_unit: AudioUnit,
    source_output_number: u32,
    dest_input_number: u32,
}
unsafe impl SafeRead for AudioUnitConnection {}

// =========================================================================
// MARK: - Константы Scope / element
// =========================================================================

const kAudioUnitScope_Global: AudioUnitScope = 0;
const kAudioUnitScope_Input: AudioUnitScope = 1;
const kAudioUnitScope_Output: AudioUnitScope = 2;
const kAudioUnitScope_Group: AudioUnitScope = 3;
const kAudioUnitScope_Part: AudioUnitScope = 4;
const kAudioUnitScope_Note: AudioUnitScope = 5;

// =========================================================================
// MARK: - Константы Property ID
// =========================================================================

const kAudioUnitProperty_ClassInfo: AudioUnitPropertyID = 0;
const kAudioUnitProperty_MakeConnection: AudioUnitPropertyID = 1;
const kAudioUnitProperty_SampleRate: AudioUnitPropertyID = 2;
const kAudioUnitProperty_ParameterList: AudioUnitPropertyID = 3;
const kAudioUnitProperty_ParameterInfo: AudioUnitPropertyID = 4;
const kAudioUnitProperty_CPULoad: AudioUnitPropertyID = 6;
const kAudioUnitProperty_StreamFormat: AudioUnitPropertyID = 8;
const kAudioUnitProperty_ElementCount: AudioUnitPropertyID = 11;
const kAudioUnitProperty_Latency: AudioUnitPropertyID = 12;
const kAudioUnitProperty_SupportedNumChannels: AudioUnitPropertyID = 13;
const kAudioUnitProperty_MaximumFramesPerSlice: AudioUnitPropertyID = 14;
const kAudioUnitProperty_ParameterValueStrings: AudioUnitPropertyID = 16;
const kAudioUnitProperty_AudioChannelLayout: AudioUnitPropertyID = 19;
const kAudioUnitProperty_TailTime: AudioUnitPropertyID = 20;
const kAudioUnitProperty_BypassEffect: AudioUnitPropertyID = 21;
const kAudioUnitProperty_LastRenderError: AudioUnitPropertyID = 22;
const kAudioUnitProperty_SetRenderCallback: AudioUnitPropertyID = 23;
const kAudioUnitProperty_FactoryPresets: AudioUnitPropertyID = 24;
const kAudioUnitProperty_RenderQuality: AudioUnitPropertyID = 26;
const kAudioUnitProperty_HostCallbacks: AudioUnitPropertyID = 27;
const kAudioUnitProperty_InPlaceProcessing: AudioUnitPropertyID = 29;
const kAudioUnitProperty_ElementName: AudioUnitPropertyID = 30;
const kAudioUnitProperty_SupportedChannelLayoutTags: AudioUnitPropertyID = 32;
const kAudioUnitProperty_PresentPreset: AudioUnitPropertyID = 36;
const kAudioUnitProperty_DependentParameters: AudioUnitPropertyID = 45;
const kAudioUnitProperty_InputSamplesInOutput: AudioUnitPropertyID = 49;
const kAudioUnitProperty_ShouldAllocateBuffer: AudioUnitPropertyID = 51;
const kAudioUnitProperty_FrequencyResponse: AudioUnitPropertyID = 52;
const kAudioUnitProperty_ParameterHistoryInfo: AudioUnitPropertyID = 53;
const kAudioUnitProperty_NickName: AudioUnitPropertyID = 54;
const kAudioUnitProperty_OfflineRender: AudioUnitPropertyID = 37;
const kAudioUnitProperty_ParameterIDName: AudioUnitPropertyID = 34;
const kAudioOutputUnitProperty_EnableIO: AudioUnitPropertyID = 2003;
const kAudioOutputUnitProperty_HasIO: AudioUnitPropertyID = 2006;
const kAudioOutputUnitProperty_StartTime: AudioUnitPropertyID = 2004;
const kAudioOutputUnitProperty_SetInputCallback: AudioUnitPropertyID = 2005;
const kAudioOutputUnitProperty_IsRunning: AudioUnitPropertyID = 2001;
const kAudioMixerProperty_Volume: AudioUnitPropertyID = 7;
const kAudioMixerProperty_Metering: AudioUnitPropertyID = 1003;
const kAudioUnitProperty_MeteringMode: AudioUnitPropertyID = 1003;

// 3D Mixer Property IDs
const kAudioUnitProperty_3DMixerDistanceParams: AudioUnitPropertyID = fourcc(b"3ddp");
const kAudioUnitProperty_MatrixLevels: AudioUnitPropertyID = fourcc(b"mxmv");
const kAudioUnitProperty_SpatializationAlgorithm: AudioUnitPropertyID = fourcc(b"spat");
const kAudioUnitProperty_3DMixerRenderingFlags: AudioUnitPropertyID = fourcc(b"3drf");

// 3D Mixer Parameter IDs
const k3DMixerParam_Azimuth: AudioUnitParameterID = 0;
const k3DMixerParam_Elevation: AudioUnitParameterID = 1;
const k3DMixerParam_Distance: AudioUnitParameterID = 2;

// =========================================================================
// MARK: - Инициализация / Деинициализация AudioUnit
// =========================================================================

fn AudioUnitInitialize(env: &mut Environment, in_unit: AudioUnit) -> OSStatus {
    log_dbg!("AudioUnitInitialize({:?})", in_unit);
    let run_loop = CFRunLoopGetMain(env);
    ns_run_loop::add_audio_unit(env, run_loop, in_unit);
    0
}

fn AudioUnitUninitialize(env: &mut Environment, in_unit: AudioUnit) -> OSStatus {
    let run_loop = CFRunLoopGetMain(env);
    match ns_run_loop::remove_audio_unit(env, run_loop, in_unit) {
        Ok(_) => 0,
        Err(_) => paramErr,
    }
}

// =========================================================================
// MARK: - Установка свойств AudioUnit
// =========================================================================

fn AudioUnitSetProperty(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_id: AudioUnitPropertyID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    in_data: ConstVoidPtr,
    in_data_size: u32,
) -> OSStatus {
    log_dbg!(
        "AudioUnitSetProperty(unit={:?}, prop={}, scope={}, element={}, \
         data={:?}, size={})",
        in_unit,
        in_id,
        in_scope,
        in_element,
        in_data,
        in_data_size
    );
    let mut update_al_distance = None;

    // Ограничиваем область видимости заимствования
    {
        let Some(host_object) = audio_components::State::get(&mut env.framework_state)
            .audio_component_instances
            .get_mut(&in_unit)
        else {
            return paramErr;
        };

        match in_id {
            kAudioUnitProperty_3DMixerDistanceParams => {
                let params = env
                    .mem
                    .read::<audio_components::MixerDistanceParams, false>(in_data.cast());
                let bus = host_object.mixer_buses.entry(in_element).or_default();
                bus.distance_params = params;

                // Сохраняем значения для OpenAL, чтобы применить их после
                // завершения borrow
                if let Some(source) = bus.al_source {
                    update_al_distance = Some((source, params));
                }
            }
            kAudioUnitProperty_MatrixLevels => {
                log_dbg!(
                    "Заглушка для kAudioUnitProperty_MatrixLevels \
                     на шине {}",
                    in_element
                );
            }
            kAudioUnitProperty_SpatializationAlgorithm
            | kAudioUnitProperty_3DMixerRenderingFlags => {
                log_dbg!(
                    "AudioUnitSetProperty: флаги \
                     spatialization/rendering проигнорированы"
                );
            }
            kAudioUnitProperty_SetRenderCallback => {
                let render_callback = env
                    .mem
                    .read::<AURenderCallbackStruct, false>(in_data.cast());
                if in_scope == kAudioUnitScope_Input {
                    let bus = host_object.mixer_buses.entry(in_element).or_default();
                    bus.render_callback = Some(render_callback);
                } else {
                    host_object.render_callback = Some(render_callback);
                }
                let proc_copy = render_callback.input_proc;
                let ref_con_copy = render_callback.input_proc_ref_con;
                log_dbg!(
                    "AudioUnitSetProperty(SetRenderCallback) \
                     unit={:?} scope={} element={} proc={:?} ref_con={:?}",
                    in_unit,
                    in_scope,
                    in_element,
                    proc_copy,
                    ref_con_copy
                );
            }
            kAudioOutputUnitProperty_SetInputCallback => {
                let cb = env
                    .mem
                    .read::<AURenderCallbackStruct, false>(in_data.cast());
                host_object.render_callback = Some(cb);
                let proc_copy = cb.input_proc;
                let ref_con_copy = cb.input_proc_ref_con;
                log_dbg!(
                    "AudioUnitSetProperty(SetInputCallback) \
                     unit={:?} scope={} element={} proc={:?} ref_con={:?}",
                    in_unit,
                    in_scope,
                    in_element,
                    proc_copy,
                    ref_con_copy
                );
            }
            kAudioUnitProperty_StreamFormat => {
                let stream_format = env
                    .mem
                    .read::<AudioStreamBasicDescription, false>(in_data.cast());
                log_if_broken_audio_format(&stream_format);
                let (sf_id, sf_sr, sf_ch, sf_bc, sf_bpf, sf_flags) = (
                    stream_format.format_id,
                    stream_format.sample_rate,
                    stream_format.channels_per_frame,
                    stream_format.bits_per_channel,
                    stream_format.bytes_per_frame,
                    stream_format.format_flags,
                );
                log_dbg!(
                    "AudioUnitSetProperty(StreamFormat) \
                     unit={:?} scope={} element={} \
                     format_id=0x{:x} sr={} ch={} bits={} bpf={} flags=0x{:x}",
                    in_unit,
                    in_scope,
                    in_element,
                    sf_id,
                    sf_sr,
                    sf_ch,
                    sf_bc,
                    sf_bpf,
                    sf_flags
                );
                match in_scope {
                    kAudioUnitScope_Global => {
                        host_object.global_stream_format = stream_format;
                    }
                    kAudioUnitScope_Output => {
                        host_object.output_stream_format = Some(stream_format);
                    }
                    kAudioUnitScope_Input => {
                        host_object.input_stream_format = Some(stream_format);
                        // Для 3D Mixer: формат шины N задаётся
                        // scope=Input, element=N.
                        let bus = host_object.mixer_buses.entry(in_element).or_default();
                        bus.stream_format = Some(stream_format);
                    }
                    _ => log_dbg!(
                        "AudioUnitSetProperty StreamFormat: \
                         неподдерживаемая область (scope) {}",
                        in_scope
                    ),
                }
            }
            kAudioUnitProperty_SampleRate => {
                let rate: f64 = env.mem.read::<f64, false>(in_data.cast());
                host_object.global_stream_format.sample_rate = rate;
            }
            kAudioUnitProperty_MaximumFramesPerSlice => {
                let frames: u32 = env.mem.read::<u32, false>(in_data.cast());
                host_object.maximum_frames_per_slice = frames;
            }
            kAudioUnitProperty_MakeConnection => {
                let conn = env.mem.read::<AudioUnitConnection, false>(in_data.cast());
                let src_unit = conn.source_audio_unit;
                let src_out = conn.source_output_number;
                let dst_in = conn.dest_input_number;
                log_dbg!(
                    "AudioUnitSetProperty(MakeConnection) \
                     dest_unit={:?} dest_input={} \
                     src_unit={:?} src_output={}",
                    in_unit,
                    dst_in,
                    src_unit,
                    src_out
                );
            }
            kAudioOutputUnitProperty_EnableIO => {
                // Ввод/Вывод включен по умолчанию. Игнорируем.
                let enabled: u32 = env.mem.read::<u32, false>(in_data.cast());
                log_dbg!(
                    "AudioUnitSetProperty(EnableIO) \
                     unit={:?} scope={} element={} enabled={}",
                    in_unit,
                    in_scope,
                    in_element,
                    enabled
                );
            }
            kAudioUnitProperty_ElementCount => {
                // Apple docs: kAudioUnitProperty_ElementCount (11)
                // Sets the number of input or output buses (elements) on a
                // multi-bus audio unit such as the 3D Mixer or Matrix Mixer.
                // scope=1 (Input) sets how many input buses exist;
                // scope=2 (Output) sets output bus count.
                // We pre-allocate the requested number of MixerBusState
                // entries so that subsequent per-bus Set/Get calls find
                // an existing entry rather than creating one on the fly.
                let count: u32 = env.mem.read::<u32, false>(in_data.cast());
                log_dbg!(
                    "AudioUnitSetProperty(ElementCount) \
                     unit={:?} scope={} element={} count={}",
                    in_unit,
                    in_scope,
                    in_element,
                    count
                );
                // Only the Input scope bus-count is meaningful for the
                // 3D Mixer / MultiChannelMixer.  Pre-populate entries so
                // that subsequent per-bus property calls find an existing slot.
                if in_scope == kAudioUnitScope_Input {
                    for bus_idx in 0..count {
                        host_object.mixer_buses.entry(bus_idx).or_default();
                    }
                }
                // Output / Global element counts are accepted silently —
                // there is nothing extra to initialise on our side.
            }
            _ => {
                log!(
                    "AudioUnitSetProperty: UNHANDLED property {} \
                     (unit={:?}, scope={}, element={}, size={})",
                    in_id,
                    in_unit,
                    in_scope,
                    in_element,
                    in_data_size
                );
            }
        }
    } // Конец заимствования host_object и env.framework_state

    // Теперь безопасно вызываем OpenAL
    if let Some((source, params)) = update_al_distance {
        let context = env
            .framework_state
            .audio_toolbox
            .make_al_context_current(&mut env.openal_manager);
        unsafe {
            context.Sourcef(source, AL_REFERENCE_DISTANCE, params.reference_distance);
            context.Sourcef(source, AL_MAX_DISTANCE, params.maximum_distance);
            context.Sourcef(source, AL_ROLLOFF_FACTOR, params.rolloff_factor);
        }
    }

    0
}

// =========================================================================
// MARK: - Получение свойств AudioUnit
// =========================================================================

/// Вспомогательная функция: безопасная запись значения в гостевую память.
/// Если указатель нулевой — запись пропускается (API допускает NULL).
fn write_if_nonnull<T: crate::mem::SafeWrite>(env: &mut Environment, ptr: MutPtr<T>, value: T) {
    if !ptr.is_null() {
        env.mem.write(ptr, value);
    }
}

fn AudioUnitGetProperty(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_id: AudioUnitPropertyID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    out_data: MutVoidPtr,
    io_data_size: MutPtr<u32>,
) -> OSStatus {
    log_dbg!(
        "AudioUnitGetProperty(unit={:?}, prop={}, scope={}, element={})",
        in_unit,
        in_id,
        in_scope,
        in_element
    );
    let Some(host_object) = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&in_unit)
    else {
        return paramErr;
    };

    match in_id {
        kAudioUnitProperty_MaximumFramesPerSlice => {
            let v = host_object.maximum_frames_per_slice;
            write_if_nonnull(env, out_data.cast(), v);
            write_if_nonnull(env, io_data_size, guest_size_of::<u32>());
        }
        kAudioUnitProperty_StreamFormat => {
            // Для scope=Input сначала смотрим per-bus формат
            // (element=N соответствует шине N у MultiChannelMixer/3DMixer),
            // затем input_stream_format, затем global_stream_format.
            // Для любых других scope — аналогично, но без bus-lookup.
            let fmt = match in_scope {
                kAudioUnitScope_Input => host_object
                    .mixer_buses
                    .get(&in_element)
                    .and_then(|b| b.stream_format)
                    .or(host_object.input_stream_format)
                    .unwrap_or(host_object.global_stream_format),
                kAudioUnitScope_Output => host_object
                    .output_stream_format
                    .unwrap_or(host_object.global_stream_format),
                _ => host_object.global_stream_format,
            };
            // out_data может быть NULL — в таком случае игра просто
            // запрашивает размер (см. документацию AudioUnitGetProperty).
            write_if_nonnull(env, out_data.cast(), fmt);
            write_if_nonnull(
                env,
                io_data_size,
                guest_size_of::<AudioStreamBasicDescription>(),
            );
        }
        kAudioUnitProperty_SampleRate => {
            let rate = host_object.global_stream_format.sample_rate;
            write_if_nonnull(env, out_data.cast(), rate);
            write_if_nonnull(env, io_data_size, guest_size_of::<f64>());
        }
        kAudioUnitProperty_ElementCount => {
            // Возвращаем количество шин микшера или 1 как дефолт.
            let count = if !host_object.mixer_buses.is_empty() {
                host_object.mixer_buses.len() as u32
            } else {
                1u32
            };
            write_if_nonnull(env, out_data.cast(), count);
            write_if_nonnull(env, io_data_size, guest_size_of::<u32>());
        }
        kAudioOutputUnitProperty_IsRunning => {
            let running: u32 = if host_object.started { 1 } else { 0 };
            write_if_nonnull(env, out_data.cast(), running);
            write_if_nonnull(env, io_data_size, guest_size_of::<u32>());
        }
        kAudioUnitProperty_Latency => {
            // Возвращаем нулевую задержку как заглушку.
            write_if_nonnull(env, out_data.cast(), 0.0f64);
            write_if_nonnull(env, io_data_size, guest_size_of::<f64>());
        }
        kAudioUnitProperty_LastRenderError => {
            write_if_nonnull(env, out_data.cast(), 0u32);
            write_if_nonnull(env, io_data_size, guest_size_of::<u32>());
        }
        kAudioUnitProperty_ShouldAllocateBuffer
        | kAudioUnitProperty_InPlaceProcessing
        | kAudioUnitProperty_BypassEffect => {
            // Булевые свойства — возвращаем 1 (да/включено) как заглушку.
            write_if_nonnull(env, out_data.cast(), 1u32);
            write_if_nonnull(env, io_data_size, guest_size_of::<u32>());
        }
        kAudioOutputUnitProperty_HasIO => {
            // IO активен по умолчанию.
            write_if_nonnull(env, out_data.cast(), 1u32);
            write_if_nonnull(env, io_data_size, guest_size_of::<u32>());
        }
        _ => {
            log!(
                "AudioUnitGetProperty: UNHANDLED property {} \
                 (unit={:?}, scope={}, element={})",
                in_id,
                in_unit,
                in_scope,
                in_element
            );
            // Записываем размер 0, чтобы гость не читал мусор.
            write_if_nonnull(env, io_data_size, 0u32);
            return -1;
        }
    }
    0
}

fn AudioUnitGetPropertyInfo(
    env: &mut Environment,
    _in_unit: AudioUnit,
    in_id: AudioUnitPropertyID,
    _in_scope: AudioUnitScope,
    _in_element: AudioUnitElement,
    out_data_size: MutPtr<u32>,
    out_writable: MutPtr<bool>,
) -> OSStatus {
    let (size, writable) = match in_id {
        kAudioUnitProperty_StreamFormat => (guest_size_of::<AudioStreamBasicDescription>(), true),
        kAudioUnitProperty_SampleRate => (guest_size_of::<f64>(), true),
        kAudioUnitProperty_MaximumFramesPerSlice => (guest_size_of::<u32>(), true),
        kAudioUnitProperty_ElementCount => (guest_size_of::<u32>(), true),
        kAudioOutputUnitProperty_IsRunning => (guest_size_of::<u32>(), false),
        kAudioOutputUnitProperty_HasIO => (guest_size_of::<u32>(), true),
        kAudioUnitProperty_Latency => (guest_size_of::<f64>(), false),
        kAudioUnitProperty_LastRenderError => (guest_size_of::<u32>(), false),
        kAudioUnitProperty_ShouldAllocateBuffer
        | kAudioUnitProperty_InPlaceProcessing
        | kAudioUnitProperty_BypassEffect => (guest_size_of::<u32>(), true),
        _ => {
            log_dbg!("AudioUnitGetPropertyInfo: unknown property {}", in_id);
            return -1;
        }
    };

    if !out_data_size.is_null() {
        env.mem.write(out_data_size, size);
    }
    if !out_writable.is_null() {
        env.mem.write(out_writable, writable);
    }
    0
}

// =========================================================================
// MARK: - Получение/установка параметров (Parameters)
// =========================================================================

fn AudioUnitSetParameter(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_id: AudioUnitParameterID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    in_value: AudioUnitParameterValue,
    _in_offset: u32,
) -> OSStatus {
    log_dbg!(
        "AudioUnitSetParameter(unit={:?}, param={}, scope={}, \
         element={}, value={})",
        in_unit,
        in_id,
        in_scope,
        in_element,
        in_value
    );
    let mut update_al_pos = None;

    // Ограничиваем область видимости заимствования
    {
        let Some(host_object) = audio_components::State::get(&mut env.framework_state)
            .audio_component_instances
            .get_mut(&in_unit)
        else {
            return paramErr;
        };

        match in_id {
            k3DMixerParam_Azimuth | k3DMixerParam_Elevation | k3DMixerParam_Distance => {
                let bus = host_object.mixer_buses.entry(in_element).or_default();
                if in_id == k3DMixerParam_Azimuth {
                    let radians = in_value.to_radians();
                    bus.position[0] = radians.sin();
                    bus.position[2] = -radians.cos();
                } else if in_id == k3DMixerParam_Elevation {
                    let radians = in_value.to_radians();
                    bus.position[1] = radians.sin();
                }

                // Сохраняем значения для OpenAL
                if let Some(source) = bus.al_source {
                    update_al_pos = Some((source, bus.position));
                }
            }
            _ => {}
        }
    } // Конец заимствования

    // Теперь безопасно вызываем OpenAL
    if let Some((source, pos)) = update_al_pos {
        let context = env
            .framework_state
            .audio_toolbox
            .make_al_context_current(&mut env.openal_manager);
        unsafe {
            context.Source3f(source, AL_POSITION, pos[0], pos[1], pos[2]);
        }
    }

    0
}

fn AudioUnitGetParameter(
    env: &mut Environment,
    in_unit: AudioUnit,
    in_id: AudioUnitParameterID,
    in_scope: AudioUnitScope,
    in_element: AudioUnitElement,
    out_value: MutPtr<AudioUnitParameterValue>,
) -> OSStatus {
    log_dbg!(
        "AudioUnitGetParameter(unit={:?}, param={}, scope={}, element={})",
        in_unit,
        in_id,
        in_scope,
        in_element
    );
    if !out_value.is_null() {
        env.mem.write(out_value, 1.0);
    }
    0
}

fn AudioUnitScheduleParameters(
    _e: &mut Environment,
    _u: AudioUnit,
    _p: ConstVoidPtr,
    _n: u32,
) -> OSStatus {
    0
}

fn AudioUnitReset(
    env: &mut Environment,
    in_unit: AudioUnit,
    _s: AudioUnitScope,
    _e: AudioUnitElement,
) -> OSStatus {
    if let Some(obj) = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get_mut(&in_unit)
    {
        obj.last_render_time = None;
    }
    0
}

// =========================================================================
// MARK: - Запуск / Остановка AudioOutputUnit
// =========================================================================

fn AudioOutputUnitStart(env: &mut Environment, ci: AudioUnit) -> OSStatus {
    let has_callback = audio_components::State::get(&mut env.framework_state)
        .audio_component_instances
        .get(&ci)
        .map(|o| o.render_callback.is_some())
        .unwrap_or(false);
    log_dbg!(
        "AudioOutputUnitStart({:?}) render_callback_set={}",
        ci,
        has_callback
    );
    setup_audio_unit_for_render(env, ci);
    0
}

/// Подготовить AudioUnit к работе в run-loop'e: завести OpenAL-источник для
/// прямого render-callback'а (если он есть) и/или для каждой input-шины
/// 3D-Mixer'а (если callback'и заданы через
/// `AUGraphSetNodeInputCallback`).
/// Используется как из `AudioOutputUnitStart`, так и из `AUGraphStart`.
pub fn setup_audio_unit_for_render(env: &mut Environment, ci: AudioUnit) {
    // Сначала собираем номера шин, у которых есть callback, но ещё нет
    // OpenAL-источника, чтобы обойтись без двойного `&mut`.
    let bus_ids_needing_source: Vec<u32> = {
        let state = audio_components::State::get(&mut env.framework_state);
        let Some(obj) = state.audio_component_instances.get(&ci) else {
            return;
        };
        obj.mixer_buses
            .iter()
            .filter_map(|(id, bus)| {
                if bus.render_callback.is_some() && bus.al_source.is_none() {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect()
    };

    let need_unit_source = {
        let state = audio_components::State::get(&mut env.framework_state);
        let Some(obj) = state.audio_component_instances.get(&ci) else {
            return;
        };
        obj.al_source.is_none()
    };

    let context = env
        .framework_state
        .audio_toolbox
        .al_context
        .make_al_context_current(&mut env.openal_manager);

    let unit_source: Option<ALuint> = if need_unit_source {
        let mut s: ALuint = 0;
        unsafe {
            context.GenSources(1, &mut s);
            context.SourcePlay(s);
        }
        Some(s)
    } else {
        None
    };

    let mut bus_sources: Vec<(u32, ALuint)> = Vec::with_capacity(bus_ids_needing_source.len());
    for bus_id in &bus_ids_needing_source {
        let mut s: ALuint = 0;
        unsafe {
            context.GenSources(1, &mut s);
            context.SourcePlay(s);
        }
        bus_sources.push((*bus_id, s));
    }
    drop(context);

    let now = Instant::now();
    let state = audio_components::State::get(&mut env.framework_state);
    let Some(obj) = state.audio_component_instances.get_mut(&ci) else {
        return;
    };
    if let Some(s) = unit_source {
        obj.al_source = Some(s);
    }
    for (bus_id, src) in bus_sources {
        if let Some(bus) = obj.mixer_buses.get_mut(&bus_id) {
            bus.al_source = Some(src);
            if bus.last_render_time.is_none() {
                bus.last_render_time = Some(now);
            }
        }
    }
    if obj.last_render_time.is_none() {
        obj.last_render_time = Some(now);
    }
    obj.started = true;
}

fn AudioOutputUnitStop(env: &mut Environment, ci: AudioUnit) -> OSStatus {
    let at_state = &mut env.framework_state.audio_toolbox;
    let context = at_state
        .al_context
        .make_al_context_current(&mut env.openal_manager);

    if let Some(audio_unit_state) = at_state
        .audio_components
        .audio_component_instances
        .get_mut(&ci)
    {
        audio_unit_state.started = false;
        if let Some(al_source) = audio_unit_state.al_source {
            unsafe {
                context.DeleteSources(1, &al_source);
            }
        }
        audio_unit_state.al_source = None;
        0
    } else {
        -1
    }
}

// =========================================================================
// MARK: - Рендеринг (Render)
// =========================================================================

fn AudioUnitAddRenderNotify(
    _e: &mut Environment,
    u: AudioUnit,
    p: ConstVoidPtr,
    r: ConstVoidPtr,
) -> OSStatus {
    log_dbg!(
        "AudioUnitAddRenderNotify(unit={:?}, proc={:?}, ref_con={:?})",
        u,
        p,
        r
    );
    0
}
fn AudioUnitRemoveRenderNotify(
    _e: &mut Environment,
    u: AudioUnit,
    p: ConstVoidPtr,
    r: ConstVoidPtr,
) -> OSStatus {
    log_dbg!(
        "AudioUnitRemoveRenderNotify(unit={:?}, proc={:?}, ref_con={:?})",
        u,
        p,
        r
    );
    0
}

fn AudioUnitRender(
    env: &mut Environment,
    in_unit: AudioUnit,
    _f: MutPtr<u32>,
    _t: ConstVoidPtr,
    _b: u32,
    _n: u32,
    _d: MutVoidPtr,
) -> OSStatus {
    render_audio_unit(env, in_unit);
    0
}

fn AudioUnitProcess(
    env: &mut Environment,
    in_unit: AudioUnit,
    _f: MutPtr<u32>,
    _t: ConstVoidPtr,
    _n: u32,
    _d: MutVoidPtr,
) -> OSStatus {
    render_audio_unit(env, in_unit);
    0
}

fn AudioUnitProcessMultiple(
    _e: &mut Environment,
    _u: AudioUnit,
    _f: MutPtr<u32>,
    _t: ConstVoidPtr,
    _n: u32,
    _in_b: u32,
    _in_bl: ConstVoidPtr,
    _out_bl: MutVoidPtr,
) -> OSStatus {
    0
}
fn AudioUnitComplexRender(
    _e: &mut Environment,
    _u: AudioUnit,
    _f: MutPtr<u32>,
    _t: ConstVoidPtr,
    _b: u32,
    _n: u32,
    _p: MutPtr<u32>,
    _pd: MutVoidPtr,
    _d: MutVoidPtr,
) -> OSStatus {
    0
}

/// Per-bus рендеринг для 3D Mixer / любого юнита, в котором через
/// `AUGraphSetNodeInputCallback` (или эквивалент) задан input render
/// callback на отдельные шины. Для каждой такой шины вызывает гостевой
/// callback, получает PCM и складывает его в свой OpenAL-источник.
/// OpenAL Soft сам микширует все источники вместе.
fn render_audio_unit_buses(env: &mut Environment, audio_unit: AudioUnit) {
    use crate::frameworks::core_audio_types::{
        kAudioFormatFlagIsPacked, kAudioFormatFlagIsSignedInteger, kAudioFormatLinearPCM,
    };

    // Готовим план: список
    // (bus_id, callback, al_source, last_render_time, format).
    let plan: Vec<(
        u32,
        AURenderCallbackStruct,
        ALuint,
        Instant,
        AudioStreamBasicDescription,
    )> = {
        let at = &mut env.framework_state.audio_toolbox;
        let hardware_sr = at.audio_session.current_hardware_sample_rate;
        let Some(obj) = at
            .audio_components
            .audio_component_instances
            .get(&audio_unit)
        else {
            return;
        };
        if obj.mixer_buses.is_empty() {
            return;
        }
        // Дефолтный формат шины 3D Mixer, если игра его явно не задавала:
        // 16-bit signed integer LE PCM, моно, текущая частота железа.
        let default_format = AudioStreamBasicDescription {
            sample_rate: if hardware_sr > 0.0 {
                hardware_sr
            } else {
                22050.0
            },
            format_id: kAudioFormatLinearPCM,
            format_flags: kAudioFormatFlagIsSignedInteger | kAudioFormatFlagIsPacked,
            bytes_per_packet: 2,
            frames_per_packet: 1,
            bytes_per_frame: 2,
            channels_per_frame: 1,
            bits_per_channel: 16,
            _reserved: 0,
        };
        let mut v = Vec::new();
        for (bus_id, bus) in obj.mixer_buses.iter() {
            let (Some(cb), Some(src), Some(last)) =
                (bus.render_callback, bus.al_source, bus.last_render_time)
            else {
                continue;
            };
            let fmt = bus.stream_format.unwrap_or(default_format);
            v.push((*bus_id, cb, src, last, fmt));
        }
        v
    };
    if plan.is_empty() {
        return;
    }

    log_once!(
        "render_audio_unit_buses: rendering bus callback(s) \
         on first iteration"
    );

    let now = Instant::now();
    for (bus_id, callback, al_source, last_render_time, fmt) in plan {
        // Ограничиваем глубину очереди OpenAL, чтобы буферы не накапливались
        // быстрее, чем воспроизводятся. Если этого не делать, при длительной
        // игре источник набирает всё больше необработанных буферов, звук
        // отстаёт по времени и начинает «скрипеть». Поведение зеркалит
        // `handle_audio_queue` в audio_queue.rs.
        let mut queued = 0;
        let mut processed = 0;
        {
            let context = env
                .framework_state
                .audio_toolbox
                .al_context
                .make_al_context_current(&mut env.openal_manager);
            unsafe {
                context.GetSourcei(al_source, AL_BUFFERS_QUEUED, &mut queued);
                context.GetSourcei(al_source, AL_BUFFERS_PROCESSED, &mut processed);
            }
        }
        if queued.saturating_sub(processed) > 1 {
            // Источник ещё не успел проиграть то, что уже в очереди.
            // Сливаем отыгранные буферы и пропускаем рендер на этот тик.
            let mut drained: Vec<ALuint> = Vec::new();
            {
                let context = env
                    .framework_state
                    .audio_toolbox
                    .al_context
                    .make_al_context_current(&mut env.openal_manager);
                unsafe {
                    while processed > 0 {
                        let mut b = 0;
                        context.SourceUnqueueBuffers(al_source, 1, &mut b);
                        drained.push(b);
                        processed -= 1;
                    }
                    if !drained.is_empty() {
                        context.DeleteBuffers(drained.len() as i32, drained.as_ptr());
                    }
                }
            }
            if let Some(obj) = audio_components::State::get(&mut env.framework_state)
                .audio_component_instances
                .get_mut(&audio_unit)
            {
                if let Some(bus) = obj.mixer_buses.get_mut(&bus_id) {
                    bus.last_render_time = Some(now);
                }
            }
            continue;
        }

        let elapsed = now.duration_since(last_render_time);
        let frames = ((elapsed.as_secs_f64() * fmt.sample_rate) as u32).clamp(64, 4096);
        let buffer_size = frames * fmt.channels_per_frame * (fmt.bits_per_channel / 8);
        if buffer_size == 0 {
            continue;
        }

        // Дренируем уже отыгранные буферы.
        let mut free_buffers: Vec<ALuint> = Vec::new();
        {
            let context = env
                .framework_state
                .audio_toolbox
                .al_context
                .make_al_context_current(&mut env.openal_manager);
            unsafe {
                let mut processed = 0;
                context.GetSourcei(al_source, AL_BUFFERS_PROCESSED, &mut processed);
                while processed > 0 {
                    let mut b = 0;
                    context.SourceUnqueueBuffers(al_source, 1, &mut b);
                    free_buffers.push(b);
                    context.GetSourcei(al_source, AL_BUFFERS_PROCESSED, &mut processed);
                }
            }
        }

        // Готовим AudioBufferList<1> и вызываем гостевой callback.
        let action_flags = env.mem.alloc_and_write(0u32);
        let buffer_data = env.mem.alloc(buffer_size);
        let abl = env.mem.alloc_and_write(AudioBufferList::<1> {
            number_buffers: 1,
            buffers: [AudioBuffer {
                number_channels: fmt.channels_per_frame,
                data_byte_size: buffer_size,
                data: buffer_data,
            }],
        });

        let input_proc = callback.input_proc;
        let input_proc_ref = callback.input_proc_ref_con;

        let _: OSStatus = input_proc.call_from_host(
            env,
            (
                input_proc_ref,
                action_flags,
                nil.cast_void().cast_const(),
                bus_id,
                frames,
                abl.cast::<std::ffi::c_void>(),
            ),
        );

        let (al_fmt, _, processed) = decode_buffer(&env.mem, &fmt, buffer_data.cast(), buffer_size);

        if !processed.is_empty() {
            let context = env
                .framework_state
                .audio_toolbox
                .al_context
                .make_al_context_current(&mut env.openal_manager);
            unsafe {
                let b = free_buffers.pop().unwrap_or_else(|| {
                    let mut x = 0;
                    context.GenBuffers(1, &mut x);
                    x
                });
                context.BufferData(
                    b,
                    al_fmt,
                    processed.as_ptr() as *const ALvoid,
                    processed.len() as i32,
                    fmt.sample_rate as i32,
                );
                context.SourceQueueBuffers(al_source, 1, &b);
                let mut state = 0;
                context.GetSourcei(al_source, AL_SOURCE_STATE, &mut state);
                if state != AL_PLAYING {
                    context.SourcePlay(al_source);
                }
                if !free_buffers.is_empty() {
                    context.DeleteBuffers(free_buffers.len() as i32, free_buffers.as_ptr());
                }
            }
        } else {
            // Если callback ничего не записал — освобождаем оставшиеся
            // буферы, чтобы они не утекли.
            if !free_buffers.is_empty() {
                let context = env
                    .framework_state
                    .audio_toolbox
                    .al_context
                    .make_al_context_current(&mut env.openal_manager);
                unsafe {
                    context.DeleteBuffers(free_buffers.len() as i32, free_buffers.as_ptr());
                }
            }
        }

        env.mem.free(action_flags.cast_void());
        env.mem.free(buffer_data.cast_void());
        env.mem.free(abl.cast_void().cast());

        // Обновляем last_render_time для шины.
        if let Some(obj) = audio_components::State::get(&mut env.framework_state)
            .audio_component_instances
            .get_mut(&audio_unit)
        {
            if let Some(bus) = obj.mixer_buses.get_mut(&bus_id) {
                bus.last_render_time = Some(now);
            }
        }
    }
}

pub fn render_audio_unit(env: &mut Environment, audio_unit: AudioUnit) {
    if env.bundle.bundle_identifier().starts_with("com.ea.simcity") {
        // Применяем хак специфичный для SimCity: пропускаем рендеринг
        return;
    }

    // Прокачиваем все input-шины (3D Mixer / AUGraph): каждой шине свой
    // OpenAL-источник.
    render_audio_unit_buses(env, audio_unit);

    let (
        sample_rate,
        started,
        is_running,
        stream_format,
        has_input_format,
        al_source,
        last_render_time,
        callback,
    ) = {
        let at = &mut env.framework_state.audio_toolbox;
        let Some(obj) = at
            .audio_components
            .audio_component_instances
            .get_mut(&audio_unit)
        else {
            log_once!("render_audio_unit: instance not found");
            return;
        };
        (
            obj.input_stream_format
                .map(|f| f.sample_rate)
                .unwrap_or(at.audio_session.current_hardware_sample_rate),
            obj.started,
            obj.is_running_handler,
            obj.input_stream_format
                .unwrap_or(obj.output_stream_format.unwrap_or(obj.global_stream_format)),
            obj.input_stream_format.is_some(),
            obj.al_source,
            obj.last_render_time,
            obj.render_callback,
        )
    };

    if !started {
        log_once!("render_audio_unit: skipped (started=false)");
        return;
    }
    if is_running {
        log_once!("render_audio_unit: skipped (already running handler)");
        return;
    }

    if let Some(obj) = env
        .framework_state
        .audio_toolbox
        .audio_components
        .audio_component_instances
        .get_mut(&audio_unit)
    {
        obj.is_running_handler = true;
    }

    let Some(al_source) = al_source else {
        log_once!("render_audio_unit: skipped (al_source = None)");
        if let Some(obj) = env
            .framework_state
            .audio_toolbox
            .audio_components
            .audio_component_instances
            .get_mut(&audio_unit)
        {
            obj.is_running_handler = false;
        }
        return;
    };
    let Some(_last_render_time) = last_render_time else {
        log_once!("render_audio_unit: skipped (last_render_time = None)");
        if let Some(obj) = env
            .framework_state
            .audio_toolbox
            .audio_components
            .audio_component_instances
            .get_mut(&audio_unit)
        {
            obj.is_running_handler = false;
        }
        return;
    };
    let Some(callback) = callback else {
        // Без unit-level callback'а просто молча выходим: бус-рендер уже
        // сделан, а 3D Mixer / RemoteIO без своего собственного callback'а
        // — это нормальный кейс при работе через AUGraph.
        if let Some(obj) = env
            .framework_state
            .audio_toolbox
            .audio_components
            .audio_component_instances
            .get_mut(&audio_unit)
        {
            obj.is_running_handler = false;
        }
        return;
    };
    log_once!("render_audio_unit: entering callback for the first time");

    let now = Instant::now();
    let mut queued_buffers = 0;
    let mut processed_buffers = 0;
    {
        let context = env
            .framework_state
            .audio_toolbox
            .al_context
            .make_al_context_current(&mut env.openal_manager);
        unsafe {
            context.GetSourcei(al_source, AL_BUFFERS_QUEUED, &mut queued_buffers);
            context.GetSourcei(al_source, AL_BUFFERS_PROCESSED, &mut processed_buffers);
        }
    }

    let remaining_buffers = queued_buffers.saturating_sub(processed_buffers);
    if remaining_buffers > 1 {
        let mut drained_buffers = Vec::new();
        {
            let context = env
                .framework_state
                .audio_toolbox
                .al_context
                .make_al_context_current(&mut env.openal_manager);
            unsafe {
                while processed_buffers > 0 {
                    let mut b = 0;
                    context.SourceUnqueueBuffers(al_source, 1, &mut b);
                    drained_buffers.push(b);
                    processed_buffers -= 1;
                }
                if !drained_buffers.is_empty() {
                    context.DeleteBuffers(drained_buffers.len() as i32, drained_buffers.as_ptr());
                }
            }
        }

        if let Some(obj) = env
            .framework_state
            .audio_toolbox
            .audio_components
            .audio_component_instances
            .get_mut(&audio_unit)
        {
            obj.last_render_time = Some(now);
            obj.is_running_handler = false;
        }
        return;
    }

    let mut al_buffers = Vec::new();
    {
        let context = env
            .framework_state
            .audio_toolbox
            .al_context
            .make_al_context_current(&mut env.openal_manager);
        unsafe {
            while processed_buffers > 0 {
                let mut b = 0;
                context.SourceUnqueueBuffers(al_source, 1, &mut b);
                al_buffers.push(b);
                processed_buffers -= 1;
            }
        }
    }

    let target_frames = (sample_rate
        * env
            .framework_state
            .audio_toolbox
            .audio_session
            .current_hardware_io_buffer_duration as f64)
        .round() as u32;
    let frames = target_frames.clamp(64, 2048);
    let buffer_size =
        frames * stream_format.channels_per_frame * (stream_format.bits_per_channel / 8);

    let action_flags = env.mem.alloc_and_write(0u32);

    // Восстанавливаем логику из оригинала: Resident Evil 4 ожидает 2 буфера
    let (audio_buffer_list, buffer1_data, buffer2_data): (
        MutVoidPtr,
        MutVoidPtr,
        Option<MutVoidPtr>,
    ) = if has_input_format {
        let buf = env.mem.alloc(buffer_size);
        let abl = env.mem.alloc_and_write(AudioBufferList::<1> {
            number_buffers: 1,
            buffers: [AudioBuffer {
                number_channels: stream_format.channels_per_frame,
                data_byte_size: buffer_size,
                data: buf,
            }],
        });
        (abl.cast(), buf, None)
    } else {
        let buf1 = env.mem.alloc(buffer_size);
        let buf2 = env.mem.alloc(buffer_size);
        let abl = env.mem.alloc_and_write(AudioBufferList::<2> {
            number_buffers: 2,
            buffers: [
                AudioBuffer {
                    number_channels: stream_format.channels_per_frame,
                    data_byte_size: buffer_size,
                    data: buf1,
                },
                AudioBuffer {
                    number_channels: stream_format.channels_per_frame,
                    data_byte_size: buffer_size,
                    data: buf2,
                },
            ],
        });
        (abl.cast(), buf1, Some(buf2))
    };

    let input_proc = callback.input_proc;
    let input_proc_ref = callback.input_proc_ref_con;

    let _: OSStatus = input_proc.call_from_host(
        env,
        (
            input_proc_ref,
            action_flags,
            nil.cast_void().cast_const(),
            0u32,
            frames,
            audio_buffer_list,
        ),
    );

    let (al_fmt, _, processed) =
        decode_buffer(&env.mem, &stream_format, buffer1_data.cast(), buffer_size);
    {
        let context = env
            .framework_state
            .audio_toolbox
            .al_context
            .make_al_context_current(&mut env.openal_manager);
        unsafe {
            let b = al_buffers.pop().unwrap_or_else(|| {
                let mut x = 0;
                context.GenBuffers(1, &mut x);
                x
            });
            context.BufferData(
                b,
                al_fmt,
                processed.as_ptr() as *const ALvoid,
                processed.len() as i32,
                sample_rate as i32,
            );
            context.SourceQueueBuffers(al_source, 1, &b);
            let mut state = 0;
            context.GetSourcei(al_source, AL_SOURCE_STATE, &mut state);
            if state != AL_PLAYING {
                context.SourcePlay(al_source);
            }
            if !al_buffers.is_empty() {
                context.DeleteBuffers(al_buffers.len() as i32, al_buffers.as_ptr());
            }
        }
    }

    env.mem.free(action_flags.cast_void());
    env.mem.free(buffer1_data.cast_void());
    if let Some(b2) = buffer2_data {
        env.mem.free(b2.cast_void());
    }
    env.mem.free(audio_buffer_list.cast_void());

    if let Some(obj) = env
        .framework_state
        .audio_toolbox
        .audio_components
        .audio_component_instances
        .get_mut(&audio_unit)
    {
        obj.last_render_time = Some(now);
        obj.is_running_handler = false;
    }
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioUnitInitialize(_)),
    export_c_func!(AudioUnitUninitialize(_)),
    export_c_func!(AudioUnitSetProperty(_, _, _, _, _, _)),
    export_c_func!(AudioUnitGetProperty(_, _, _, _, _, _)),
    export_c_func!(AudioUnitGetPropertyInfo(_, _, _, _, _, _)),
    export_c_func!(AudioUnitSetParameter(_, _, _, _, _, _)),
    export_c_func!(AudioUnitGetParameter(_, _, _, _, _)),
    export_c_func!(AudioUnitScheduleParameters(_, _, _)),
    export_c_func!(AudioUnitReset(_, _, _)),
    export_c_func!(AudioOutputUnitStart(_)),
    export_c_func!(AudioOutputUnitStop(_)),
    export_c_func!(AudioUnitAddRenderNotify(_, _, _)),
    export_c_func!(AudioUnitRemoveRenderNotify(_, _, _)),
    export_c_func!(AudioUnitRender(_, _, _, _, _, _)),
    export_c_func!(AudioUnitProcess(_, _, _, _, _)),
    export_c_func!(AudioUnitProcessMultiple(_, _, _, _, _, _, _)),
];
