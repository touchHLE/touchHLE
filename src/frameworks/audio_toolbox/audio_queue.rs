/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AudioQueue.h` (Audio Queue Services)
//!
//! The audio playback here is mapped onto OpenAL Soft for convenience.
//! Apple's implementation probably uses Core Audio instead.

use crate::abi::{CallFromHost, GuestFunction};
use crate::audio::decode_ima4;
use crate::audio::openal as al;
use crate::audio::openal::al_types::*;
use crate::audio::openal::alc_types::*;
use crate::dyld::{export_c_func, FunctionExports};
use crate::frameworks::carbon_core::OSStatus;
use crate::frameworks::core_audio_types::{
    debug_fourcc, fourcc, kAudioFormatAppleIMA4, kAudioFormatFlagIsBigEndian,
    kAudioFormatFlagIsFloat, kAudioFormatFlagIsPacked, kAudioFormatLinearPCM,
    AudioStreamBasicDescription,
};
use crate::frameworks::core_foundation::cf_run_loop::{
    kCFRunLoopCommonModes, CFRunLoopGetMain, CFRunLoopMode, CFRunLoopRef,
};
use crate::frameworks::foundation::ns_run_loop;
use crate::frameworks::foundation::ns_string::get_static_str;
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, Mem, MutPtr, MutVoidPtr, Ptr, SafeRead};
use crate::objc::msg;
use crate::Environment;
use std::collections::{HashMap, VecDeque};

#[derive(Default)]
pub struct State {
    audio_queues: HashMap<AudioQueueRef, AudioQueueHostObject>,
    al_device_and_context: Option<(*mut ALCdevice, *mut ALCcontext)>,
}
impl State {
    fn get(framework_state: &mut crate::frameworks::State) -> &mut Self {
        &mut framework_state.audio_toolbox.audio_queue
    }
    fn make_al_context_current(&mut self) -> ContextManager {
        if self.al_device_and_context.is_none() {
            let device = unsafe { al::alcOpenDevice(std::ptr::null()) };
            assert!(!device.is_null());
            let context = unsafe { al::alcCreateContext(device, std::ptr::null()) };
            assert!(!context.is_null());
            log_dbg!(
                "New internal OpenAL device ({:?}) and context ({:?})",
                device,
                context
            );
            self.al_device_and_context = Some((device, context));
        }
        let (device, context) = self.al_device_and_context.unwrap();
        assert!(!device.is_null() && !context.is_null());

        // This object will make sure the existing context, which will belong
        // to the guest app, is restored once we're done.
        ContextManager::make_active(context)
    }
}

#[must_use]
struct ContextManager(*mut ALCcontext);
impl ContextManager {
    pub fn make_active(new_context: *mut ALCcontext) -> ContextManager {
        let old_context = unsafe { al::alcGetCurrentContext() };
        assert!(unsafe { al::alcMakeContextCurrent(new_context) } == al::ALC_TRUE);
        ContextManager(old_context)
    }
}
impl Drop for ContextManager {
    fn drop(&mut self) {
        assert!(unsafe { al::alcMakeContextCurrent(self.0) } == al::ALC_TRUE)
    }
}

struct AudioQueueHostObject {
    format: AudioStreamBasicDescription,
    callback_proc: AudioQueueOutputCallback,
    callback_user_data: MutVoidPtr,
    /// Weak reference
    run_loop: CFRunLoopRef,
    volume: f32,
    buffers: Vec<AudioQueueBufferRef>,
    /// There is also a queue of OpenAL buffers, which must be kept in sync:
    /// the nth item in this queue must also be the nth item in the OpenAL
    /// queue, though the OpenAL queue may be shorter.
    buffer_queue: VecDeque<AudioQueueBufferRef>,
    /// Tracks whether this audio queue has been started, so we can restart the
    /// OpenAL source if it automatically stops due to running out of data.
    is_running: bool,
    al_source: Option<ALuint>,
    al_unused_buffers: Vec<ALuint>,
    aq_is_running_proc: Option<AudioQueuePropertyListenerProc>,
    aq_is_running_user_data: Option<MutVoidPtr>,
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

pub type AudioQueuePropertyID = u32;
pub const kAudioQueueProperty_IsRunning: AudioQueuePropertyID = fourcc(b"aqrn");

/// (*void)(void *in_user_data, AudioQueueRef in_aq, AudioQueuePropertyID in_id)
type AudioQueuePropertyListenerProc = GuestFunction;

const kAudioQueueErr_InvalidBuffer: OSStatus = -66687;

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
    // NULL is a synonym of kCFRunLoopCommonModes here
    assert!(
        in_callback_run_loop_mode.is_null() || {
            let common_modes = get_static_str(env, kCFRunLoopCommonModes);
            msg![env; in_callback_run_loop_mode isEqualTo:common_modes]
        }
    );

    let in_callback_run_loop = if in_callback_run_loop.is_null() {
        // FIXME: According to the documentation, "one of the audio queue's
        // internal threads" should be used if you don't specify a run loop.
        // We should have an "internal thread" instead of using the main thread.
        CFRunLoopGetMain(env)
    } else {
        in_callback_run_loop
    };

    let format = env.mem.read(in_format);

    let host_object = AudioQueueHostObject {
        format,
        callback_proc: in_callback_proc,
        callback_user_data: in_user_data,
        run_loop: in_callback_run_loop,
        volume: 1.0,
        buffers: Vec::new(),
        buffer_queue: VecDeque::new(),
        is_running: false,
        al_source: None,
        al_unused_buffers: Vec::new(),
        aq_is_running_proc: None,
        aq_is_running_user_data: None,
    };

    let aq_ref = env.mem.alloc_and_write(OpaqueAudioQueue { _filler: 0 });
    State::get(&mut env.framework_state)
        .audio_queues
        .insert(aq_ref, host_object);
    env.mem.write(out_aq, aq_ref);

    ns_run_loop::add_audio_queue(env, in_callback_run_loop, aq_ref);

    if !is_supported_audio_format(&format) {
        log_dbg!("Warning: Audio queue {:?} will be ignored because its format is not yet supported: {:#?}", aq_ref, format);
    }

    log_dbg!(
        "AudioQueueNewOutput() for format {:#?}, new audio queue handle: {:?}",
        format,
        aq_ref,
    );

    0 // success
}

fn AudioQueueGetParameter(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_param_id: AudioQueueParameterID,
    out_value: MutPtr<AudioQueueParameterValue>,
) -> OSStatus {
    return_if_null!(in_aq);

    assert!(in_param_id == kAudioQueueParam_Volume); // others unimplemented

    let state = State::get(&mut env.framework_state);
    let host_object = state.audio_queues.get_mut(&in_aq).unwrap();

    env.mem.write(out_value, host_object.volume);

    0 // success
}

fn AudioQueueSetParameter(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_param_id: AudioQueueParameterID,
    in_value: AudioQueueParameterValue,
) -> OSStatus {
    return_if_null!(in_aq);

    assert!(in_param_id == kAudioQueueParam_Volume); // others unimplemented

    let state = State::get(&mut env.framework_state);
    let host_object = state.audio_queues.get_mut(&in_aq).unwrap();

    host_object.volume = in_value;
    if let Some(al_source) = host_object.al_source {
        let _context_manager = state.make_al_context_current();
        unsafe {
            al::alSourcef(al_source, al::AL_MAX_GAIN, in_value);
            assert!(al::alGetError() == 0);
        }
    }

    0 // success
}

fn AudioQueueAllocateBuffer(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_buffer_byte_size: GuestUSize,
    out_buffer: MutPtr<AudioQueueBufferRef>,
) -> OSStatus {
    return_if_null!(in_aq);

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
    _in_num_packet_descs: u32,
    _in_packet_descs: MutVoidPtr,
) -> OSStatus {
    return_if_null!(in_aq);

    // Variable packet size unimplemented (no formats supported that need it).
    // We don't assert the count is 0 because we might get a useless one even
    // for formats that don't need it.

    let host_object = State::get(&mut env.framework_state)
        .audio_queues
        .get_mut(&in_aq)
        .unwrap();

    if !host_object.buffers.contains(&in_buffer) {
        return kAudioQueueErr_InvalidBuffer;
    }

    host_object.buffer_queue.push_back(in_buffer);
    log_dbg!("New buffer enqueued: {:?}", in_buffer);

    0 // success
}

fn AudioQueueAddPropertyListener(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_id: AudioQueuePropertyID,
    in_proc: AudioQueuePropertyListenerProc,
    in_user_data: MutVoidPtr,
) -> OSStatus {
    return_if_null!(in_aq);

    if in_id == kAudioQueueProperty_IsRunning {
        let host_object = State::get(&mut env.framework_state)
            .audio_queues
            .get_mut(&in_aq)
            .unwrap();

        host_object.aq_is_running_proc = Some(in_proc);
        host_object.aq_is_running_user_data = Some(in_user_data);
    } else {
        log!(
            "TODO: AudioQueueAddPropertyListener({:?}, {}, {:?}, {:?})",
            in_aq,
            debug_fourcc(in_id),
            in_proc,
            in_user_data
        );
    }
    0 // success
}
fn AudioQueueRemovePropertyListener(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_id: AudioQueuePropertyID,
    in_proc: AudioQueuePropertyListenerProc,
    in_user_data: MutVoidPtr,
) -> OSStatus {
    return_if_null!(in_aq);

    if in_id == kAudioQueueProperty_IsRunning {
        let host_object = State::get(&mut env.framework_state)
            .audio_queues
            .get_mut(&in_aq)
            .unwrap();

        host_object.aq_is_running_proc = None;
        host_object.aq_is_running_user_data = None;
    } else {
        log!(
            "TODO: AudioQueueRemovePropertyListener({:?}, {}, {:?}, {:?})",
            in_aq,
            debug_fourcc(in_id),
            in_proc,
            in_user_data
        );
    }
    0 // success
}

/// Check if the format of an audio queue is one we currently support.
/// If not, we should skip trying to play it rather than crash.
fn is_supported_audio_format(format: &AudioStreamBasicDescription) -> bool {
    let &AudioStreamBasicDescription {
        format_id,
        format_flags,
        channels_per_frame,
        bits_per_channel,
        ..
    } = format;
    match format_id {
        kAudioFormatAppleIMA4 => (channels_per_frame == 1) || (channels_per_frame == 2),
        kAudioFormatLinearPCM => {
            // TODO: support more PCM formats
            (channels_per_frame == 1 || channels_per_frame == 2)
                && (bits_per_channel == 8 || bits_per_channel == 16)
                && (format_flags & kAudioFormatFlagIsPacked) != 0
                && (format_flags & kAudioFormatFlagIsBigEndian) == 0
                && (format_flags & kAudioFormatFlagIsFloat) == 0
        }
        _ => false,
    }
}

/// Decode an [AudioQueueBuffer]'s content to raw PCM suitable for an OpenAL
/// buffer.
fn decode_buffer(
    mem: &Mem,
    format: &AudioStreamBasicDescription,
    buffer: &AudioQueueBuffer,
) -> (ALenum, ALsizei, Vec<u8>) {
    let data_slice = mem.bytes_at(buffer.audio_data.cast(), buffer.audio_data_byte_size);

    assert!(is_supported_audio_format(format));

    match format.format_id {
        kAudioFormatAppleIMA4 => {
            assert!(data_slice.len() % 34 == 0);
            let mut out_pcm = Vec::<u8>::with_capacity((data_slice.len() / 34) * 64 * 2);
            let packets = data_slice.chunks(34);

            if format.channels_per_frame == 1 {
                for packet in packets {
                    let pcm_packet: [i16; 64] = decode_ima4(packet.try_into().unwrap());
                    let pcm_bytes: &[u8] = unsafe {
                        std::slice::from_raw_parts(pcm_packet.as_ptr() as *const u8, 128)
                    };
                    out_pcm.extend_from_slice(pcm_bytes);
                }

                (al::AL_FORMAT_MONO16, format.sample_rate as ALsizei, out_pcm)
            } else {
                let packets = packets.collect::<Vec<_>>();
                assert_eq!(packets.len() % 2, 0);
                for i in 0..packets.len() / 2 {
                    let left_pcm_packet: [i16; 64] =
                        decode_ima4(packets[2 * i].try_into().unwrap());
                    let right_pcm_packet: [i16; 64] =
                        decode_ima4(packets[2 * i + 1].try_into().unwrap());
                    let t = left_pcm_packet
                        .iter()
                        .zip(right_pcm_packet.iter())
                        .flat_map(|(&a, &b)| vec![a, b])
                        .collect::<Vec<_>>();
                    let pcm_bytes: &[u8] =
                        unsafe { std::slice::from_raw_parts(t.as_ptr() as *const u8, 128 * 2) };
                    out_pcm.extend_from_slice(pcm_bytes);
                }

                (
                    al::AL_FORMAT_STEREO16,
                    format.sample_rate as ALsizei,
                    out_pcm,
                )
            }
        }
        kAudioFormatLinearPCM => {
            // The end of the data might be misaligned (this happens in Crash
            // Bandicoot Nitro Kart 3D somehow).
            let misaligned_by = data_slice.len() % (format.bytes_per_frame as usize);
            let data_slice = if misaligned_by != 0 {
                &data_slice[..data_slice.len() - misaligned_by]
            } else {
                data_slice
            };

            let f = match (format.channels_per_frame, format.bits_per_channel) {
                (1, 8) => al::AL_FORMAT_MONO8,
                (1, 16) => al::AL_FORMAT_MONO16,
                (2, 8) => al::AL_FORMAT_STEREO8,
                (2, 16) => al::AL_FORMAT_STEREO16,
                _ => unreachable!(),
            };
            (f, format.sample_rate as ALsizei, data_slice.to_owned())
        }
        _ => unreachable!(),
    }
}

/// Ensure an audio queue has an OpenAL source and at least one queued OpenAL
/// buffer.
fn prime_audio_queue(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    context_manager: Option<ContextManager>,
) -> ContextManager {
    let state = State::get(&mut env.framework_state);

    let context_manager = context_manager.unwrap_or_else(|| state.make_al_context_current());
    let host_object = state.audio_queues.get_mut(&in_aq).unwrap();

    if !is_supported_audio_format(&host_object.format) {
        return context_manager;
    }

    if host_object.al_source.is_none() {
        let mut al_source = 0;
        unsafe {
            al::alGenSources(1, &mut al_source);
            al::alSourcef(al_source, al::AL_MAX_GAIN, host_object.volume);
            assert!(al::alGetError() == 0);
        };
        host_object.al_source = Some(al_source);
    }
    let al_source = host_object.al_source.unwrap();

    loop {
        let mut al_buffers_queued = 0;
        let mut al_buffers_processed = 0;
        unsafe {
            al::alGetSourcei(al_source, al::AL_BUFFERS_QUEUED, &mut al_buffers_queued);
            al::alGetSourcei(
                al_source,
                al::AL_BUFFERS_PROCESSED,
                &mut al_buffers_processed,
            );
            assert!(al::alGetError() == 0);
        }
        let al_buffers_queued: usize = al_buffers_queued.try_into().unwrap();
        let al_buffers_processed: usize = al_buffers_processed.try_into().unwrap();

        assert!(al_buffers_queued <= host_object.buffer_queue.len());
        let unprocessed_buffers = al_buffers_queued - al_buffers_processed;

        if unprocessed_buffers > 1 || al_buffers_queued == host_object.buffer_queue.len() {
            break;
        }

        let next_buffer_idx = al_buffers_queued;
        let next_buffer_ref = host_object.buffer_queue[next_buffer_idx];
        let next_buffer = env.mem.read(next_buffer_ref);

        log_dbg!(
            "Decoding buffer {:?} for queue {:?}",
            next_buffer_ref,
            in_aq
        );

        let next_al_buffer = host_object.al_unused_buffers.pop().unwrap_or_else(|| {
            let mut al_buffer = 0;
            unsafe { al::alGenBuffers(1, &mut al_buffer) };
            assert!(unsafe { al::alGetError() } == 0);
            al_buffer
        });

        let (al_format, al_frequency, data) =
            decode_buffer(&env.mem, &host_object.format, &next_buffer);
        unsafe {
            al::alBufferData(
                next_al_buffer,
                al_format,
                data.as_ptr() as *const ALvoid,
                data.len().try_into().unwrap(),
                al_frequency,
            )
        };
        unsafe { al::alSourceQueueBuffers(al_source, 1, &next_al_buffer) };
        assert!(unsafe { al::alGetError() } == 0);
    }

    context_manager
}

fn unqueue_buffers<F: FnMut(ALuint)>(al_source: ALuint, mut callback: F) {
    loop {
        let mut al_buffers_processed = 0;
        unsafe {
            al::alGetSourcei(
                al_source,
                al::AL_BUFFERS_PROCESSED,
                &mut al_buffers_processed,
            );
            assert!(al::alGetError() == 0);
        }
        if al_buffers_processed == 0 {
            break;
        }

        let mut al_buffer = 0;
        unsafe {
            al::alSourceUnqueueBuffers(al_source, 1, &mut al_buffer);
            assert!(al::alGetError() == 0);
        }

        callback(al_buffer);
    }
}

/// For use by `NSRunLoop`: check the status of an audio queue, recycle buffers,
/// call callbacks, push new buffers etc.
pub fn handle_audio_queue(env: &mut Environment, in_aq: AudioQueueRef) {
    // Collect used buffers and call the user callback so the app can provide
    // new buffers.

    let state = State::get(&mut env.framework_state);

    let context_manager = state.make_al_context_current();

    let host_object = state.audio_queues.get_mut(&in_aq).unwrap();
    let Some(al_source) = host_object.al_source else {
        return;
    };
    if !is_supported_audio_format(&host_object.format) {
        return;
    }

    let mut buffers_to_reuse = Vec::new();

    unqueue_buffers(al_source, |al_buffer| {
        host_object.al_unused_buffers.push(al_buffer);
        let buffer_ref = host_object.buffer_queue.pop_front().unwrap();
        buffers_to_reuse.push(buffer_ref);
    });

    let &mut AudioQueueHostObject {
        callback_proc,
        callback_user_data,
        is_running,
        ..
    } = host_object;

    for buffer_ref in buffers_to_reuse.drain(..) {
        log_dbg!(
            "Recyling buffer {:?} for queue {:?}. Calling callback {:?} with user data {:?}.",
            buffer_ref,
            in_aq,
            callback_proc,
            callback_user_data
        );

        let () = callback_proc.call_from_host(env, (callback_user_data, in_aq, buffer_ref));
    }

    // Push new buffers etc.

    let _context_manager = prime_audio_queue(env, in_aq, Some(context_manager));

    if is_running {
        unsafe {
            let mut al_source_state = 0;
            al::alGetSourcei(al_source, al::AL_SOURCE_STATE, &mut al_source_state);
            assert!(al::alGetError() == 0);
            // Source probably ran out data and needs restarting
            // TODO: We currently have to do this even when touchHLE is not
            // lagging, because we're not ensuring OpenAL always has at least
            // one buffer it hasn't processed yet. We need to change our queue
            // handling.
            if al_source_state == al::AL_STOPPED {
                al::alSourcePlay(al_source);
                log_dbg!("Restarted OpenAL source for queue {:?}", in_aq);
            }
        }
    }
}

fn AudioQueuePrime(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    _in_number_of_frames_to_prepare: u32,
    out_number_of_frames_prepared: MutPtr<u32>,
) -> OSStatus {
    return_if_null!(in_aq);

    assert!(out_number_of_frames_prepared.is_null()); // TODO
    let _context_manager = prime_audio_queue(env, in_aq, None);
    0 // success
}

fn AudioQueueStart(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_device_start_time: ConstVoidPtr, // should be `const AudioTimeStamp*`
) -> OSStatus {
    return_if_null!(in_aq);

    assert!(in_device_start_time.is_null()); // TODO

    let _context_manager = prime_audio_queue(env, in_aq, None);

    let host_object = State::get(&mut env.framework_state)
        .audio_queues
        .get_mut(&in_aq)
        .unwrap();

    host_object.is_running = true;

    if is_supported_audio_format(&host_object.format) {
        let al_source = host_object.al_source.unwrap();
        unsafe { al::alSourcePlay(al_source) };
        assert!(unsafe { al::alGetError() } == 0);
    } else {
        log!(
            "AudioQueueStart: Unsupported format {}",
            debug_fourcc(host_object.format.format_id)
        );
    }

    if let (Some(in_proc), Some(in_user_data)) = (
        host_object.aq_is_running_proc,
        host_object.aq_is_running_user_data,
    ) {
        <GuestFunction as CallFromHost<(), (MutVoidPtr, Ptr<OpaqueAudioQueue, true>, u32)>>::
        call_from_host(
            &in_proc, env, (in_user_data, in_aq, kAudioQueueProperty_IsRunning)
        );
    }

    0 // success
}

fn AudioQueuePause(env: &mut Environment, in_aq: AudioQueueRef) -> OSStatus {
    return_if_null!(in_aq);

    let state = State::get(&mut env.framework_state);

    let _context_manager = state.make_al_context_current();

    let host_object = state.audio_queues.get_mut(&in_aq).unwrap();
    host_object.is_running = false;
    if let Some(al_source) = host_object.al_source {
        unsafe { al::alSourcePause(al_source) };
        assert!(unsafe { al::alGetError() } == 0);
    }

    0 // success
}

fn AudioQueueStop(env: &mut Environment, in_aq: AudioQueueRef, in_immediate: bool) -> OSStatus {
    return_if_null!(in_aq);

    let state = State::get(&mut env.framework_state);

    let _context_manager = state.make_al_context_current();

    let host_object = state.audio_queues.get_mut(&in_aq).unwrap();
    host_object.is_running = false;

    if in_immediate {
        if let Some(al_source) = host_object.al_source {
            unsafe { al::alSourceStop(al_source) };
            assert!(unsafe { al::alGetError() } == 0);
        }
    }

    0 // success
}

fn AudioQueueReset(env: &mut Environment, in_aq: AudioQueueRef) -> OSStatus {
    // TODO: flush any queued buffers
    // TODO: unify with AudioQueueStop
    AudioQueueStop(env, in_aq, true);
    AudioQueueStart(env, in_aq, ConstPtr::null())
}

fn AudioQueueFreeBuffer(
    env: &mut Environment,
    in_aq: AudioQueueRef,
    in_buffer: AudioQueueBufferRef,
) -> OSStatus {
    return_if_null!(in_aq);

    let host_object = State::get(&mut env.framework_state)
        .audio_queues
        .get_mut(&in_aq)
        .unwrap();

    if !host_object.buffers.contains(&in_buffer) {
        return kAudioQueueErr_InvalidBuffer;
    }

    // TODO: what about buffer_queue?
    let index = host_object
        .buffers
        .iter()
        .position(|x| x == &in_buffer)
        .unwrap();
    host_object.buffers.remove(index);

    log_dbg!("Freeing buffer: {:?}", in_buffer);

    let buffer = env.mem.read(in_buffer);
    env.mem.free(buffer.audio_data);
    env.mem.free(in_buffer.cast());

    0 // success
}

fn AudioQueueDispose(env: &mut Environment, in_aq: AudioQueueRef, in_immediate: bool) -> OSStatus {
    return_if_null!(in_aq);

    assert!(in_immediate); // TODO

    let state = State::get(&mut env.framework_state);

    let mut host_object = state.audio_queues.remove(&in_aq).unwrap();
    log_dbg!("Disposing of audio queue {:?}", in_aq);

    env.mem.free(in_aq.cast());

    for buffer_ptr in host_object.buffers {
        let buffer = env.mem.read(buffer_ptr);
        env.mem.free(buffer.audio_data);
        env.mem.free(buffer_ptr.cast());
    }

    if let Some(al_source) = host_object.al_source {
        let _context_manager = state.make_al_context_current();

        unsafe {
            al::alSourceStop(al_source);
            assert!(al::alGetError() == 0);
        }

        unqueue_buffers(al_source, |al_buffer| {
            host_object.al_unused_buffers.push(al_buffer)
        });

        unsafe {
            al::alDeleteBuffers(
                host_object.al_unused_buffers.len().try_into().unwrap(),
                host_object.al_unused_buffers.as_ptr(),
            );
            assert!(al::alGetError() == 0);
        }
    }

    ns_run_loop::remove_audio_queue(env, host_object.run_loop, in_aq);

    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioQueueNewOutput(_, _, _, _, _, _, _)),
    export_c_func!(AudioQueueGetParameter(_, _, _)),
    export_c_func!(AudioQueueSetParameter(_, _, _)),
    export_c_func!(AudioQueueAllocateBuffer(_, _, _)),
    export_c_func!(AudioQueueEnqueueBuffer(_, _, _, _)),
    export_c_func!(AudioQueueAddPropertyListener(_, _, _, _)),
    export_c_func!(AudioQueueRemovePropertyListener(_, _, _, _)),
    export_c_func!(AudioQueuePrime(_, _, _)),
    export_c_func!(AudioQueueStart(_, _)),
    export_c_func!(AudioQueuePause(_)),
    export_c_func!(AudioQueueStop(_, _)),
    export_c_func!(AudioQueueReset(_)),
    export_c_func!(AudioQueueFreeBuffer(_, _)),
    export_c_func!(AudioQueueDispose(_, _)),
];
