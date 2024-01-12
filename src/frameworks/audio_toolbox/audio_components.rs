use std::collections::HashMap;

use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::frameworks::carbon_core::OSStatus;
use crate::frameworks::core_audio_types::fourcc;
use crate::mem::{ConstPtr, ConstVoidPtr, MutPtr, SafeRead};
use crate::objc::nil;

const kAudioUnitType_Output: u32 = fourcc(b"auou");
const kAudioUnitSubType_RemoteIO: u32 = fourcc(b"rioc");
const kAudioUnitManufacturer_Apple: u32 = fourcc(b"appl");

#[derive(Default)]
pub struct State {
    pub audio_component_instances:
        HashMap<AudioComponentInstance, AudioComponentInstanceHostObject>,
}
impl State {
    pub fn get(framework_state: &mut crate::frameworks::State) -> &mut Self {
        &mut framework_state.audio_toolbox.audio_components
    }
}

#[derive(Default, Clone)]
pub struct AudioComponentInstanceHostObject {
    pub started: bool,
    pub render_callback: Option<AURenderCallbackStruct>,
}

#[derive(Clone)]
#[repr(C, packed)]
pub struct AURenderCallbackStruct {
    pub inputProc: ConstVoidPtr,
    pub inputProcRefCon: ConstVoidPtr,
}
unsafe impl SafeRead for AURenderCallbackStruct {}

#[repr(C, packed)]
struct OpaqueAudioComponent {}
unsafe impl SafeRead for OpaqueAudioComponent {}

type AudioComponent = MutPtr<OpaqueAudioComponent>;

#[repr(C, packed)]
pub struct OpaqueAudioComponentInstance {
    _pad: u8,
}
unsafe impl SafeRead for OpaqueAudioComponentInstance {}

pub type AudioComponentInstance = MutPtr<OpaqueAudioComponentInstance>;

#[repr(C, packed)]
struct AudioComponentDescription {
    componentType: u32,
    componentSubType: u32,
    componentManufacturer: u32,
    componentFlags: u32,
    componentFlagsMask: u32,
}
unsafe impl SafeRead for AudioComponentDescription {}

fn AudioComponentFindNext(
    env: &mut Environment,
    inComponent: AudioComponent,
    inDesc: ConstPtr<AudioComponentDescription>,
) -> AudioComponent {
    let audio_comp_descr = env.mem.read(inDesc);
    assert!(audio_comp_descr.componentType == kAudioUnitType_Output);
    assert!(audio_comp_descr.componentSubType == kAudioUnitSubType_RemoteIO);
    assert!(audio_comp_descr.componentManufacturer == kAudioUnitManufacturer_Apple);

    let out_component = nil.cast();
    log!(
        "TODO: AudioComponentFindNext({:?}, {:?}) -> {:?}",
        inComponent,
        inDesc,
        out_component
    );
    out_component
}

fn AudioComponentInstanceNew(
    env: &mut Environment,
    inComponent: AudioComponent,
    outInstance: MutPtr<AudioComponentInstance>,
) -> OSStatus {
    let host_object = AudioComponentInstanceHostObject::default();

    let guest_instance: AudioComponentInstance = env
        .mem
        .alloc_and_write(OpaqueAudioComponentInstance { _pad: 0 });
    State::get(&mut env.framework_state)
        .audio_component_instances
        .insert(guest_instance, host_object);

    env.mem.write(outInstance, guest_instance);

    let result = 0; // success
    log_dbg!(
        "AudioComponentInstanceNew({:?}, {:?}) -> {:?}",
        inComponent,
        outInstance,
        result
    );
    result
}

fn AudioComponentInstanceDispose(
    env: &mut Environment,
    inInstance: AudioComponentInstance,
) -> OSStatus {
    let result = if inInstance.is_null() {
        -50
    } else {
        State::get(&mut env.framework_state)
            .audio_component_instances
            .remove(&inInstance);
        env.mem.free(inInstance.cast());
        0
    };
    log_dbg!(
        "AudioComponentInstanceDispose({:?}) -> {:?}",
        inInstance,
        result
    );
    result
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioComponentFindNext(_, _)),
    export_c_func!(AudioComponentInstanceNew(_, _)),
    export_c_func!(AudioComponentInstanceDispose(_)),
];
