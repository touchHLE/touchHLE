//! Traits for application binary interface (ABI) translation, in particular
//! calling conventions.
//!
//! Useful resources:
//! * Apple's [Writing ARMv6 code for iOS](https://developer.apple.com/documentation/xcode/writing-armv6-code-for-ios), read together with Arm's [Procedure Call Standard for the Arm Architecture (AAPCS32)](https://github.com/ARM-software/abi-aa).

use crate::Environment;

/// This trait represents a host function that can be called from guest code.
///
/// For a guest-to-host call to work, several pieces need to work in concert:
/// * The dynamic linker ([crate::dyld]) creates a stub guest function that the
///   guest code can call.
/// * When the guest code calls the stub function, the CPU emulation is paused,
///   and the correct [CallFromGuest] implementation is looked up and called.
/// * The [CallFromGuest] implementation
///   * extracts the arguments, if any, from the guest CPU registers or guest
///     stack, according to the calling convention;
///   * executes the actual implementation of the function; and
///   * writes the return value, if any, to the guest CPU registers or guest
///     stack, according to the calling convention.
/// * The CPU emulation is resumed.
/// * The stub function returns to the caller (if appropriate).
///
/// This module should provide generic implementations of this trait for Rust
/// [function pointers][fn] with compatible argument and return types. Only
/// unusual cases should need to provide their own implementation.
pub trait CallFromGuest {
    fn call_from_guest(&self, env: &mut Environment);
}

// TODO: implementations for other parameter types and counts
impl CallFromGuest for fn(&mut Environment, u32, u32) {
    fn call_from_guest(&self, env: &mut Environment) {
        let args = (env.cpu.regs()[0], env.cpu.regs()[1]);
        self(env, args.0, args.1)
    }
}
