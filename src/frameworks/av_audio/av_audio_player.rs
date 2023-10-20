/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! AVAudioPlayer
//!
//! Implemented using Audio Queue Services based on [the PlayingAudio example](https://developer.apple.com/library/archive/documentation/MusicAudio/Conceptual/AudioQueueProgrammingGuide/AQPlayback/PlayingAudio.html)

use crate::dyld::FunctionExports;
use crate::frameworks::audio_toolbox::audio_file::{
    kAudioFilePropertyDataFormat, kAudioFilePropertyPacketSizeUpperBound, kAudioFileReadPermission,
    AudioFileClose, AudioFileGetProperty, AudioFileID, AudioFileOpenURL, AudioFileReadPackets,
};
use crate::frameworks::audio_toolbox::audio_queue::{
    kAudioQueueParam_Volume, AudioQueueAllocateBuffer, AudioQueueBufferRef, AudioQueueDispose,
    AudioQueueEnqueueBuffer, AudioQueueNewOutput, AudioQueueOutputCallback, AudioQueuePause,
    AudioQueueRef, AudioQueueSetParameter, AudioQueueStart, AudioQueueStop,
};
use crate::frameworks::carbon_core::eofErr;
use crate::frameworks::core_audio_types::AudioStreamBasicDescription;
use crate::frameworks::core_foundation::cf_run_loop::kCFRunLoopCommonModes;
use crate::frameworks::foundation::ns_string;
use crate::mem::{guest_size_of, GuestUSize, MutPtr, MutVoidPtr, Ptr};
use crate::msg;
use crate::objc::{id, nil, retain, Class, ClassExports, HostObject, NSZonePtr};
use crate::objc_classes;
use crate::{export_c_func, Environment};

const kNumberBuffers: usize = 3;

struct AVAudioPlayerHostObject {
    audio_file_url: id,
    output_callback: AudioQueueOutputCallback,
    audio_file_id: Option<AudioFileID>,
    audio_desc: Option<AudioStreamBasicDescription>,
    audio_queue: Option<AudioQueueRef>,
    audio_queue_buffers: Option<MutPtr<AudioQueueBufferRef>>,
    num_packets_to_read: u32,
    current_packet: i64,
    is_playing: bool,
}
impl HostObject for AVAudioPlayerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation AVAudioPlayer: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let symb = "__av_audio_player_handle_output_buffer";
    let callback = env
        .dyld
        .create_private_proc_address(&mut env.mem, &mut env.cpu, symb)
        .unwrap_or_else(|_| panic!("create_private_proc_address failed {}", symb));

    let host_object = Box::new(AVAudioPlayerHostObject {
        audio_file_url: nil,
        output_callback: callback,
        audio_file_id: None,
        audio_desc: None,
        audio_queue: None,
        audio_queue_buffers: None,
        num_packets_to_read: 0,
        current_packet: 0,
        is_playing: false
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithContentsOfURL:(id)url error:(id)error {
    assert!(error.is_null());
    let path: id = msg![env; url path];
    let path_str = ns_string::to_rust_string(env, path);
    log_dbg!("initWithContentsOfURL: {}", path_str);

    retain(env, url);

    let host_object = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this);
    host_object.audio_file_url = url;
    this
}

- (())setDelegate:(id)delegate {
    log!("TODO: [(AVAudioPlayer*){:?} setDelegate:{:?}]", this, delegate);
}

- (())setVolume:(f32)volume {
    let aq_ref = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_queue.unwrap();
    let status = AudioQueueSetParameter(env, aq_ref, kAudioQueueParam_Volume, volume);
    assert_eq!(status, 0);
}

- (())prepareToPlay {
    let audio_file_id = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_file_id;
    if audio_file_id.is_some() {
        return;
    }

    let audio_file_url = env.objc.borrow::<AVAudioPlayerHostObject>(this).audio_file_url;
    let callback = env.objc.borrow::<AVAudioPlayerHostObject>(this).output_callback;

    let tmp_afi_ptr: MutPtr<AudioFileID> = env.mem.alloc(guest_size_of::<AudioFileID>()).cast();
    let status = AudioFileOpenURL(env, audio_file_url, kAudioFileReadPermission, 0, tmp_afi_ptr);
    assert_eq!(status, 0);
    let audio_file_id = env.mem.read(tmp_afi_ptr);
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_file_id = Some(audio_file_id);
    env.mem.free(tmp_afi_ptr.cast());

    let size = guest_size_of::<AudioStreamBasicDescription>();
    let tmp_size_ptr: MutPtr<GuestUSize> = env.mem.alloc(guest_size_of::<GuestUSize>()).cast();
    env.mem.write(tmp_size_ptr, size);
    let tmp_data_ptr: MutPtr<AudioStreamBasicDescription> = env.mem.alloc(size).cast();
    let status = AudioFileGetProperty(
        env, audio_file_id, kAudioFilePropertyDataFormat, tmp_size_ptr, tmp_data_ptr.cast()
    );
    assert_eq!(status, 0);
    assert_eq!(size, env.mem.read(tmp_size_ptr));
    let audio_desc = env.mem.read(tmp_data_ptr);
    log_dbg!("audio_desc {:?}", audio_desc);
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_desc = Some(audio_desc);

    let aq_ref_ptr: MutPtr<AudioQueueRef> = env.mem.alloc(guest_size_of::<AudioQueueRef>()).cast();
    let common_modes = ns_string::get_static_str(env, kCFRunLoopCommonModes);
    let status = AudioQueueNewOutput(
        env, tmp_data_ptr.cast_const(), callback, this.cast(),
        Ptr::null(), common_modes, 0, aq_ref_ptr
    );
    assert_eq!(status, 0);
    let aq_ref = env.mem.read(aq_ref_ptr);
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_queue = Some(aq_ref);

    let size = guest_size_of::<u32>();
    env.mem.write(tmp_size_ptr, size);
    let prop_size_ptr: MutPtr<u32> = env.mem.alloc(size).cast();
    let status = AudioFileGetProperty(
        env, audio_file_id, kAudioFilePropertyPacketSizeUpperBound, tmp_size_ptr, prop_size_ptr.cast()
    );
    assert_eq!(status, 0);
    assert_eq!(size, env.mem.read(tmp_size_ptr));
    let prop_size = env.mem.read(prop_size_ptr);

    let (buffer_byte_size, num_packets_to_read) = derive_buffer_size(audio_desc, prop_size, 0.5);
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).num_packets_to_read = num_packets_to_read;

    let buffers: MutPtr<AudioQueueBufferRef> = env.mem.alloc(kNumberBuffers as GuestUSize * guest_size_of::<AudioQueueBufferRef>()).cast();
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_queue_buffers = Some(buffers);

    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).is_playing = true;
    for i in 0..kNumberBuffers {
        let status = AudioQueueAllocateBuffer(env, aq_ref, buffer_byte_size, buffers + i as u32);
        assert_eq!(status, 0);

        _av_audio_player_handle_output_buffer(env, this.cast(), aq_ref, env.mem.read(buffers + i as u32));
    }
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).is_playing = false;

    env.mem.free(tmp_size_ptr.cast());
    env.mem.free(aq_ref_ptr.cast());
    env.mem.free(tmp_data_ptr.cast());
}

- (bool)isPlaying {
    env.objc.borrow::<AVAudioPlayerHostObject>(this).is_playing
}

- (bool)play {
    () = msg![env; this prepareToPlay];

    let aq_ref = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_queue.unwrap();

    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).is_playing = true;

    let status = AudioQueueStart(env, aq_ref, Ptr::null());
    assert_eq!(status, 0);

    true
}

- (())pause {
    env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).is_playing = false;
    if let Some(aq_ref) = env.objc.borrow::<AVAudioPlayerHostObject>(this).audio_queue {
         AudioQueuePause(env, aq_ref);
    }
}

- (())stop {
    let &mut AVAudioPlayerHostObject {
        audio_file_id,
        audio_queue,
        audio_queue_buffers,
        ..
    } = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this);
    if audio_queue.is_none() {
        // already being stopped
        return;
    }
    AudioQueueDispose(env, audio_queue.unwrap(), true);
    AudioFileClose(env, audio_file_id.unwrap());
    env.mem.free(audio_queue_buffers.unwrap().cast());

    let callback = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).output_callback;
    *env.objc.borrow_mut::<AVAudioPlayerHostObject>(this) = AVAudioPlayerHostObject {
        audio_file_url: nil,
        output_callback: callback,
        audio_file_id: None,
        audio_desc: None,
        audio_queue: None,
        audio_queue_buffers: None,
        num_packets_to_read: 0,
        current_packet: 0,
        is_playing: false
    };
}

@end

};

// Listing 3-7 from `Deriving a playback audio queue buffer size`
// from the Apple's guide
fn derive_buffer_size(
    audio_desc: AudioStreamBasicDescription,
    max_packet_size: u32,
    seconds: f64,
) -> (u32, u32) {
    let mut out_buffer_size;

    const max_buffer_size: u32 = 0x50000;
    const min_buffer_size: u32 = 0x4000;

    if audio_desc.frames_per_packet != 0 {
        let num_packets_to_time =
            audio_desc.sample_rate / audio_desc.frames_per_packet as f64 * seconds;
        out_buffer_size = num_packets_to_time as u32 * max_packet_size;
    } else {
        out_buffer_size = if max_buffer_size > max_packet_size {
            max_buffer_size
        } else {
            max_packet_size
        }
    }

    if out_buffer_size > max_buffer_size && out_buffer_size > max_packet_size {
        out_buffer_size = max_buffer_size
    } else if out_buffer_size < min_buffer_size {
        out_buffer_size = min_buffer_size
    }

    let out_num_packets_to_read = out_buffer_size / max_packet_size;
    (out_buffer_size, out_num_packets_to_read)
}

/// (*void)(void *in_user_data, AudioQueueRef in_aq, AudioQueueBufferRef in_buf)
fn _av_audio_player_handle_output_buffer(
    env: &mut Environment,
    in_user_data: MutVoidPtr,
    in_aq: AudioQueueRef,
    in_buf: AudioQueueBufferRef,
) {
    let av_audio_player: id = in_user_data.cast();
    let class: Class = msg![env; av_audio_player class];
    log_dbg!(
        "_av_audio_player_handle_output_buffer on object of class: {}",
        env.objc.get_class_name(class)
    );
    assert_eq!(
        class,
        env.objc.get_known_class("AVAudioPlayer", &mut env.mem)
    );

    let &AVAudioPlayerHostObject {
        audio_file_id,
        audio_queue,
        num_packets_to_read,
        current_packet,
        is_playing,
        ..
    } = env.objc.borrow(av_audio_player);
    let aq = audio_queue.unwrap();
    assert_eq!(aq, in_aq);

    if !is_playing {
        return;
    }

    let num_bytes_ptr: MutPtr<u32> = env.mem.alloc(guest_size_of::<u32>()).cast();
    let num_packets_ptr: MutPtr<u32> = env.mem.alloc(guest_size_of::<u32>()).cast();
    env.mem.write(num_packets_ptr, num_packets_to_read);
    let mut audio_queue_buffer = env.mem.read(in_buf);
    let status = AudioFileReadPackets(
        env,
        audio_file_id.unwrap(),
        false,
        num_bytes_ptr,
        Ptr::null(),
        current_packet,
        num_packets_ptr,
        audio_queue_buffer.audio_data,
    );
    if status == eofErr {
        // TODO: respect number of loops
        return;
    } else {
        assert_eq!(status, 0);
    }
    let num_packets = env.mem.read(num_packets_ptr);
    if num_packets > 0 {
        audio_queue_buffer.audio_data_byte_size = env.mem.read(num_bytes_ptr);
        env.mem.write(in_buf, audio_queue_buffer);
        let status = AudioQueueEnqueueBuffer(env, aq, in_buf, 0, Ptr::null());
        assert_eq!(status, 0);
        env.objc
            .borrow_mut::<AVAudioPlayerHostObject>(av_audio_player)
            .current_packet = current_packet + num_packets as i64;
    } else {
        let status = AudioQueueStop(env, aq, false);
        assert_eq!(status, 0);
        env.objc
            .borrow_mut::<AVAudioPlayerHostObject>(av_audio_player)
            .is_playing = false;
    }

    env.mem.free(num_packets_ptr.cast());
    env.mem.free(num_bytes_ptr.cast());
}

pub const PRIVATE_FUNCTIONS: FunctionExports = &[export_c_func!(
    _av_audio_player_handle_output_buffer(_, _, _)
)];
