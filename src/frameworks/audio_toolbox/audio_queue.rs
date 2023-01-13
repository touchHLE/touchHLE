//! `AudioQueue.h` (Audio Queue Services)

use crate::abi::GuestFunction;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::core_audio_types::{
    debug_fourcc, kAudioFormatAppleIMA4, kAudioFormatLinearPCM, AudioStreamBasicDescription,
};
use crate::frameworks::core_foundation::cf_run_loop::{
    kCFRunLoopCommonModes, CFRunLoopMode, CFRunLoopRef,
};
use crate::frameworks::foundation::ns_run_loop;
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::mac_types::OSStatus;
use crate::mem::{ConstPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr, SafeRead};
use crate::objc::msg;
use crate::Environment;
use std::collections::{HashMap, VecDeque};

#[derive(Default)]
pub struct State {
    audio_queues: HashMap<AudioQueueRef, AudioQueueHostObject>,
}
impl State {
    pub fn get(framework_state: &mut crate::frameworks::State) -> &mut Self {
        &mut framework_state.audio_toolbox.audio_queue
    }
}

struct AudioQueueHostObject {
    _format: AudioStreamBasicDescription,
    _callback_proc: AudioQueueOutputCallback,
    _callback_user_data: MutVoidPtr,
    /// Weak reference
    _run_loop: CFRunLoopRef,
    volume: f32,
    buffers: Vec<AudioQueueBufferRef>,
    buffer_queue: VecDeque<AudioQueueBufferRef>,
}

#[repr(C, packed)]
pub struct OpaqueAudioQueue {
    _filler: u8,
}
unsafe impl SafeRead for OpaqueAudioQueue {}

pub type AudioQueueRef = MutPtr<OpaqueAudioQueue>;

#[repr(C, packed)]
struct AudioQueueBuffer {
    audio_data_bytes_capacity: u32,
    audio_data: MutVoidPtr,
    audio_data_byte_size: u32,
    user_data: MutVoidPtr,
    _packet_description_capacity: u32,
    /// Should be a `MutPtr<AudioStreamPacketDescription>`, but that's not
    /// implemented yet.
    _packet_descriptions: MutVoidPtr,
    _packet_description_count: u32,
}
unsafe impl SafeRead for AudioQueueBuffer {}

type AudioQueueBufferRef = MutPtr<AudioQueueBuffer>;

/// (*void)(void *in_user_data, AudioQueueRef in_aq, AudioQueueBufferRef in_buf)
type AudioQueueOutputCallback = GuestFunction;

type AudioQueueParameterID = u32;
const kAudioQueueParam_Volume: AudioQueueParameterID = 1;

type AudioQueueParameterValue = f32;

fn AudioQueueNewOutput(
    env: &mut Environment,
    in_format: ConstPtr<AudioStreamBasicDescription>,
    in_callback_proc: AudioQueueOutputCallback,
    in_user_data: MutVoidPtr,
    in_callback_run_loop: CFRunLoopRef,
    in_callback_run_loop_mode: CFRunLoopMode,
    in_flags: u32,
    out_aq: MutPtr<AudioQueueRef>,
) -> OSStatus {
    // reserved
    assert!(in_flags == 0);
    // NULL not implemented
    assert!(!in_callback_run_loop.is_null());
    // NULL is a synonym of kCFRunLoopCommonModes here
    assert!(
        in_callback_run_loop_mode.is_null() || {
            let common_modes = get_static_str(env, kCFRunLoopCommonModes);
            msg![env; in_callback_run_loop_mode isEqualTo:common_modes]
        }
    );

    let format = env.mem.read(in_format);

    let format_id = format.format_id;
    // others not implemented yet
    assert!(format_id == kAudioFormatLinearPCM || format_id == kAudioFormatAppleIMA4);

    let host_object = AudioQueueHostObject {
        _format: format,
        _callback_proc: in_callback_proc,
        _callback_user_data: in_user_data,
        _run_loop: in_callback_run_loop,
        volume: 1.0,
        buffers: Vec::new(),
        buffer_queue: VecDeque::new(),
    };

    let aq_ref = env.mem.alloc_and_write(OpaqueAudioQueue { _filler: 0 });
    State::get(&mut env.framework_state)
        .audio_queues
        .insert(aq_ref, host_object);
    env.mem.write(out_aq, aq_ref);

    ns_run_loop::add_audio_queue(env, in_callback_run_loop, aq_ref);

    log_dbg!(
        "AudioQueueNewOutput() for format {}, new audio queue handle: {:?}",
        debug_fourcc(format_id),
        aq_ref,
    );

    0 // success
}

fn AudioQueueSetParameter(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_param_id: AudioQueueParameterID,
    in_value: AudioQueueParameterValue,
) -> OSStatus {
    assert!(in_param_id == kAudioQueueParam_Volume); // others unimplemented

    let host_object = State::get(&mut env.framework_state)
        .audio_queues
        .get_mut(&in_aq)
        .unwrap();

    host_object.volume = in_value;

    0 // success
}

fn AudioQueueAllocateBuffer(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_buffer_byte_size: GuestUSize,
    out_buffer: MutPtr<AudioQueueBufferRef>,
) -> OSStatus {
    let host_object = State::get(&mut env.framework_state)
        .audio_queues
        .get_mut(&in_aq)
        .unwrap();

    let audio_data = env.mem.alloc(in_buffer_byte_size);
    let buffer_ptr = env.mem.alloc_and_write(AudioQueueBuffer {
        audio_data_bytes_capacity: in_buffer_byte_size,
        audio_data,
        audio_data_byte_size: 0,
        user_data: Ptr::null(),
        _packet_description_capacity: 0,
        _packet_descriptions: Ptr::null(),
        _packet_description_count: 0,
    });
    host_object.buffers.push(buffer_ptr);
    env.mem.write(out_buffer, buffer_ptr);

    0 // success
}

fn AudioQueueEnqueueBuffer(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_buffer: AudioQueueBufferRef,
    in_num_packet_descs: u32,
    in_packet_descs: MutVoidPtr,
) -> OSStatus {
    // variable packet size unimplemented
    assert!(in_num_packet_descs == 0 && in_packet_descs.is_null());

    let host_object = State::get(&mut env.framework_state)
        .audio_queues
        .get_mut(&in_aq)
        .unwrap();

    // TODO: Return error if buffer doesn't belong to audio queue
    assert!(host_object.buffers.contains(&in_buffer));

    host_object.buffer_queue.push_back(in_buffer);

    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioQueueNewOutput(_, _, _, _, _, _, _)),
    export_c_func!(AudioQueueSetParameter(_, _, _)),
    export_c_func!(AudioQueueAllocateBuffer(_, _, _)),
    export_c_func!(AudioQueueEnqueueBuffer(_, _, _, _)),
];
