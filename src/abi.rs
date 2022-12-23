//! Traits for application binary interface (ABI) translation, in particular
//! calling conventions.
//!
//! Useful resources:
//! * Apple's [Writing ARMv6 code for iOS](https://developer.apple.com/documentation/xcode/writing-armv6-code-for-ios), read together with Arm's [Procedure Call Standard for the Arm Architecture (AAPCS32)](https://github.com/ARM-software/abi-aa).

use crate::mem::Ptr;
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

macro_rules! impl_CallFromGuest {
    ( $($p:tt => $P:ident),* ) => {
        impl<R, $($P),*> CallFromGuest for fn(&mut Environment, $($P),*) -> R
            where R: GuestRet, $($P: GuestArg,)* {
            // ignore warnings for the zero-argument case
            #[allow(unused_variables, unused_mut, clippy::unused_unit)]
            fn call_from_guest(&self, env: &mut Environment) {
                let args: ($($P,)*) = {
                    let regs = env.cpu.regs();
                    let mut reg_offset = 0;
                    ($(read_next_arg::<$P>(&mut reg_offset, regs),)*)
                };
                let retval = self(env, $(args.$p),*);
                retval.to_regs(env.cpu.regs_mut());
            }
        }
    }
}

impl_CallFromGuest!();
impl_CallFromGuest!(0 => P0);
impl_CallFromGuest!(0 => P0, 1 => P1);
impl_CallFromGuest!(0 => P0, 1 => P1, 2 => P2);
impl_CallFromGuest!(0 => P0, 1 => P1, 2 => P2, 3 => P3);

/// Calling convention translation for a function argument type.
pub trait GuestArg: Sized {
    /// How many registers does this argument type consume?
    const REG_COUNT: usize;

    /// Read the argument from registers. Only `&regs[0..Self::REG_COUNT]` may
    /// be accessed.
    fn from_regs(regs: &[u32]) -> Self;
}

/// Read a single argument from registers. Call this for each argument in order.
fn read_next_arg<T: GuestArg>(reg_offset: &mut usize, regs: &[u32]) -> T {
    // After the fourth register is used, the arguments go on the stack.
    // (Support not implemented yet, Rust will panic if indexing out-of-bounds.)
    let regs = &regs[0..4];

    let val = T::from_regs(&regs[*reg_offset..][..T::REG_COUNT]);
    *reg_offset += T::REG_COUNT;
    val
}

macro_rules! impl_GuestArg_with {
    ($for:ty, $with:ty) => {
        impl GuestArg for $for {
            const REG_COUNT: usize = <$with as GuestArg>::REG_COUNT;
            fn from_regs(regs: &[u32]) -> Self {
                <$with as GuestArg>::from_regs(regs) as $for
            }
        }
    };
}

// GuestArg implementations for u32-like types

impl GuestArg for u32 {
    const REG_COUNT: usize = 1;
    fn from_regs(regs: &[u32]) -> Self {
        regs[0]
    }
}

impl_GuestArg_with!(i32, u32);
impl_GuestArg_with!(u16, u32);
impl_GuestArg_with!(i16, u32);
impl_GuestArg_with!(u8, u32);
impl_GuestArg_with!(i8, u32);

impl GuestArg for f32 {
    const REG_COUNT: usize = <u32 as GuestArg>::REG_COUNT;
    fn from_regs(regs: &[u32]) -> Self {
        Self::from_bits(<u32 as GuestArg>::from_regs(regs))
    }
}

impl<T, const MUT: bool> GuestArg for Ptr<T, MUT> {
    const REG_COUNT: usize = <u32 as GuestArg>::REG_COUNT;
    fn from_regs(regs: &[u32]) -> Self {
        Self::from_bits(<u32 as GuestArg>::from_regs(regs))
    }
}

// TODO: Do we need to distinguish arguments from return types, don't they
// usually behave the same? Are there exceptions? Do we merge the types?

/// Calling convention translation for a function return type.
pub trait GuestRet: Sized {
    // The main purpose of GuestArg::REG_COUNT is for advancing the register
    // index. But there can only be one return value for a function, so we can
    // probably get away with not having it for now?

    /// Write the return value to registers.
    fn to_regs(self, regs: &mut [u32]);
}

macro_rules! impl_GuestRet_with {
    ($for:ty, $with:ty) => {
        impl GuestRet for $for {
            fn to_regs(self, regs: &mut [u32]) {
                <$with as GuestRet>::to_regs(self as $with, regs)
            }
        }
    };
}

impl GuestRet for () {
    fn to_regs(self, _regs: &mut [u32]) {
        // objc_msgSend (see src/objc/messages.rs) relies on this not touching
        // the registers, because () will be "returned" after the function it's
        // meant to be tail-calling.
    }
}

// GuestRet implementations for u32-like types

impl GuestRet for u32 {
    fn to_regs(self, regs: &mut [u32]) {
        regs[0] = self;
    }
}

impl_GuestRet_with!(i32, u32);
impl_GuestRet_with!(u16, u32);
impl_GuestRet_with!(i16, u32);
impl_GuestRet_with!(u8, u32);
impl_GuestRet_with!(i8, u32);

impl GuestRet for f32 {
    fn to_regs(self, regs: &mut [u32]) {
        <u32 as GuestRet>::to_regs(self.to_bits(), regs)
    }
}

impl<T, const MUT: bool> GuestRet for Ptr<T, MUT> {
    fn to_regs(self, regs: &mut [u32]) {
        <u32 as GuestRet>::to_regs(self.to_bits(), regs)
    }
}
