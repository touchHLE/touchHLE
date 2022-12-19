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

mod allocator;

/// Equivalent of `usize` for guest memory.
pub type GuestUSize = u32;

/// Internal type for representing an untyped virtual address.
type VAddr = GuestUSize;

/// Pointer type for guest memory, or the "guest pointer" type.
///
/// The `MUT` type parameter determines whether this is mutable or not.
/// Don't write it out explicitly, use [ConstPtr] or [MutPtr] instead.
///
/// The implemented methods try to mirror the Rust [pointer] type's methods,
/// where possible.
#[repr(transparent)]
pub struct Ptr<T, const MUT: bool>(VAddr, std::marker::PhantomData<T>);

// #[derive(Clone, Copy)] doesn't work for this type because T isn't always Copy
impl<T, const MUT: bool> Clone for Ptr<T, MUT> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, const MUT: bool> Copy for Ptr<T, MUT> {}

/// Constant guest pointer type (like Rust's `*const T`).
pub type ConstPtr<T> = Ptr<T, false>;
/// Mutable guest pointer type (like Rust's `*mut T`).
pub type MutPtr<T> = Ptr<T, true>;

impl<T, const MUT: bool> Ptr<T, MUT> {
    pub fn to_bits(self) -> VAddr {
        self.0
    }
    pub fn from_bits(bits: VAddr) -> Self {
        Ptr(bits, std::marker::PhantomData)
    }

    pub fn cast<U>(self) -> Ptr<U, MUT> {
        Ptr::<U, MUT>::from_bits(self.to_bits())
    }
}

// C-like pointer arithmetic
impl<T, const MUT: bool> std::ops::Add<GuestUSize> for Ptr<T, MUT> {
    type Output = Self;

    fn add(self, other: GuestUSize) -> Self {
        let size: GuestUSize = std::mem::size_of::<T>().try_into().unwrap();
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

/// Marker trait for types that can be safely read from guest memory.
///
/// Reading from guest memory is essentially doing a [std::mem::transmute],
/// which is notoriously unsafe in Rust. Only types for which all possible bit
/// patterns are legal (e.g. integers) should have this trait.
///
/// There is no marker trait for types that can be safely written because
/// transmuting something _into_ a series of bytes is always safe.
pub trait SafeRead {}
impl SafeRead for i8 {}
impl SafeRead for u8 {}
impl SafeRead for i16 {}
impl SafeRead for u16 {}
impl SafeRead for i32 {}
impl SafeRead for u32 {}
impl SafeRead for i64 {}
impl SafeRead for u64 {}
impl SafeRead for f32 {}
impl SafeRead for f64 {}
impl<T, const MUT: bool> SafeRead for Ptr<T, MUT> {}

type Bytes = [u8; 1 << 32];

/// The type that owns the guest memory and provides accessors for it.
pub struct Memory {
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
    bytes: *mut Bytes,

    allocator: allocator::Allocator,
}

impl Drop for Memory {
    fn drop(&mut self) {
        let layout = std::alloc::Layout::new::<Bytes>();
        unsafe {
            std::alloc::dealloc(self.bytes as *mut _, layout);
        }
    }
}

impl Memory {
    /// The first 4KiB of address space on iPhone OS is unused, so null pointer
    /// accesses can be trapped.
    ///
    /// We don't have full memory protection, but we can check accesses in that
    /// range.
    pub const NULL_PAGE_SIZE: VAddr = 0x1000;

    /// [According to Apple](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/Multithreading/CreatingThreads/CreatingThreads.html)
    /// among others, the iPhone OS main thread stack size is 1MiB.
    pub const MAIN_THREAD_STACK_SIZE: GuestUSize = 1024 * 1024;

    /// Address of the lowest byte (not the base) of the main thread's stack.
    ///
    /// We are arbitrarily putting the stack at the top of the virtual address
    /// space (see also: stack.rs), I have no idea if this matches iPhone OS.
    pub const MAIN_THREAD_STACK_LOW_END: VAddr = 0u32.wrapping_sub(Self::MAIN_THREAD_STACK_SIZE);

    pub fn new() -> Memory {
        // This will hopefully get the host OS to lazily allocate the memory.
        let layout = std::alloc::Layout::new::<Bytes>();
        let bytes = unsafe { std::alloc::alloc_zeroed(layout) as *mut Bytes };

        let allocator = allocator::Allocator::new();

        Memory { bytes, allocator }
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

    pub fn bytes_at<const MUT: bool>(&self, ptr: Ptr<u8, MUT>, count: GuestUSize) -> &[u8] {
        if ptr.to_bits() < Self::NULL_PAGE_SIZE {
            Self::null_check_fail(ptr.to_bits(), count)
        }
        &self.bytes()[ptr.to_bits() as usize..][..count as usize]
    }
    pub fn bytes_at_mut(&mut self, ptr: MutPtr<u8>, count: GuestUSize) -> &mut [u8] {
        if ptr.to_bits() < Self::NULL_PAGE_SIZE {
            Self::null_check_fail(ptr.to_bits(), count)
        }
        &mut self.bytes_mut()[ptr.to_bits() as usize..][..count as usize]
    }

    pub fn read<T, const MUT: bool>(&self, ptr: Ptr<T, MUT>) -> T
    where
        T: SafeRead,
    {
        let size = std::mem::size_of::<T>().try_into().unwrap();
        let slice = self.bytes_at(ptr.cast(), size);
        let ptr: *const T = slice.as_ptr().cast();
        // This is only safe if we are careful with which types SafeRead is
        // implemented for!
        // It's unaligned because what is well-aligned for the guest is not
        // necessarily well-aligned for the host.
        unsafe { ptr.read_unaligned() }
    }
    pub fn write<T>(&mut self, ptr: MutPtr<T>, value: T) {
        let size = std::mem::size_of::<T>().try_into().unwrap();
        let slice = self.bytes_at_mut(ptr.cast(), size);
        let ptr: *mut T = slice.as_mut_ptr().cast();
        // It's unaligned because what is well-aligned for the guest is not
        // necessarily well-aligned for the host.
        unsafe { ptr.write_unaligned(value) }
    }

    /// Permanently mark a region of address space as being unusable to the
    /// memory allocator.
    pub fn reserve(&mut self, base: VAddr, size: GuestUSize) {
        self.allocator.reserve(allocator::Chunk::new(base, size));
    }
}
