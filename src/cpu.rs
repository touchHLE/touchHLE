//! CPU emulation.
//!
//! Implemented using the C++ library dynarmic, which is a dynamic recompiler.

use touchHLE_dynarmic_wrapper::*;

pub struct Cpu {}

impl Cpu {
    pub fn new() -> Cpu {
        println!("According to dynarmic, 1 + 2 = {}!", unsafe {
            test_cpu_by_adding_numbers(1, 2)
        });
        // TODO: Actual CPU implementation
        Cpu {}
    }
}
