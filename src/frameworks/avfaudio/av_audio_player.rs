/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `AVAudioPlayer`

use touchHLE_openal_soft_wrapper::{
    alBufferData, alDeleteBuffers, alDeleteSources, alGenBuffers, alGenSources, alSourcePlay,
    alSourceStop, alSourcef, alSourcei,
    al_types::ALint,
    al_types::{ALuint, ALvoid},
    AL_BUFFER, AL_FORMAT_MONO16, AL_FORMAT_MONO8, AL_FORMAT_STEREO16, AL_FORMAT_STEREO8, AL_GAIN,
    AL_LOOPING,
};

use crate::{
    audio::{AudioDescription, AudioFile},
    environment::Environment,
    frameworks::foundation::{ns_url::to_rust_path, NSInteger, NSTimeInterval},
    msg,
    objc::{id, nil, release, retain, ClassExports, HostObject},
    objc_classes,
};
use std::{
    alloc::{alloc, dealloc, Layout},
    cmp::Ordering,
    mem::align_of,
};

#[derive(Default)]
struct AVAudioPlayerHostObject {
    url: id,
    out_error: id, // TODO: use this variable

    number_of_loops: NSInteger,
    volume: f32,

    audio_data: Option<(ALuint, ALuint, *mut u8, Layout)>, // buffer, source, data, data_layout
}
impl HostObject for AVAudioPlayerHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation AVAudioPlayer: NSObject

+ (id)alloc {
    let host_object = Box::new(AVAudioPlayerHostObject {
        url: nil,
        out_error: nil,
        number_of_loops: 0,
        volume: 1.0,
        audio_data: None
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithContentsOfURL:(id)url // NSURL *
                      error:(id)outError { // NSError * _Nullable *
    let host_object = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this);
    host_object.url = url;
    host_object.out_error = outError;
    retain(env, url);
    this
}

- (bool)prepareToPlay {
    // TODO: Determine the correct behavior
    if env.objc.borrow::<AVAudioPlayerHostObject>(this).audio_data.is_some() {
        // Return true if it's already set up
        return true;
    }

    let url = env.objc.borrow::<AVAudioPlayerHostObject>(this).url;
    let path = to_rust_path(env, url);
    let path_string = path.as_str().to_string();
    let Ok(mut audio_file) = AudioFile::open_for_reading(path, &env.fs) else {
        log!("Warning: couldn't open audio file {:?}", path_string);
        env.objc.borrow_mut::<AVAudioPlayerHostObject>(this).audio_data = None;
        return false;
    };

    let AudioDescription {
        sample_rate,
        channels_per_frame,
        bits_per_channel,
        ..
    } = audio_file.audio_description();
    let audio_format = {
        if channels_per_frame == 1 && bits_per_channel == 8 {
            AL_FORMAT_MONO8
        } else if channels_per_frame == 1 && bits_per_channel == 16 {
            AL_FORMAT_MONO16
        } else if channels_per_frame == 2 && bits_per_channel == 8 {
            AL_FORMAT_STEREO8
        } else if channels_per_frame == 2 && bits_per_channel == 16 {
            AL_FORMAT_STEREO16
        } else {
            // TODO: 0 bits_per_channel means the audio file is compressed,
            // audio_file.read_bytes must decompress it.
            log!("Warning: Attempted to prepare audio with a sample rate of {} Hz, {} channels and {} bits per channel", sample_rate, channels_per_frame, bits_per_channel);
            return false;
        }
    };

    let mut buffer: ALuint = 0;
    let mut source: ALuint = 0;
    let data_layout: Layout;
    let data: *mut u8;
    unsafe {
        let audio_file_bytes = audio_file.byte_count();
        data_layout = Layout::from_size_align(audio_file_bytes as usize, align_of::<u8>()).unwrap();
        data = alloc(data_layout);
        audio_file.read_bytes(0, std::slice::from_raw_parts_mut(data, audio_file_bytes as usize)).unwrap();

        // TODO: Error handling by reading alGetError
        alGenBuffers(1, &mut buffer);
        alBufferData(buffer, audio_format, data as *mut ALvoid, audio_file_bytes as i32, sample_rate as i32);
        alGenSources(1, &mut source);
        alSourcei(source, AL_BUFFER, buffer as ALint);
    }

    let host_object = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this);
    host_object.audio_data = Some((buffer, source, data, data_layout));

    // Re-apply properties in case they were set before the player was prepared
    let host_object = env.objc.borrow::<AVAudioPlayerHostObject>(this);
    let number_of_loops = host_object.number_of_loops;
    let volume = host_object.volume;
    () = msg![env; this setNumberOfLoops:number_of_loops];
    () = msg![env; this setVolume:volume];

    true
}

- (bool)play {
    if env.objc.borrow::<AVAudioPlayerHostObject>(this).audio_data.is_none() {
        let _: bool = msg![env; this prepareToPlay];
    }

    let host_object = env.objc.borrow::<AVAudioPlayerHostObject>(this);
    if let Some((_, source, _, _)) = host_object.audio_data {
        unsafe { alSourcePlay(source) }
        true
    } else {
        false
    }
}

- (bool)stop {
    let host_object = env.objc.borrow::<AVAudioPlayerHostObject>(this);
    if let Some((_, source, _, _)) = host_object.audio_data {
        unsafe { alSourceStop(source) }
    }

    // "Calling stop, or allowing a sound to finish playing, undoes this setup."
    // https://developer.apple.com/documentation/avfaudio/avaudioplayer/1386886-preparetoplay?language=objc
    cleanUpPreparation(env, this);

    true
}

- (())setNumberOfLoops:(NSInteger)numberOfLoops {
    let host_object = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this);
    host_object.number_of_loops = numberOfLoops;

    if let Some((_, source, _, _)) = host_object.audio_data {
        match numberOfLoops.cmp(&0) {
            Ordering::Equal => unsafe { alSourcei(source, AL_LOOPING, 0) },
            Ordering::Less => unsafe { alSourcei(source, AL_LOOPING, 1) },
            Ordering::Greater => unimplemented!()
        }
    }
}

- (())setVolume:(f32)volume {
    let host_object = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this);
    host_object.volume = volume;

    if let Some((_, source, _, _)) = host_object.audio_data {
        unsafe { alSourcef(source, AL_GAIN, volume); }
    }
}

- (())setCurrentTime:(NSTimeInterval)currentTime {
    log!("TODO: [(AVAudioPlayer *) {:?} setCurrentTime: {}]", this, currentTime);
}

- (())dealloc {
    cleanUpPreparation(env, this);
    let host_object = env.objc.borrow::<AVAudioPlayerHostObject>(this);
    release(env, host_object.url);
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

};

// TODO: Call this method when the sound finishes playing
fn cleanUpPreparation(env: &mut Environment, this: id) {
    let host_object = env.objc.borrow_mut::<AVAudioPlayerHostObject>(this);
    if let Some((buffer, source, data, data_layout)) = host_object.audio_data {
        unsafe {
            dealloc(data, data_layout);
            alDeleteSources(1, &source);
            alDeleteBuffers(1, &buffer);
        }
    }
    host_object.audio_data = None;
}
