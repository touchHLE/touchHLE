/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0.
 * If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

//! `AudioConverter.h` — Audio format conversion.
//!
//! Provides a passthrough implementation for LPCM -> LPCM conversion,
//! which is used by the Symphony Engine in PvZ.

use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::FunctionExports;
use crate::export_c_func;
use crate::frameworks::carbon_core::OSStatus;
use crate::frameworks::core_audio_types::{debug_fourcc, AudioStreamBasicDescription};
use crate::mem::{ConstPtr, MutPtr, MutVoidPtr, SafeRead};
use crate::Environment;

const kAudioConverterErr_InvalidInputSize: OSStatus = -50;

#[repr(C, packed)]
pub struct AudioBuffer {
    pub mNumberChannels: u32,
    pub mDataByteSize: u32,
    pub mData: MutVoidPtr,
}
unsafe impl SafeRead for AudioBuffer {}

#[repr(C, packed)]
pub struct AudioBufferList {
    pub mNumberBuffers: u32,
    pub mBuffers: [AudioBuffer; 1],
}
unsafe impl SafeRead for AudioBufferList {}

#[repr(C, packed)]
pub struct AudioStreamPacketDescription {
    pub mStartOffset: i64,
    pub mVariableFramesInPacket: u32,
    pub mDataByteSize: u32,
}
unsafe impl SafeRead for AudioStreamPacketDescription {}

pub type AudioConverterComplexInputDataProc = GuestFunction;

#[repr(C, packed)]
struct OpaqueAudioConverter {
    source_format: AudioStreamBasicDescription,
    dest_format: AudioStreamBasicDescription,
}
unsafe impl SafeRead for OpaqueAudioConverter {}

type AudioConverterRef = MutPtr<OpaqueAudioConverter>;

fn AudioConverterNew(
    env: &mut Environment,
    in_source_format: ConstPtr<AudioStreamBasicDescription>,
    in_destination_format: ConstPtr<AudioStreamBasicDescription>,
    out_audio_converter: MutPtr<AudioConverterRef>,
) -> OSStatus {
    let source_format = env.mem.read(in_source_format);
    let dest_format = env.mem.read(in_destination_format);

    log_dbg!(
        "AudioConverterNew: {} -> {}",
        debug_fourcc(source_format.format_id),
        debug_fourcc(dest_format.format_id)
    );

    let converter_data = OpaqueAudioConverter {
        source_format,
        dest_format,
    };

    let converter: AudioConverterRef = env.mem.alloc_and_write(converter_data);
    env.mem.write(out_audio_converter, converter);

    0 // noErr
}

fn AudioConverterDispose(env: &mut Environment, in_audio_converter: AudioConverterRef) -> OSStatus {
    if in_audio_converter.is_null() {
        return kAudioConverterErr_InvalidInputSize;
    }
    env.mem.free(in_audio_converter.cast());
    0
}

fn AudioConverterReset(_env: &mut Environment, _in_audio_converter: AudioConverterRef) -> OSStatus {
    0
}

fn AudioConverterGetProperty(
    _env: &mut Environment,
    in_audio_converter: AudioConverterRef,
    in_property_id: u32,
    _io_data_size: MutPtr<u32>,
    _out_property_data: MutPtr<u8>,
) -> OSStatus {
    log_dbg!(
        "AudioConverterGetProperty({:?}, {}) -> stubbed",
        in_audio_converter,
        debug_fourcc(in_property_id),
    );
    -1 // Unimplemented
}

fn AudioConverterSetProperty(
    _env: &mut Environment,
    in_audio_converter: AudioConverterRef,
    in_property_id: u32,
    in_data_size: u32,
    _in_property_data: ConstPtr<u8>,
) -> OSStatus {
    log_dbg!(
        "AudioConverterSetProperty({:?}, {}, size={}) -> stubbed",
        in_audio_converter,
        debug_fourcc(in_property_id),
        in_data_size,
    );
    0
}

/// Complex buffer conversion (Passthrough for LPCM -> LPCM)
fn AudioConverterFillComplexBuffer(
    env: &mut Environment,
    in_audio_converter: AudioConverterRef,
    in_input_data_proc: AudioConverterComplexInputDataProc,
    in_input_data_proc_user_data: MutVoidPtr,
    io_output_data_packet_size: MutPtr<u32>,
    out_output_data: MutPtr<AudioBufferList>,
    out_packet_description: MutPtr<MutPtr<AudioStreamPacketDescription>>,
) -> OSStatus {
    if in_audio_converter.is_null() {
        return kAudioConverterErr_InvalidInputSize;
    }

    // Трюк с пробросом (Passthrough hack):
    // Поскольку у нас LPCM -> LPCM, нам не нужно конвертировать данные.
    // Мы просто передаем выходные буферы (out_output_data) напрямую в игровой
    // коллбэк,
    // и игра сама запишет звук сразу в нужный буфер!

    let callback_status: OSStatus = in_input_data_proc.call_from_host(
        env,
        (
            in_audio_converter,
            io_output_data_packet_size,
            out_output_data,
            out_packet_description,
            in_input_data_proc_user_data,
        ),
    );

    // Возвращаем статус коллбэка. Размеры пакетов уже обновлены самой игрой по
    // указателям.
    callback_status
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioConverterNew(_, _, _)),
    export_c_func!(AudioConverterDispose(_)),
    export_c_func!(AudioConverterReset(_)),
    export_c_func!(AudioConverterGetProperty(_, _, _, _)),
    export_c_func!(AudioConverterSetProperty(_, _, _, _)),
    // У функции 6 аргументов помимо env, поэтому 6 подчеркиваний
    export_c_func!(AudioConverterFillComplexBuffer(_, _, _, _, _, _)),
];
