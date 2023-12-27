/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Types related to the virtual memory of the emulated application, or the
//! "guest memory".
//!
//! The virtual address space is 32-bit, as is the pointer size.
//!
//! No attempt is made to do endianness conversion for reads and writes to
//! memory, because all supported emulated and host platforms are little-endian.
//!
//! Relevant Apple documentation:
//! * [Memory Usage Performance Guidelines](https://developer.apple.com/library/archive/documentation/Performance/Conceptual/ManagingMemory/ManagingMemory.html)

use crate::libc::wchar::wchar_t;

mod allocator;

/// Equivalent of `usize` for guest memory.
pub type GuestUSize = u32;

/// Equivalent of `isize` for guest memory.
pub type GuestISize = i32;

/// [std::mem::size_of], but returning a [GuestUSize].
pub const fn guest_size_of<T: Sized>() -> GuestUSize {
    assert!(std::mem::size_of::<T>() <= u32::MAX as usize);
    std::mem::size_of::<T>() as u32
}

/// Internal type for representing an untyped virtual address.
type VAddr = GuestUSize;

/// Pointer type for guest memory, or the "guest pointer" type.
///
/// The `MUT` type parameter determines whether this is mutable or not.
/// Don't write it out explicitly, use [ConstPtr], [MutPtr], [ConstVoidPtr] or
/// [MutVoidPtr] instead instead.
///
/// The implemented methods try to mirror the Rust [pointer] type's methods,
/// where possible.
#[repr(transparent)]
pub struct Ptr<T, const MUT: bool>(VAddr, std::marker::PhantomData<T>);

// #[derive(...)] doesn't work for this type because it expects T to have the
// trait we want implemented
impl<T, const MUT: bool> Clone for Ptr<T, MUT> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, const MUT: bool> Copy for Ptr<T, MUT> {}
impl<T, const MUT: bool> PartialEq for Ptr<T, MUT> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T, const MUT: bool> Eq for Ptr<T, MUT> {}
impl<T, const MUT: bool> std::hash::Hash for Ptr<T, MUT> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// Constant guest pointer type (like Rust's `*const T`).
pub type ConstPtr<T> = Ptr<T, false>;
/// Mutable guest pointer type (like Rust's `*mut T`).
pub type MutPtr<T> = Ptr<T, true>;
#[allow(dead_code)]
/// Constant guest pointer-to-void type (like C's `const void *`)
pub type ConstVoidPtr = ConstPtr<std::ffi::c_void>;
/// Mutable guest pointer-to-void type (like C's `void *`)
pub type MutVoidPtr = MutPtr<std::ffi::c_void>;

impl<T, const MUT: bool> Ptr<T, MUT> {
    pub const fn null() -> Self {
        Ptr(0, std::marker::PhantomData)
    }

    pub fn to_bits(self) -> VAddr {
        self.0
    }
    pub const fn from_bits(bits: VAddr) -> Self {
        Ptr(bits, std::marker::PhantomData)
    }

    pub fn cast<U>(self) -> Ptr<U, MUT> {
        Ptr::<U, MUT>::from_bits(self.to_bits())
    }

    pub fn cast_void(self) -> Ptr<std::ffi::c_void, MUT> {
        self.cast()
    }

    pub fn is_null(self) -> bool {
        self.to_bits() == 0
    }
}

impl<T> ConstPtr<T> {
    #[allow(dead_code)]
    pub fn cast_mut(self) -> MutPtr<T> {
        Ptr::from_bits(self.to_bits())
    }
}
impl<T> MutPtr<T> {
    pub fn cast_const(self) -> ConstPtr<T> {
        Ptr::from_bits(self.to_bits())
    }
}

impl<T, const MUT: bool> Default for Ptr<T, MUT> {
    fn default() -> Self {
        Self::null()
    }
}

impl<T, const MUT: bool> std::fmt::Debug for Ptr<T, MUT> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_null() {
            write!(f, "(null)")
        } else {
            write!(f, "{:#x}", self.to_bits())
        }
    }
}

// C-like pointer arithmetic
impl<T, const MUT: bool> std::ops::Add<GuestUSize> for Ptr<T, MUT> {
    type Output = Self;

    fn add(self, other: GuestUSize) -> Self {
        let size: GuestUSize = guest_size_of::<T>();
        assert_ne!(size, 0);
        Self::from_bits(
            self.to_bits()
                .checked_add(other.checked_mul(size).unwrap())
                .unwrap(),
        )
    }
}
impl<T, const MUT: bool> std::ops::AddAssign<GuestUSize> for Ptr<T, MUT> {
    fn add_assign(&mut self, rhs: GuestUSize) {
        *self = *self + rhs;
    }
}
impl<T, const MUT: bool> std::ops::Sub<GuestUSize> for Ptr<T, MUT> {
    type Output = Self;

    fn sub(self, other: GuestUSize) -> Self {
        let size: GuestUSize = guest_size_of::<T>();
        assert_ne!(size, 0);
        Self::from_bits(
            self.to_bits()
                .checked_sub(other.checked_mul(size).unwrap())
                .unwrap(),
        )
    }
}
impl<T, const MUT: bool> std::ops::SubAssign<GuestUSize> for Ptr<T, MUT> {
    fn sub_assign(&mut self, rhs: GuestUSize) {
        *self = *self - rhs;
    }
}

/// Marker trait for types that can be safely read from guest memory.
///
/// See also [SafeWrite] and [crate::abi].
///
/// # Safety
/// Reading from guest memory is essentially doing a [std::mem::transmute],
/// which is notoriously unsafe in Rust. Only types for which all possible bit
/// patterns are legal (e.g. integers) should have this trait.
pub unsafe trait SafeRead: Sized {}
// bool is one byte in size and has 0 as false, 1 as true in both Rust and ObjC
unsafe impl SafeRead for bool {}
unsafe impl SafeRead for i8 {}
unsafe impl SafeRead for u8 {}
unsafe impl SafeRead for i16 {}
unsafe impl SafeRead for u16 {}
unsafe impl SafeRead for i32 {}
unsafe impl SafeRead for u32 {}
unsafe impl SafeRead for i64 {}
unsafe impl SafeRead for u64 {}
unsafe impl SafeRead for f32 {}
unsafe impl SafeRead for f64 {}
unsafe impl<T, const MUT: bool> SafeRead for Ptr<T, MUT> {}

/// Marker trait for types that can be written to guest memory.
///
/// Unlike for [SafeRead], there is no (Rust) safety consideration here; it's
/// just a way to catch accidental use of types unintended for guest use.
/// This was added after discovering that `()` is "[Sized]" and therefore a
/// single stray semicolon can wreak havoc...
///
/// Especially for structs, be careful that the type matches the expected ABI.
/// At minimum you should have `#[repr(C, packed)]` and appropriate padding
/// members.
///
/// See also [SafeRead] and [crate::abi].
pub trait SafeWrite: Sized {}
impl<T: SafeRead> SafeWrite for T {}

type Bytes = [u8; 1 << 32];

/// The type that owns the guest memory and provides accessors for it.
pub struct Mem {
    /// This array is 4GiB in size so that it can cover the entire 32-bit
    /// virtual address space, but it should not use that much physical memory,
    /// assuming that the host OS backs it with lazily-allocated pages and we
    /// are careful to avoid accessing most of it.
    ///
    /// iPhone OS devices only had 128MiB or 256MiB of RAM total, with no swap
    /// space, so less than 6.25% of this array should be used, assuming no
    /// fragmentation.
    ///
    /// This is a raw pointer because inevitably we will have to hand out
    /// pointers to memory sometimes, and being able to hold a `&mut` on this
    /// array simultaneously seems like an undefined behavior trap. This also
    /// means that the underlying memory should never be moved, and therefore
    /// the array can't be growable.
    ///
    /// One advantage of `[u8; 1 << 32]` over `[u8]` is that it might help rustc
    /// optimize away bounds checks for `memory.bytes[ptr_32bit as usize]`.
    ///
    /// Note that unless direct memory access is disabled, the CPU emulation
    /// (dynarmic) accesses memory via this pointer directly except when a page
    /// fault occurs.
    bytes: *mut Bytes,

    /// The size of the __PAGE_ZERO segment, where pointer accesses are trapped
    /// to prevent null pointer derefrences.
    ///
    /// We don't have full memory protection, but we can check accesses in that
    /// range.
    null_segment_size: VAddr,

    allocator: allocator::Allocator,
}

impl Drop for Mem {
    fn drop(&mut self) {
        let layout = std::alloc::Layout::new::<Bytes>();
        unsafe {
            std::alloc::dealloc(self.bytes as *mut _, layout);
        }
    }
}

impl Mem {
    /// [According to Apple](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Multithreading/CreatingThreads/CreatingThreads.html)
    /// among others, the iPhone OS main thread stack size is 1MiB.
    pub const MAIN_THREAD_STACK_SIZE: GuestUSize = 1024 * 1024;

    /// Address of the lowest byte (not the base) of the main thread's stack.
    ///
    /// We are arbitrarily putting the stack at the top of the virtual address
    /// space (see also: stack.rs), I have no idea if this matches iPhone OS.
    pub const MAIN_THREAD_STACK_LOW_END: VAddr = 0u32.wrapping_sub(Self::MAIN_THREAD_STACK_SIZE);

    /// iPhone OS secondary thread stack size.
    pub const SECONDARY_THREAD_DEFAULT_STACK_SIZE: GuestUSize = 512 * 1024;

    /// Create a fresh instance of guest memory.
    pub fn new() -> Mem {
        // This will hopefully get the host OS to lazily allocate the memory.
        let layout = std::alloc::Layout::new::<Bytes>();
        let bytes = unsafe { std::alloc::alloc_zeroed(layout) as *mut Bytes };

        let allocator = allocator::Allocator::new();

        Mem {
            bytes,
            null_segment_size: 0,
            allocator,
        }
    }

    /// Take an existing instance of [Mem], but free and zero all the
    /// allocations so it's "like new".
    ///
    /// Note that, since there is no protection against writing outside an
    /// allocation, there might be stray bytes preserved in the result.
    pub fn refurbish(mut mem: Mem) -> Mem {
        let Mem {
            bytes: _,
            null_segment_size: _,
            ref mut allocator,
        } = mem;
        let used_chunks = allocator.reset_and_drain_used_chunks();
        for allocator::Chunk { base, size } in used_chunks {
            mem.bytes_mut()[base as usize..][..size.get() as usize].fill(0);
        }
        mem.null_segment_size = 0;
        mem
    }

    /// Sets up the null segment of the given size. There's no reason to call
    /// this outside of binary loading, and it won't be respected even if you
    /// do. The size must not have been set already, and must be page aligned.
    pub fn set_null_segment_size(&mut self, new_null_segment_size: VAddr) {
        // TODO?: Maybe this should be replaced with a per-page rwx/callback
        //        setting? Currently we don't properly follow segment
        //        protections, which means that applications can write into
        //        segments they shouldn't be able to. Adding that would fix
        //        this, along with removing this special case.
        assert!(self.null_segment_size == 0);
        assert!(new_null_segment_size % 0x1000 == 0);
        self.allocator
            .reserve(allocator::Chunk::new(0, new_null_segment_size));
        self.null_segment_size = new_null_segment_size;
    }

    pub fn null_segment_size(&self) -> VAddr {
        self.null_segment_size
    }

    /// Get a pointer to the full 4GiB of memory. This is only for use when
    /// setting up the CPU, never call this otherwise.
    ///
    /// Safety: You must ensure that this pointer does not outlive the instance
    /// of [Mem]. You must not use it while a `&mut` is held on some region of
    /// guest memory.
    pub unsafe fn direct_memory_access_ptr(&mut self) -> *mut std::ffi::c_void {
        self.bytes.cast()
    }

    fn bytes(&self) -> &Bytes {
        unsafe { &*self.bytes }
    }
    fn bytes_mut(&mut self) -> &mut Bytes {
        unsafe { &mut *self.bytes }
    }

    // the performance characteristics of this hasn't been profiled, but it
    // seems like a good idea to help the compiler optimise for the fast path
    #[cold]
    fn null_check_fail(at: VAddr, size: GuestUSize) {
        panic!(
            "Attempted null-page access at {:#x} ({:#x} bytes)",
            at, size
        )
    }

    /// Special version of [Self::bytes_at] that returns [None] rather than
    /// panicking on failure. Only for use by [crate::gdb::GdbServer].
    pub fn get_bytes_fallible(&self, addr: ConstVoidPtr, count: GuestUSize) -> Option<&[u8]> {
        if addr.to_bits() < self.null_segment_size {
            return None;
        }
        self.bytes()
            .get(addr.to_bits() as usize..)?
            .get(..count as usize)
    }
    /// Special version of [Self::bytes_at_mut] that returns [None] rather than
    /// panicking on failure. Only for use by [crate::gdb::GdbServer].
    pub fn get_bytes_fallible_mut(
        &mut self,
        addr: ConstVoidPtr,
        count: GuestUSize,
    ) -> Option<&mut [u8]> {
        if addr.to_bits() < self.null_segment_size {
            return None;
        }
        self.bytes_mut()
            .get_mut(addr.to_bits() as usize..)?
            .get_mut(..count as usize)
    }

    /// Get a slice for reading `count` bytes. This is the basic primitive for
    /// safe read-only memory access.
    ///
    /// This will panic when `ptr` is within the null page, even if `count` is
    /// 0. This may be inconvenient in some cases, but it makes the behavior
    /// when deriving a pointer from the slice consistent (though you should use
    /// [Self::ptr_at] for that).
    pub fn bytes_at<const MUT: bool>(&self, ptr: Ptr<u8, MUT>, count: GuestUSize) -> &[u8] {
        if ptr.to_bits() < self.null_segment_size {
            Self::null_check_fail(ptr.to_bits(), count)
        }
        &self.bytes()[ptr.to_bits() as usize..][..count as usize]
    }
    /// Get a slice for reading `count` bytes without a null-page check.
    ///
    /// This **doesn't** panic at access within the null page.
    ///
    /// You shall have a good reason to use it instead of [Self::bytes_at]
    pub fn unchecked_bytes_at<const MUT: bool>(
        &self,
        ptr: Ptr<u8, MUT>,
        count: GuestUSize,
    ) -> &[u8] {
        &self.bytes()[ptr.to_bits() as usize..][..count as usize]
    }
    /// Get a slice for reading or writing `count` bytes. This is the basic
    /// primitive for safe read-write memory access.
    ///
    /// This will panic when `ptr` is within the null page, even if `count` is
    /// 0. This may be inconvenient in some cases, but it makes the behavior
    /// when deriving a pointer from the slice consistent (though you should use
    /// [Self::ptr_at_mut] for that).
    pub fn bytes_at_mut(&mut self, ptr: MutPtr<u8>, count: GuestUSize) -> &mut [u8] {
        if ptr.to_bits() < self.null_segment_size {
            Self::null_check_fail(ptr.to_bits(), count)
        }
        &mut self.bytes_mut()[ptr.to_bits() as usize..][..count as usize]
    }

    /// Get a pointer for reading an array of `count` elements of type `T`.
    /// Only use this for interfacing with unsafe C-like APIs.
    ///
    /// The `count` argument is purely for bounds-checking and does not affect
    /// the result.
    ///
    /// No guarantee is made about the alignment of the resulting pointer!
    /// Pointers that are well-aligned for the guest are not necessarily
    /// well-aligned for the host. Rust strictly requires pointers to be
    /// well-aligned when dereferencing them, or when constructing references or
    /// slices from them, so **be very careful**.
    pub fn ptr_at<T, const MUT: bool>(&self, ptr: Ptr<T, MUT>, count: GuestUSize) -> *const T
    where
        T: SafeRead,
    {
        let size = count.checked_mul(guest_size_of::<T>()).unwrap();
        self.bytes_at(ptr.cast(), size).as_ptr().cast()
    }
    /// A variation of [Self::ptr_at] without a null-page check.
    ///
    /// This **doesn't** panic at access within the null page.
    ///
    /// You shall have a good reason to use it instead of [Self::ptr_at]
    pub fn unchecked_ptr_at<T, const MUT: bool>(
        &self,
        ptr: Ptr<T, MUT>,
        count: GuestUSize,
    ) -> *const T
    where
        T: SafeRead,
    {
        let size = count.checked_mul(guest_size_of::<T>()).unwrap();
        self.unchecked_bytes_at(ptr.cast(), size).as_ptr().cast()
    }
    /// Get a pointer for reading or writing to an array of `count` elements of
    /// type `T`. Only use this for interfacing with unsafe C-like APIs.
    ///
    /// The `count` argument is purely for bounds-checking and does not affect
    /// the result.
    ///
    /// No guarantee is made about the alignment of the resulting pointer!
    /// Pointers that are well-aligned for the guest are not necessarily
    /// well-aligned for the host. Rust strictly requires pointers to be
    /// well-aligned when dereferencing them, or when constructing references or
    /// slices from them, so **be very careful**.
    pub fn ptr_at_mut<T>(&mut self, ptr: MutPtr<T>, count: GuestUSize) -> *mut T
    where
        T: SafeRead + SafeWrite,
    {
        let size = count.checked_mul(guest_size_of::<T>()).unwrap();
        self.bytes_at_mut(ptr.cast(), size).as_mut_ptr().cast()
    }

    /// Transform a host pointer addressing a location in guest memory back into
    /// a guest pointer. This exists solely to deal with OpenGL `glGetPointerv`.
    /// You should never have another reason to use this.
    ///
    /// Panics if the host pointer is not addressing a location in guest memory.
    pub fn host_ptr_to_guest_ptr(&self, host_ptr: *const std::ffi::c_void) -> ConstVoidPtr {
        let host_ptr = host_ptr.cast::<u8>();
        let guest_mem_range = self.bytes().as_ptr_range();
        assert!(guest_mem_range.contains(&host_ptr));
        let guest_addr = host_ptr as usize - guest_mem_range.start as usize;
        Ptr::from_bits(u32::try_from(guest_addr).unwrap())
    }

    /// Read a value for memory. This is the preferred way to read memory in
    /// most cases.
    pub fn read<T, const MUT: bool>(&self, ptr: Ptr<T, MUT>) -> T
    where
        T: SafeRead,
    {
        // This is unsafe unless we are careful with which types SafeRead is
        // implemented for!
        // This would also be unsafe if the non-unaligned method was used.
        unsafe { self.ptr_at(ptr, 1).read_unaligned() }
    }
    /// Write a value to memory. This is the preferred way to write memory in
    /// most cases.
    pub fn write<T>(&mut self, ptr: MutPtr<T>, value: T)
    where
        T: SafeWrite,
    {
        let size = guest_size_of::<T>();
        assert!(size > 0);
        let slice = self.bytes_at_mut(ptr.cast(), size);
        let ptr: *mut T = slice.as_mut_ptr().cast();
        // It's unaligned because what is well-aligned for the guest is not
        // necessarily well-aligned for the host.
        // This would be unsafe if the non-unaligned method was used.
        unsafe { ptr.write_unaligned(value) }
    }

    /// C-style `memmove`.
    pub fn memmove(&mut self, dest: MutVoidPtr, src: ConstVoidPtr, size: GuestUSize) {
        let src = src.to_bits() as usize;
        let dest = dest.to_bits() as usize;
        let size = size as usize;
        self.bytes_mut()
            .copy_within(src..src.checked_add(size).unwrap(), dest)
    }

    /// Allocate `size` bytes.
    pub fn alloc(&mut self, size: GuestUSize) -> MutVoidPtr {
        let ptr = Ptr::from_bits(self.allocator.alloc(size));
        log_dbg!("Allocated {:?} ({:#x} bytes)", ptr, size);
        ptr
    }

    pub fn realloc(&mut self, old_ptr: MutVoidPtr, size: GuestUSize) -> MutVoidPtr {
        if old_ptr.is_null() {
            return self.alloc(size);
        }
        // TODO: for a moment we always assume that we do not have enough size
        //       to realloc inplace
        let old_size = self.allocator.find_allocated_size(old_ptr.to_bits());
        if old_size >= size {
            return old_ptr;
        }
        let new_ptr = self.alloc(size);
        self.memmove(new_ptr, old_ptr.cast_const(), old_size);
        self.free(old_ptr);
        new_ptr
    }

    /// Free an allocation made with one of the `alloc` methods on this type.
    pub fn free(&mut self, ptr: MutVoidPtr) {
        let size = self.allocator.free(ptr.to_bits());
        self.bytes_at_mut(ptr.cast(), size).fill(0);
        log_dbg!("Freed {:?} ({:#x} bytes)", ptr, size);
    }

    /// Allocate memory large enough for a value of type `T` and write the value
    /// to it. Equivalent to [Self::alloc] + [Self::write].
    pub fn alloc_and_write<T>(&mut self, value: T) -> MutPtr<T>
    where
        T: SafeWrite,
    {
        let ptr = self.alloc(guest_size_of::<T>()).cast();
        self.write(ptr, value);
        ptr
    }

    /// Allocate and write a C string. This method will add a null terminator,
    /// so it is optimal if the input slice does not already contain one.
    pub fn alloc_and_write_cstr(&mut self, str_bytes: &[u8]) -> MutPtr<u8> {
        let len = str_bytes.len().try_into().unwrap();
        let ptr = self.alloc(len + 1).cast();
        self.bytes_at_mut(ptr, len).copy_from_slice(str_bytes);
        self.write(ptr + len, b'\0');
        ptr
    }

    /// Get a C string (null-terminated) as a slice. The null terminator is not
    /// included in the slice.
    pub fn cstr_at<const MUT: bool>(&self, ptr: Ptr<u8, MUT>) -> &[u8] {
        let mut len = 0;
        while self.read(ptr + len) != b'\0' {
            len += 1;
        }
        self.bytes_at(ptr, len)
    }

    /// Get a C string (null-terminated) as a string slice, if it is valid
    /// UTF-8, otherwise returning a byte slice. The null terminator is not
    /// included in the slice.
    pub fn cstr_at_utf8<const MUT: bool>(&self, ptr: Ptr<u8, MUT>) -> Result<&str, &[u8]> {
        let bytes = self.cstr_at(ptr);
        std::str::from_utf8(bytes).map_err(|_| bytes)
    }

    pub fn wcstr_at<const MUT: bool>(&self, ptr: Ptr<wchar_t, MUT>) -> String {
        let mut len = 0;
        while self.read(ptr + len) != wchar_t::default() {
            len += 1;
        }
        let iter = self
            .bytes_at(ptr.cast(), len * guest_size_of::<wchar_t>())
            .chunks(4)
            .map(|chunk| char::from_u32(u32::from_le_bytes(chunk.try_into().unwrap())).unwrap());
        String::from_iter(iter)
    }

    /// Permanently mark a region of address space as being unusable to the
    /// memory allocator.
    pub fn reserve(&mut self, base: VAddr, size: GuestUSize) {
        self.allocator.reserve(allocator::Chunk::new(base, size));
    }
}
