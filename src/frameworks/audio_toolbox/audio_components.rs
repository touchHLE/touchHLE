use crate::{objc::nil, mem::{ConstVoidPtr, MutVoidPtr}, export_c_func, dyld::FunctionExports, environment::Environment, frameworks::carbon_core::OSStatus};

fn AudioComponentInstanceNew(_env: &mut Environment, inComponent: MutVoidPtr, outInstance: MutVoidPtr) -> OSStatus {
    log!("TODO: AudioComponentInstanceNew({:?}, {:?})", inComponent, outInstance);
    0 // success
}

fn AudioComponentFindNext(_env: &mut Environment, inComponent: MutVoidPtr, inDesc: ConstVoidPtr) -> MutVoidPtr {
    log!("TODO: AudioComponentFindNext({:?}, {:?})", inComponent, inDesc);
    nil.cast()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioComponentInstanceNew(_, _)),
    export_c_func!(AudioComponentFindNext(_, _)),
];
