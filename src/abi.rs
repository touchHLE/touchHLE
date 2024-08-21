/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Traits for application binary interface (ABI) translation, in particular
//! calling conventions.
//!
//! Useful resources:
//! * Apple's [Writing ARMv6 code for iOS](https://developer.apple.com/documentation/xcode/writing-armv6-code-for-ios), read together with Arm's [Procedure Call Standard for the Arm Architecture (AAPCS32)](https://github.com/ARM-software/abi-aa/blob/main/aapcs32/aapcs32.rst).
//!
//! See also: [crate::mem::SafeRead] and [crate::mem::SafeWrite].

use crate::cpu::Cpu;
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, Mem, MutPtr, MutVoidPtr, Ptr, SafeRead};
use crate::Environment;

/// The register number of the frame pointer in Apple's ABI.
pub const FRAME_POINTER: usize = 7;

/// Address of an A32 or T32 instruction, with the mode encoded using the Thumb
/// bit. This is also used as an (untyped) guest function pointer.
///
/// It is wrapped in a struct to prevent mixing with other pointers.
#[derive(Copy, Clone, Debug)]
pub struct GuestFunction(ConstVoidPtr);
unsafe impl SafeRead for GuestFunction {}
impl GuestFunction {
    pub const THUMB_BIT: u32 = 0x1;

    pub fn from_addr_with_thumb_bit(addr: u32) -> Self {
        GuestFunction(Ptr::from_bits(addr))
    }

    pub fn from_addr_and_thumb_flag(pc: u32, thumb: bool) -> Self {
        GuestFunction(Ptr::from_bits(pc | ((thumb as u32) * Self::THUMB_BIT)))
    }

    /// Returns `true` if the function uses regular Thumb (T32) instructions,
    /// or `false` if it uses regular regular Arm (A32) instructions.
    pub fn is_thumb(self) -> bool {
        self.0.to_bits() & Self::THUMB_BIT == Self::THUMB_BIT
    }

    pub fn addr_with_thumb_bit(self) -> u32 {
        self.0.to_bits()
    }

    /// Get the underlying address with the Thumb bit set to 0, i.e. the actual
    /// memory address of the first instruction.
    pub fn addr_without_thumb_bit(self) -> u32 {
        self.0.to_bits() & !Self::THUMB_BIT
    }

    pub fn to_ptr(self) -> ConstVoidPtr {
        self.0
    }

    /// Like [CallFromHost::call_from_host], but doesn't push a new guest stack
    /// frame. This is not a true tail call: PC and LR are still preserved.
    ///
    /// This is only needed in special applications where having a new stack
    /// frame would be troublesome, e.g. a tail call with stack argument
    /// pass-through.
    pub fn call_without_pushing_stack_frame(self, env: &mut Environment) {
        log_dbg!(
            "Begin call to guest function {:?} (no new stack frame)",
            self
        );

        let (old_pc, old_lr) = env
            .cpu
            .branch_with_link(self, env.dyld.return_to_host_routine());

        env.run_call();

        env.cpu.branch(old_pc);
        env.cpu.regs_mut()[Cpu::LR] = old_lr.addr_with_thumb_bit();

        log_dbg!(
            "End call to guest function {:?} (no stack frame popped)",
            self
        );
    }
}

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
///
/// See also [CallFromHost] and
/// [GuestFunction::call_without_pushing_stack_frame].
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
                let mut reg_offset = 0;
                let regs = env.cpu.regs();
                let retval_ptr = R::SIZE_IN_MEM.map(|_| {
                    read_next_arg(&mut reg_offset, regs, Ptr::from_bits(regs[Cpu::SP]), &env.mem)
                });
                let args: ($($P,)*) = {
                    ($(read_next_arg::<$P>(&mut reg_offset, regs, Ptr::from_bits(regs[Cpu::SP]), &env.mem),)*)
                };
                log_dbg!("CallFromGuest {:?}", args);
                let retval = self(env, $(args.$p),*);
                log_dbg!("CallFromGuest => {:?}", retval);
                if let Some(retval_ptr) = retval_ptr {
                    retval.to_mem(retval_ptr, &mut env.mem);
                } else {
                    retval.to_regs(env.cpu.regs_mut());
                }
            }
        }
        impl<R, $($P),*> CallFromGuest for fn(&mut Environment, $($P,)* DotDotDot) -> R
            where R: GuestRet, $($P: GuestArg,)* {
            // ignore warnings for the zero-argument case
            #[allow(unused_variables, unused_mut, clippy::unused_unit)]
            fn call_from_guest(&self, env: &mut Environment) {
                let mut reg_offset = 0;
                let regs = env.cpu.regs();
                let retval_ptr = R::SIZE_IN_MEM.map(|_| {
                    read_next_arg(&mut reg_offset, regs, Ptr::from_bits(regs[Cpu::SP]), &env.mem)
                });
                let args: ($($P,)*) = {
                    ($(read_next_arg::<$P>(&mut reg_offset, regs, Ptr::from_bits(regs[Cpu::SP]), &env.mem),)*)
                };
                let va_list = DotDotDot(VaList {
                    reg_offset,
                    stack_pointer: Ptr::from_bits(regs[Cpu::SP])
                });
                log_dbg!("CallFromGuest {:?}, ...{:?}", args, va_list);
                let retval = self(env, $(args.$p,)* va_list);
                log_dbg!("CallFromGuest => {:?}", retval);
                if let Some(retval_ptr) = retval_ptr {
                    retval.to_mem(retval_ptr, &mut env.mem);
                } else {
                    retval.to_regs(env.cpu.regs_mut());
                }
            }
        }
    }
}

impl_CallFromGuest!();
impl_CallFromGuest!(0 => P0);
impl_CallFromGuest!(0 => P0, 1 => P1);
impl_CallFromGuest!(0 => P0, 1 => P1, 2 => P2);
impl_CallFromGuest!(0 => P0, 1 => P1, 2 => P2, 3 => P3);
impl_CallFromGuest!(0 => P0, 1 => P1, 2 => P2, 3 => P3, 4 => P4);
impl_CallFromGuest!(0 => P0, 1 => P1, 2 => P2, 3 => P3, 4 => P4, 5 => P5);
impl_CallFromGuest!(0 => P0, 1 => P1, 2 => P2, 3 => P3, 4 => P4, 5 => P5, 6 => P6);
impl_CallFromGuest!(0 => P0, 1 => P1, 2 => P2, 3 => P3, 4 => P4, 5 => P5, 6 => P6, 7 => P7);
impl_CallFromGuest!(0 => P0, 1 => P1, 2 => P2, 3 => P3, 4 => P4, 5 => P5, 6 => P6, 7 => P7, 8 => P8);

/// This trait represents a guest or host function that can be called from host
/// code, but using the guest ABI. See [CallFromGuest], which this is the
/// inverse of.
pub trait CallFromHost<R, P> {
    /// Execute the "guest" function and return to the host when it is done.
    /// Note that this does not mean that the function is actually a guest
    /// function, just that it acts as one for this trait.
    ///
    /// For calls to actual guest functions, a new stack frame will be
    /// created for the duration of the call.
    ///
    /// For a host-to-guest call to work, several pieces need to work in
    /// concert:
    /// * A new stack frame is pushed (this is optional, but makes stack
    ///     traces clearer)
    /// * The arguments to the function are placed in registers or the
    ///     stack according to the calling convention. This is handled by
    ///     this trait; functions that pass through arguments to another
    ///     function (such as `objc_msgsend`) should call
    ///     [GuestFunction::call_without_pushing_stack_frame] instead.
    /// * The program counter (PC) and Thumb flag have to be set to match
    ///     the function being called;
    /// * The link register (LR) to point to a special routine for
    ///     returning to the host;
    /// * The emulated function eventually returns to the caller by jumping to
    ///     the address in the link register, which should be the special
    ///     routine.
    /// * The CPU emulation recognises the special routine and returns back to
    ///     this method.
    /// * This method restores the original PC, Thumb flag and LR, and pops any
    ///     stack arguments and the stack frame.
    /// * The return values are extracted from registers or the stack, if
    ///     appropriate.
    fn call_from_host(&self, env: &mut Environment, args: P) -> R;
}

macro_rules! impl_CallFromHost {
    ( $($p:tt => $P:ident),* ) => {
        impl <T, R, $($P),*> CallFromHost<R, ($($P,)*)> for T
            where T: CallFromGuest, R: GuestRet, $($P: GuestArg,)* {
            // ignore warnings for the zero-argument case
            #[allow(unused_variables, unused_mut, clippy::unused_unit)]
            fn call_from_host(
                &self,
                env: &mut Environment,
                args: ($($P,)*),
            ) -> R {
                let mut reg_offset = 0;
                let regs = env.cpu.regs_mut();
                let retval_ptr = R::SIZE_IN_MEM.map(|size| {
                    regs[Cpu::SP] -= size;
                    let ptr: ConstVoidPtr = Ptr::from_bits(regs[Cpu::SP]);
                    write_next_arg(&mut reg_offset, regs, &mut env.mem, ptr);
                    ptr
                });
                let old_sp = extend_stack_for_args(
                    0 $(+ <$P as GuestArg>::REG_COUNT)*,
                    regs,
                );
                $(write_next_arg::<$P>(&mut reg_offset, regs, &mut env.mem, args.$p);)*
                self.call_from_guest(env);
                let regs = env.cpu.regs_mut(); // reborrow
                regs[Cpu::SP] = old_sp;
                if let Some(retval_ptr) = retval_ptr {
                    regs[Cpu::SP] += R::SIZE_IN_MEM.unwrap();
                    <R as GuestRet>::from_mem(retval_ptr, &env.mem)
                } else {
                    <R as GuestRet>::from_regs(regs)
                }
            }
        }

        impl <R, $($P),*> CallFromHost<R, ($($P,)*)> for GuestFunction
            where R: GuestRet, $($P: GuestArg,)* {
            // ignore warnings for the zero-argument case
            #[allow(unused_variables, unused_mut, clippy::unused_unit)]
            fn call_from_host(
                &self,
                env: &mut Environment,
                args: ($($P,)*),
            ) -> R {
                log_dbg!("Begin call to guest function {:?}", self);

                let (old_pc, old_lr) = env
                    .cpu
                    .branch_with_link(*self, env.dyld.return_to_host_routine());

                // Create a new guest stack frame. This is redundant considering
                // we are storing this data on the host stack, but this makes
                // stack traces work nicely. :)
                let (old_sp, old_fp) = {
                    let regs = env.cpu.regs_mut();
                    let old_sp = regs[Cpu::SP];
                    let old_fp = regs[FRAME_POINTER];
                    regs[Cpu::SP] -= 8;
                    regs[FRAME_POINTER] = regs[Cpu::SP];
                    env.mem
                        .write(Ptr::from_bits(regs[Cpu::SP]), old_fp);
                    env.mem.write(Ptr::from_bits(regs[Cpu::SP] + 4), old_lr);
                    (old_sp, old_fp)
                };

                assert!(R::SIZE_IN_MEM.is_none()); // pointer return TODO
                let regs = env.cpu.regs_mut();
                let _ = extend_stack_for_args(
                    0 $(+ <$P as GuestArg>::REG_COUNT)*,
                    regs,
                );
                let mut reg_offset = 0;
                $(write_next_arg::<$P>(&mut reg_offset, regs, &mut env.mem, args.$p);)*

                // It would actually be possible to use
                // [GuestFunction::call_without_pushing_stack_frame] here, but
                // it would mess up debug logging, so duplicating the code
                // is easier.
                env.run_call();

                env.cpu.branch(old_pc);

                let regs = env.cpu.regs_mut();
                log_dbg!("End call to guest function {:?}", self);

                regs[Cpu::LR] = old_lr.addr_with_thumb_bit();
                regs[Cpu::SP] = old_sp;
                regs[FRAME_POINTER] = old_fp;
                <R as GuestRet>::from_regs(env.cpu.regs())
            }
        }

    }
}

impl_CallFromHost!();
impl_CallFromHost!(0 => P0);
impl_CallFromHost!(0 => P0, 1 => P1);
impl_CallFromHost!(0 => P0, 1 => P1, 2 => P2);
impl_CallFromHost!(0 => P0, 1 => P1, 2 => P2, 3 => P3);
impl_CallFromHost!(0 => P0, 1 => P1, 2 => P2, 3 => P3, 4 => P4);
impl_CallFromHost!(0 => P0, 1 => P1, 2 => P2, 3 => P3, 4 => P4, 5 => P5);
impl_CallFromHost!(0 => P0, 1 => P1, 2 => P2, 3 => P3, 4 => P4, 5 => P5, 6 => P6);
impl_CallFromHost!(0 => P0, 1 => P1, 2 => P2, 3 => P3, 4 => P4, 5 => P5, 6 => P6, 7 => P7);
impl_CallFromHost!(0 => P0, 1 => P1, 2 => P2, 3 => P3, 4 => P4, 5 => P5, 6 => P6, 7 => P7, 8 => P8);

/// Calling convention translation for a function argument type.
pub trait GuestArg: std::fmt::Debug + Sized {
    /// How many registers does this argument type consume?
    const REG_COUNT: usize;

    /// Read the argument from registers. Only `&regs[0..Self::REG_COUNT]` may
    /// be accessed.
    fn from_regs(regs: &[u32]) -> Self;

    /// Write the argument to registers. Only '&mut regs[0..Self::REG_COUNT]`
    /// may be accessed.
    fn to_regs(self, regs: &mut [u32]);
}

/// Read a single argument from registers or the stack. Call this for each
/// argument in order.
fn read_next_arg<T: GuestArg>(
    reg_offset: &mut usize,
    regs: &[u32],
    stack_ptr: ConstPtr<u32>,
    mem: &Mem,
) -> T {
    // After the fourth register is used, the arguments go on the stack.
    // In some cases the argument is split over both registers and the stack.

    // Rust doesn't allow [0u32; Trait::T] alas, so we need to set some
    // arbitrary limit. 16 is high enough for everything right now.
    let mut fake_regs = [0u32; 16];
    let fake_regs = &mut fake_regs[0..T::REG_COUNT];

    for fake_reg in fake_regs.iter_mut() {
        if *reg_offset < 4 {
            *fake_reg = regs[*reg_offset];
        } else {
            *fake_reg = mem.read(stack_ptr + (*reg_offset - 4).try_into().unwrap());
        }
        *reg_offset += 1;
    }

    T::from_regs(fake_regs)
}

/// Decrements the stack pointer to prepare for calling [write_next_arg]. Pass
/// the sum of the [GuestArg::REG_COUNT]s for all the arguments to be written,
/// and this will update the stack pointer if necessary, as well as returning
/// a copy of the original stack pointer so it can be restored later.
pub fn extend_stack_for_args(reg_count_sum: usize, regs: &mut [u32]) -> u32 {
    // After the fourth register is used, the arguments go on the stack.
    // In some cases the argument is split over both registers and the stack.

    let old = regs[Cpu::SP];
    if reg_count_sum > 4 {
        let old: ConstPtr<u32> = Ptr::from_bits(old);
        regs[Cpu::SP] = (old - (reg_count_sum - 4).try_into().unwrap()).to_bits()
    }
    old
}

/// Write a single argument to registers or the stack. Call this for each
/// argument in order.
///
/// If `reg_offset` is or will be >= 4, the stack pointer **must** be
/// appropriately decremented in advance! See [extend_stack_for_args].
pub fn write_next_arg<T: GuestArg>(
    reg_offset: &mut usize,
    regs: &mut [u32],
    mem: &mut Mem,
    arg: T,
) {
    // After the fourth register is used, the arguments go on the stack.
    // In some cases the argument is split over both registers and the stack.

    // Rust doesn't allow [0u32; Trait::T] alas.
    // 16 is high enough for everything right now.
    let mut fake_regs = [0u32; 16];
    let fake_regs = &mut fake_regs[0..T::REG_COUNT];
    arg.to_regs(fake_regs);

    for &mut fake_reg in fake_regs {
        if *reg_offset < 4 {
            regs[*reg_offset] = fake_reg;
        } else {
            let stack_ptr: MutPtr<u32> = Ptr::from_bits(regs[Cpu::SP]);
            mem.write(stack_ptr + (*reg_offset - 4).try_into().unwrap(), fake_reg);
        }
        *reg_offset += 1;
    }
}

/// Represents variable arguments in a [CallFromGuest] function signature,
/// like C `...`, e.g. in the signature of `printf()`. See also [VaList].
#[derive(Debug)]
pub struct DotDotDot(VaList);
impl DotDotDot {
    pub fn start(&self) -> VaList {
        self.0
    }
}

/// Calling convention translation for a variable arguments list (like C
/// `va_list`). When used as a function argument, this is equivalent to
/// passing a `va_list` struct as an argument, e.g. `vprintf()`.
/// See also [DotDotDot].
#[derive(Copy, Clone, Debug)]
pub struct VaList {
    reg_offset: usize,
    stack_pointer: ConstVoidPtr,
}
impl VaList {
    /// Get the next argument, like C's `va_arg()`. Be careful as the type may
    /// be inferred from the call-site if you don't specify it explicitly.
    pub fn next<T: GuestArg>(&mut self, env: &mut Environment) -> T {
        let sp_reg = self.stack_pointer.cast();
        read_next_arg(&mut self.reg_offset, env.cpu.regs_mut(), sp_reg, &env.mem)
    }
}

macro_rules! impl_GuestArg_with {
    ($for:ty, $with:ty) => {
        impl GuestArg for $for {
            const REG_COUNT: usize = <$with as GuestArg>::REG_COUNT;
            fn from_regs(regs: &[u32]) -> Self {
                <$with as GuestArg>::from_regs(regs) as $for
            }
            fn to_regs(self, regs: &mut [u32]) {
                <$with as GuestArg>::to_regs(self as $with, regs)
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
    fn to_regs(self, regs: &mut [u32]) {
        regs[0] = self;
    }
}

impl_GuestArg_with!(i32, u32);
impl_GuestArg_with!(u16, u32);
impl_GuestArg_with!(i16, u32);
impl_GuestArg_with!(u8, u32);
impl_GuestArg_with!(i8, u32);

impl GuestArg for bool {
    const REG_COUNT: usize = 1;
    fn from_regs(regs: &[u32]) -> Self {
        <u32 as GuestArg>::from_regs(regs) != 0
    }
    fn to_regs(self, regs: &mut [u32]) {
        <u32 as GuestArg>::to_regs(self as u32, regs)
    }
}

impl GuestArg for f32 {
    const REG_COUNT: usize = <u32 as GuestArg>::REG_COUNT;
    fn from_regs(regs: &[u32]) -> Self {
        Self::from_bits(<u32 as GuestArg>::from_regs(regs))
    }
    fn to_regs(self, regs: &mut [u32]) {
        <u32 as GuestArg>::to_regs(self.to_bits(), regs)
    }
}

impl<T, const MUT: bool> GuestArg for Ptr<T, MUT> {
    const REG_COUNT: usize = <u32 as GuestArg>::REG_COUNT;
    fn from_regs(regs: &[u32]) -> Self {
        Self::from_bits(<u32 as GuestArg>::from_regs(regs))
    }
    fn to_regs(self, regs: &mut [u32]) {
        <u32 as GuestArg>::to_regs(self.to_bits(), regs)
    }
}

impl GuestArg for GuestFunction {
    const REG_COUNT: usize = <ConstVoidPtr as GuestArg>::REG_COUNT;
    fn from_regs(regs: &[u32]) -> Self {
        GuestFunction(<ConstVoidPtr as GuestArg>::from_regs(regs))
    }
    fn to_regs(self, regs: &mut [u32]) {
        <ConstVoidPtr as GuestArg>::to_regs(self.0, regs)
    }
}

impl GuestArg for VaList {
    const REG_COUNT: usize = <ConstVoidPtr as GuestArg>::REG_COUNT;
    fn from_regs(regs: &[u32]) -> Self {
        // `reg_offset` initialized to 4 as we want to use `stack_pointer` when
        // calling [read_next_arg]
        VaList {
            reg_offset: 4,
            stack_pointer: <ConstVoidPtr as GuestArg>::from_regs(regs),
        }
    }
    fn to_regs(self, _regs: &mut [u32]) {
        todo!()
    }
}

// GuestArg implementations for u64-like types

impl GuestArg for u64 {
    const REG_COUNT: usize = 2;
    fn from_regs(regs: &[u32]) -> Self {
        let mut bytes = [0u8; 8];
        bytes[0..4].copy_from_slice(&regs[0].to_le_bytes());
        bytes[4..8].copy_from_slice(&regs[1].to_le_bytes());
        u64::from_le_bytes(bytes)
    }
    fn to_regs(self, regs: &mut [u32]) {
        let bytes = self.to_le_bytes();
        regs[0] = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        regs[1] = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    }
}

impl_GuestArg_with!(i64, u64);

impl GuestArg for f64 {
    const REG_COUNT: usize = <u64 as GuestArg>::REG_COUNT;
    fn from_regs(regs: &[u32]) -> Self {
        Self::from_bits(<u64 as GuestArg>::from_regs(regs))
    }
    fn to_regs(self, regs: &mut [u32]) {
        <u64 as GuestArg>::to_regs(self.to_bits(), regs)
    }
}

// TODO: Do we need to distinguish arguments from return types, don't they
// usually behave the same? Are there exceptions? Do we merge the types?

/// Calling convention translation for a function return type.
pub trait GuestRet: std::fmt::Debug + Sized {
    /// If this is `None`, then the return value is passed directly in
    /// registers and the `to_regs` and `from_regs` methods should be used.
    /// If this is `Some(size)`, then the return value is of `size` bytes and is
    /// stored in memory at the location specified by an implicit pointer
    /// argument in r0, and the `to_mem` and `from_mem` methods should be used.
    const SIZE_IN_MEM: Option<GuestUSize> = None;

    /// Read the return value from registers.
    fn from_regs(regs: &[u32]) -> Self {
        let _ = regs;
        panic!()
    }
    /// Write the return value to registers.
    fn to_regs(self, regs: &mut [u32]) {
        let _ = regs;
        panic!()
    }

    /// Read the return value from memory.
    fn from_mem(ptr: ConstVoidPtr, mem: &Mem) -> Self {
        let _ = (ptr, mem);
        panic!()
    }
    /// Write the return value to memory.
    fn to_mem(self, ptr: MutVoidPtr, mem: &mut Mem) {
        let _ = (ptr, mem);
        panic!()
    }
}

macro_rules! impl_GuestRet_with {
    ($for:ty, $with:ty) => {
        impl GuestRet for $for {
            fn from_regs(regs: &[u32]) -> Self {
                <$with as GuestRet>::from_regs(regs) as $for
            }
            fn to_regs(self, regs: &mut [u32]) {
                <$with as GuestRet>::to_regs(self as $with, regs)
            }
        }
    };
}

/// Generates a trait implementation of [GuestRet] for a struct type that is
/// larger than 4 bytes (and thus returned via an implicit pointer parameter
/// rather than via registers). The type must have implementations of
/// [crate::mem::SafeRead] and [crate::mem::SafeWrite].
#[macro_export]
macro_rules! impl_GuestRet_for_large_struct {
    ($for:ty) => {
        impl $crate::abi::GuestRet for $for {
            const SIZE_IN_MEM: Option<$crate::mem::GuestUSize> =
                Some($crate::mem::guest_size_of::<$for>());

            fn from_mem(ptr: $crate::mem::ConstVoidPtr, mem: &$crate::mem::Mem) -> Self {
                let ptr = ptr.cast::<Self>();
                mem.read(ptr)
            }
            fn to_mem(self, ptr: $crate::mem::MutVoidPtr, mem: &mut $crate::mem::Mem) {
                let ptr = ptr.cast::<Self>();
                mem.write(ptr, self)
            }
        }
    };
}
pub use crate::impl_GuestRet_for_large_struct; // #[macro_export] is weird...

impl GuestRet for () {
    fn to_regs(self, _regs: &mut [u32]) {
        // objc_msgSend (see src/objc/messages.rs) relies on this not touching
        // the registers, because () will be "returned" after the function it's
        // meant to be tail-calling.
    }
    fn from_regs(_regs: &[u32]) -> Self {}
}

// GuestRet implementations for u32-like types

impl GuestRet for u32 {
    fn from_regs(regs: &[u32]) -> Self {
        regs[0]
    }
    fn to_regs(self, regs: &mut [u32]) {
        regs[0] = self;
    }
}

impl_GuestRet_with!(i32, u32);
impl_GuestRet_with!(u16, u32);
impl_GuestRet_with!(i16, u32);
impl_GuestRet_with!(u8, u32);
impl_GuestRet_with!(i8, u32);

impl GuestRet for bool {
    fn from_regs(regs: &[u32]) -> Self {
        <u32 as GuestRet>::from_regs(regs) != 0
    }
    fn to_regs(self, regs: &mut [u32]) {
        <u32 as GuestRet>::to_regs(self as u32, regs)
    }
}

impl GuestRet for f32 {
    fn from_regs(regs: &[u32]) -> Self {
        Self::from_bits(<u32 as GuestRet>::from_regs(regs))
    }
    fn to_regs(self, regs: &mut [u32]) {
        <u32 as GuestRet>::to_regs(self.to_bits(), regs)
    }
}

impl<T, const MUT: bool> GuestRet for Ptr<T, MUT> {
    fn from_regs(regs: &[u32]) -> Self {
        Self::from_bits(<u32 as GuestRet>::from_regs(regs))
    }
    fn to_regs(self, regs: &mut [u32]) {
        <u32 as GuestRet>::to_regs(self.to_bits(), regs)
    }
}

// GuestRet implementations for u64-like types

impl GuestRet for u64 {
    fn from_regs(regs: &[u32]) -> Self {
        let mut bytes = [0u8; 8];
        bytes[0..4].copy_from_slice(&regs[0].to_le_bytes());
        bytes[4..8].copy_from_slice(&regs[1].to_le_bytes());
        u64::from_le_bytes(bytes)
    }
    fn to_regs(self, regs: &mut [u32]) {
        let bytes = self.to_le_bytes();
        regs[0] = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        regs[1] = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    }
}

impl_GuestRet_with!(i64, u64);

impl GuestRet for f64 {
    fn from_regs(regs: &[u32]) -> Self {
        Self::from_bits(<u64 as GuestRet>::from_regs(regs))
    }
    fn to_regs(self, regs: &mut [u32]) {
        <u64 as GuestRet>::to_regs(self.to_bits(), regs)
    }
}
