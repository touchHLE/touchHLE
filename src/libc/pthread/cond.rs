use crate::dyld::FunctionExports;
use crate::environment::Environment;
use crate::export_c_func;
use crate::mem::{ConstVoidPtr, MutVoidPtr};

fn pthread_cond_init(_env: &mut Environment, cond: MutVoidPtr, attr: ConstVoidPtr) -> i32 {
    log!("TODO: pthread_cond_init({:?}, {:?})", cond, attr);
    0
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(pthread_cond_init(_, _))];
