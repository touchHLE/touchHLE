//! CPU emulation.
//!
//! Implemented using the C++ library dynarmic, which is a dynamic recompiler.

// See build.rs and src/cpu/dynarmic_wrapper.cpp
extern "C" {
    fn test_cpu_by_adding_numbers(a: i32, b: i32) -> i32;
}

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
