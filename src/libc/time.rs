//! `time.h`

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::MutPtr;
use crate::Environment;
use std::time::SystemTime;

#[derive(Default)]
pub struct State {
    y2k38_warned: bool,
}

#[allow(non_camel_case_types)]
type time_t = i32;

fn time(env: &mut Environment, out: MutPtr<time_t>) -> time_t {
    let time64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let time = time64 as time_t;
    if !env.libc_state.time.y2k38_warned && time64 != time as u64 {
        env.libc_state.time.y2k38_warned = true;
        log!("Warning: system clock is beyond Y2K38 and might confuse the app");
    }
    if !out.is_null() {
        env.mem.write(out, time);
    }
    time
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(time(_))];
