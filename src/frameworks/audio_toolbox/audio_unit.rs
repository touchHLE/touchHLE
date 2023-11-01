use crate::{environment::Environment, mem::{MutVoidPtr, ConstVoidPtr, ConstPtr}, frameworks::carbon_core::OSStatus, dyld::FunctionExports, export_c_func};

fn AudioUnitInitialize(_env: &mut Environment, inUnit: MutVoidPtr) -> OSStatus {
    log!("TODO: AudioUnitInitialize({:?})", inUnit);
    0 // success
}

fn AudioUnitSetProperty(_env: &mut Environment, inUnit: MutVoidPtr, inID: u32, inScope: u32, inElement: u32, inData: ConstVoidPtr, inDataSize: u32) -> OSStatus {
    log!("TODO: AudioUnitSetProperty({:?}, {:?}, {:?}, {:?}, {:?}, {:?})", inUnit, inID, inScope, inElement, inData, inDataSize);
    0 // success
}

fn AudioUnitGetProperty(_env: &mut Environment, inUnit: MutVoidPtr, inID: u32, inScope: u32, inElement: u32, outData: ConstVoidPtr, ioDataSize: ConstPtr<u32>) -> OSStatus {
    log!("TODO: AudioUnitGetProperty({:?}, {:?}, {:?}, {:?}, {:?}, {:?})", inUnit, inID, inScope, inElement, outData, ioDataSize);
    0 // success
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(AudioUnitInitialize(_)),
    export_c_func!(AudioUnitSetProperty(_, _, _, _, _, _)),
    export_c_func!(AudioUnitGetProperty(_, _, _, _, _, _)),
];
