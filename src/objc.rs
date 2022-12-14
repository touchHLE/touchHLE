//! Objective-C runtime.

use crate::cpu::Cpu;
use crate::dyld::FunctionExports;
use crate::Environment;

#[allow(non_snake_case)]
fn objc_msgSend(
    env: &mut Environment,
    self_: u32,
    op: u32,
    // other arguments not handled yet
) {
    unimplemented!(
        "objc_msgSend({:#x}, {:#x}, ...) called from {:#x}",
        self_,
        op,
        env.cpu.regs()[Cpu::PC]
    );
}

pub const FUNCTIONS: FunctionExports = &[(
    "_objc_msgSend",
    &(objc_msgSend as fn(&mut Environment, u32, u32)),
)];
