/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::dyld::{ConstantExports, HostConstant};
use crate::mem::MutPtr;
use crate::objc::{id, objc_classes, ClassExports, HostObject, nil};

#[derive(Default)]
pub struct State {
    av_audio_session: Option<id>,
}

struct AudioSessionHost {
    delegate: id, //Unretained
}
impl HostObject for AudioSessionHost {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation AVAudioSession: NSObject

+ (id)sharedInstance {
    if let Some(sess) = env.framework_state.avf_audio.av_audio_session.av_audio_session {
        sess
    } else {
        let new = env.objc.alloc_static_object(
            this,
            Box::new(AudioSessionHost{delegate: nil}),
            &mut env.mem
        );
        env.framework_state.avf_audio.av_audio_session.av_audio_session = Some(new);
        new
   }
}

- (())setDelegate:(id)new {
    env.objc.borrow_mut::<AudioSessionHost>(this).delegate = new;
}

- (bool)setCategory:(id)_category
              error:(MutPtr<id>)err {
    if !err.is_null() {
        env.mem.write(err, nil)
    }
    true
}

- (id)retain { this }
- (())release {}
- (id)autorelease { this }

@end

};

pub const CONSTANTS: ConstantExports = &[
    (
        "_AVAudioSessionCategorySoloAmbient",
        HostConstant::NSString("AVAudioSessionCategorySoloAmbient"),
    ),
    (
        "_AVAudioSessionCategoryAmbient",
        HostConstant::NSString("AVAudioSessionCategoryAmbient"),
    ),
];