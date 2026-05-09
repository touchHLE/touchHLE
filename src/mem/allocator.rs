/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use super::{GuestUSize, VAddr, PAGE_SIZE};
use std::collections::BTreeMap;
use std::num::NonZeroU32;

/// iPhone OS's allocator always aligns to 16 bytes at minimum, and this
/// is also the minimum allocation size.
pub const MIN_CHUNK_SIZE: GuestUSize = 16;

/// A non-empty range of bytes in virtual address space.
///
/// Similar to [`RangeInclusive<u32>`][std::ops::RangeInclusive] but with a
/// more convenient representation.
#[derive(PartialEq, Eq, Copy, Clone)]
pub struct Chunk {
    pub(super) base: VAddr,
    pub(super) size: NonZeroU32,
}

impl Chunk {
    pub fn new(base: VAddr, size: GuestUSize) -> Chunk {
        Chunk {
            base,
            size: NonZeroU32::new(size).unwrap(),
        }
    }

    fn merge(self, other: Chunk) -> Chunk {
        assert!(
            self.last_byte() + 1 == other.base || other.last_byte() + 1 == self.base,
            "Chunks must be adjacent to merge"
        );
        Chunk::new(
            self.base.min(other.base),
            self.size.get() + other.size.get(),
        )
    }

    #[inline(always)]
    fn last_byte(&self) -> VAddr {
        self.base + (self.size.get() - 1)
    }

    #[inline(always)]
    fn contains(&self, addr: VAddr) -> bool {
        self.base <= addr && addr <= self.last_byte()
    }

    #[inline(always)]
    fn trisect_by(&self, middle: Chunk) -> Option<(Option<Chunk>, Option<Chunk>)> {
        if !self.contains(middle.base) || !self.contains(middle.last_byte()) {
            return None;
        }

        Some(self.difference(middle))
    }

    #[inline(always)]
    /// Returns parts of `self` that don't overlap with other
    fn difference(&self, other: Chunk) -> (Option<Chunk>, Option<Chunk>) {
        if other.last_byte() < self.base {
            return (None, Some(*self));
        }

        if other.base > self.last_byte() {
            return (Some(*self), None);
        }

        let left = match other.base.checked_sub(self.base) {
            None | Some(0) => None,
            Some(size) => Some(Chunk::new(self.base, size)),
        };

        let right = match self.last_byte().checked_sub(other.last_byte()) {
            None | Some(0) => None,
            Some(size) => Some(Chunk::new(other.last_byte() + 1, size)),
        };

        (left, right)
    }
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Chunk ({:#x}–{:#x}; {:#x} bytes)",
            self.base,
            self.base + (self.size.get() - 1),
            self.size.get()
        )
    }
}

#[cfg(test)]
mod chunk_tests {
    use super::Chunk;
    #[test]
    fn contains() {
        assert!(Chunk::new(2, 4).contains(2));
        assert!(Chunk::new(2, 4).contains(5));
        assert!(!Chunk::new(2, 4).contains(6));
    }

    #[test]
    fn trisect() {
        assert_eq!(
            Chunk::new(2, 4).trisect_by(Chunk::new(3, 2)),
            Some((Some(Chunk::new(2, 1)), Some(Chunk::new(5, 1))))
        );
        assert_eq!(
            Chunk::new(2, 4).trisect_by(Chunk::new(2, 2)),
            Some((None, Some(Chunk::new(4, 2))))
        );
        assert_eq!(
            Chunk::new(2, 4).trisect_by(Chunk::new(4, 2)),
            Some((Some(Chunk::new(2, 2)), None))
        );
        assert_eq!(Chunk::new(2, 4).trisect_by(Chunk::new(1, 2)), None);
        assert_eq!(Chunk::new(2, 4).trisect_by(Chunk::new(5, 2)), None);
    }

    #[test]
    fn merge() {
        let a = Chunk::new(0, 10);
        let b = Chunk::new(10, 10);
        assert_eq!(a.merge(b), Chunk::new(0, 20));
        assert_eq!(b.merge(a), Chunk::new(0, 20));
    }

    #[test]
    #[should_panic]
    fn merge_non_adjacent() {
        let a = Chunk::new(0, 10);
        let b = Chunk::new(20, 10);
        a.merge(b);
    }

    #[test]
    fn difference() {
        let test_chunk = Chunk::new(10, 10);
        assert_eq!(
            test_chunk.difference(Chunk::new(0, 5)),
            (None, Some(Chunk::new(10, 10)))
        );

        assert_eq!(
            test_chunk.difference(Chunk::new(20, 10)),
            (Some(Chunk::new(10, 10)), None)
        );

        assert_eq!(test_chunk.difference(Chunk::new(10, 10)), (None, None));

        assert_eq!(test_chunk.difference(Chunk::new(9, 15)), (None, None));

        assert_eq!(
            test_chunk.difference(Chunk::new(5, 10)),
            (None, Some(Chunk::new(15, 5)))
        );

        assert_eq!(
            test_chunk.difference(Chunk::new(15, 10)),
            (Some(Chunk::new(10, 5)), None)
        );

        assert_eq!(
            test_chunk.difference(Chunk::new(13, 3)),
            (Some(Chunk::new(10, 3)), Some(Chunk::new(16, 4)))
        );
    }
}

/// Specialized collection types. They're kept in their own module so the
/// allocator can only access them via their public methods, so that there's
/// less places inconsistencies between the sub-collections could happen.
mod collections {
    use super::*;

    #[derive(Default, Debug)]
    pub struct ChunkMap {
        chunks: BTreeMap<VAddr, NonZeroU32>,
    }
    impl ChunkMap {
        #[inline(always)]
        pub fn insert(&mut self, Chunk { base, size }: Chunk) {
            assert!(self.chunks.insert(base, size).is_none());
        }
        #[inline(always)]
        pub fn remove_with_base(&mut self, base: VAddr) -> Option<Chunk> {
            self.chunks.remove(&base).map(|size| Chunk { base, size })
        }
        #[inline(always)]
        pub fn remove_with_end(&mut self, end: VAddr) -> Option<Chunk> {
            let (&base, &size) = self.chunks.range(..end).next_back()?;
            let chunk = Chunk { base, size };
            if chunk.last_byte() + 1 != end {
                return None;
            }
            Some(self.remove_with_base(chunk.base).unwrap())
        }
        #[inline(always)]
        pub fn get_size_with_base(&self, base: VAddr) -> Option<NonZeroU32> {
            self.chunks.get(&base).copied()
        }

        pub fn overlapping_chunks(&self, chunk: Chunk) -> impl Iterator<Item = Chunk> + '_ {
            let start = self
                .chunks
                .range(..=chunk.base)
                .next_back()
                .filter(|(&base, &size)| chunk.base < base + size.get())
                .map(|(&base, _)| base)
                .unwrap_or(chunk.base);

            self.chunks
                .range(start..=chunk.last_byte())
                .map(|(&base, size)| Chunk::new(base, size.get()))
        }

        /// Remove all chunks overlapping a provided Chunk. In the case
        /// of overlap the non overlapping portion is left
        pub fn remove_range(&mut self, remove: Chunk) {
            let bases: Vec<VAddr> = self
                .overlapping_chunks(remove)
                .map(|chunk| chunk.base)
                .collect();

            for base in bases {
                let chunk = self.remove_with_base(base).unwrap();
                let (left, right) = chunk.difference(remove);
                if let Some(left) = left {
                    self.insert(left);
                }

                if let Some(right) = right {
                    self.insert(right);
                }
            }
        }
    }

    #[derive(Debug)]
    pub struct SizeBucketedChunkMap {
        min_chunk_size: u32,
        chunks: ChunkMap,
        chunks_by_log2_size: Vec<Vec<Chunk>>,
    }
    impl SizeBucketedChunkMap {
        pub fn new(min_chunk_size: u32) -> Self {
            Self {
                min_chunk_size,
                chunks: Default::default(),
                chunks_by_log2_size: vec![
                    Vec::new();
                    (u32::MAX.ilog2() - min_chunk_size.ilog2()) as usize + 1
                ],
            }
        }

        /// Get log2 size bucket for chunk.
        fn bucket_for(&self, size: GuestUSize) -> usize {
            (size.ilog2() - self.min_chunk_size.ilog2()) as usize
        }

        pub fn insert(&mut self, chunk: Chunk) {
            assert!(chunk.size.get() >= self.min_chunk_size);
            self.chunks.insert(chunk);
            let bucket_size = self.bucket_for(chunk.size.get());
            self.chunks_by_log2_size[bucket_size].push(chunk);
        }

        #[inline(always)]
        fn remove_from_bucket(&mut self, chunk: Chunk) {
            let bucket_size = self.bucket_for(chunk.size.get());
            let bucket = &mut self.chunks_by_log2_size[bucket_size];
            // Search from the end (recent frees are usually at the end, so
            // following the generational hypothesis, that's a better place to
            // start)
            let idx = bucket
                .iter()
                .rposition(|chunk2| chunk.base == chunk2.base)
                .unwrap();
            assert_eq!(chunk, bucket.swap_remove(idx));
        }

        pub fn remove_with_base(&mut self, base: VAddr) -> Option<Chunk> {
            let chunk = self.chunks.remove_with_base(base)?;
            self.remove_from_bucket(chunk);
            Some(chunk)
        }

        pub fn remove_with_end(&mut self, end: VAddr) -> Option<Chunk> {
            let chunk = self.chunks.remove_with_end(end)?;
            self.remove_from_bucket(chunk);
            Some(chunk)
        }

        /// Remove all chunks overlapping a provided Chunk. In the case
        /// of overlap the non overlapping portion is carved out
        pub fn remove_range(&mut self, remove: Chunk) {
            let bases: Vec<VAddr> = self
                .chunks
                .overlapping_chunks(remove)
                .map(|chunk| chunk.base)
                .collect();

            for base in bases {
                let chunk = self.remove_with_base(base).unwrap();
                let (left, right) = chunk.difference(remove);
                if let Some(left) = left {
                    self.insert(left);
                }

                if let Some(right) = right {
                    self.insert(right);
                }
            }
        }

        fn allocate_in_bucket(&mut self, size: GuestUSize, bucket: usize) -> Option<Chunk> {
            let (idx, _) = {
                let mut best_chunk: Option<(usize, GuestUSize)> = None;

                // Search from end because we should prefer recently-freed
                // allocations that might be the right size.
                for (idx, chunk) in self.chunks_by_log2_size[bucket]
                    .iter_mut()
                    .enumerate()
                    .rev()
                {
                    if chunk.size.get() >= size
                        && (best_chunk.is_none() || best_chunk.unwrap().1 > chunk.size.get())
                    {
                        best_chunk = Some((idx, chunk.size.get()));
                        if chunk.size.get() == size {
                            break;
                        }
                    }
                }

                best_chunk
            }?;

            let existing = self.chunks_by_log2_size[bucket].swap_remove(idx);
            let existing2 = self.chunks.remove_with_base(existing.base);
            assert_eq!(Some(existing), existing2);

            if existing.size.get() == size {
                return Some(existing);
            }

            let alloc = Chunk::new(existing.base, size);
            let rump = Chunk::new(existing.base + size, existing.size.get() - size);
            self.insert(rump);

            Some(alloc)
        }

        pub fn allocate(&mut self, size: GuestUSize) -> Option<Chunk> {
            assert!(size >= self.min_chunk_size);

            // Look in the smallest bucket first. This is the only bucket where
            // an exact match can be found.

            let bucket = self.bucket_for(size);
            if let Some(alloc) = self.allocate_in_bucket(size, bucket) {
                return Some(alloc);
            }

            // Exact match has been ruled out, find the smallest chunk in the
            // next largest non-empty bucket.

            let bucket = self.chunks_by_log2_size[bucket + 1..]
                .iter()
                .position(|bucket| !bucket.is_empty())?
                + bucket
                + 1;
            self.allocate_in_bucket(size, bucket)
        }

        pub fn iter(&self) -> impl Iterator<Item = Chunk> + '_ {
            self.chunks_by_log2_size
                .iter()
                .flat_map(|chunks| chunks.iter())
                .copied()
        }
    }
}
use collections::{ChunkMap, SizeBucketedChunkMap};

/// Tracks which memory is in use and makes allocations from it.
#[derive(Debug)]
pub struct HeapAllocator {
    used_chunks: ChunkMap,
    unused_chunks: SizeBucketedChunkMap,
    // These are chunks that are managed by an external allocator
    external_chunks: ChunkMap,
}

impl HeapAllocator {
    pub fn new(base: VAddr, size: GuestUSize) -> HeapAllocator {
        let allocation_space = Chunk::new(base, size);

        let mut unused_chunks = SizeBucketedChunkMap::new(MIN_CHUNK_SIZE);
        unused_chunks.insert(allocation_space);

        HeapAllocator {
            used_chunks: Default::default(),
            unused_chunks,
            external_chunks: Default::default(),
        }
    }

    pub fn alloc(&mut self, size: GuestUSize) -> Option<VAddr> {
        let size = size.max(MIN_CHUNK_SIZE);
        let size = Self::align(size, MIN_CHUNK_SIZE);

        let alloc = self.unused_chunks.allocate(size)?;

        self.used_chunks.insert(alloc);

        Some(alloc.base)
    }

    fn align(size: GuestUSize, align: GuestUSize) -> GuestUSize {
        if !size.is_multiple_of(align) {
            size + align - (size % align)
        } else {
            size
        }
    }

    /// This is used for realloc
    pub fn find_allocated_size(&mut self, base: VAddr) -> GuestUSize {
        if let Some(size) = self.external_chunks.get_size_with_base(base) {
            return size.get();
        }
        let Some(size) = self.used_chunks.get_size_with_base(base) else {
            panic!("Can't find {base:#x}, unknown allocation!");
        };
        size.get()
    }

    /// Add a chunk that was allocated by an external allocator
    pub fn add_external_allocation(&mut self, chunk: Chunk) {
        self.external_chunks.insert(chunk);
    }

    /// Returns the size of the freed chunk so it can be zeroed if desired
    #[must_use]
    pub fn free(&mut self, base: VAddr) -> GuestUSize {
        if let Some(freed) = self.external_chunks.remove_with_base(base) {
            return freed.size.get();
        }

        let Some(freed) = self.used_chunks.remove_with_base(base) else {
            log!("Can't free {:#x}, unknown allocation!", base);
            return 0;
        };

        if let Some(adjacent) = self
            .unused_chunks
            .remove_with_base(freed.last_byte() + 1)
            .or_else(|| self.unused_chunks.remove_with_end(freed.base))
        {
            let combined = adjacent.merge(freed);
            self.unused_chunks.insert(combined);
        } else {
            self.unused_chunks.insert(freed);
        }

        freed.size.get()
    }
}

#[derive(Debug)]
pub enum VMAllocError {
    AddressUnavailable,
    NoSpace,
}

/// Virtual Memory Allocator which handles allocation with page granularity
#[derive(Debug)]
pub struct VMAllocator {
    used_chunks: ChunkMap,
    unused_chunks: SizeBucketedChunkMap,
}

impl VMAllocator {
    pub fn new(base: VAddr, size: GuestUSize) -> VMAllocator {
        let allocation_space = Chunk::new(base, size);

        let mut unused_chunks = SizeBucketedChunkMap::new(PAGE_SIZE);
        unused_chunks.insert(allocation_space);

        VMAllocator {
            used_chunks: Default::default(),
            unused_chunks,
        }
    }

    pub fn allocate(
        &mut self,
        address: Option<VAddr>,
        size: GuestUSize,
    ) -> Result<Chunk, VMAllocError> {
        let size = size.next_multiple_of(PAGE_SIZE);
        match address {
            Some(address) => {
                let address = address & !(PAGE_SIZE - 1);
                self.allocate_at(address, size)
            }
            None => self.allocate_any(size),
        }
    }

    pub fn deallocate(&mut self, address: VAddr, size: GuestUSize) {
        // From testing vm_deallocate you can deallocate anything as long as
        // the memory is not protected. Since we have no permissions we can
        // always succeed
        let size = size.next_multiple_of(PAGE_SIZE);
        let address = address & !(PAGE_SIZE - 1);
        let freed = Chunk::new(address, size);

        self.used_chunks.remove_range(freed);
        self.unused_chunks.remove_range(freed);

        let left_adjacent = self.unused_chunks.remove_with_base(freed.last_byte() + 1);
        let right_adjacent = self.unused_chunks.remove_with_end(freed.base);

        let mut combined = freed;

        if let Some(adjacent) = left_adjacent {
            combined = combined.merge(adjacent);
        }

        if let Some(adjacent) = right_adjacent {
            combined = combined.merge(adjacent);
        }

        self.unused_chunks.insert(combined);
    }

    fn allocate_at(&mut self, address: VAddr, size: GuestUSize) -> Result<Chunk, VMAllocError> {
        assert!(address.is_multiple_of(PAGE_SIZE));
        assert!(size.is_multiple_of(PAGE_SIZE) && size >= PAGE_SIZE);
        let chunk = Chunk::new(address, size);

        let to_trisect = self
            .unused_chunks
            .iter()
            .find(|unused_chunk| unused_chunk.contains(address))
            .ok_or(VMAllocError::AddressUnavailable)?;

        let (before, after) = to_trisect.trisect_by(chunk).ok_or(VMAllocError::NoSpace)?;
        self.unused_chunks.remove_with_base(to_trisect.base);
        if let Some(before) = before {
            self.unused_chunks.insert(before);
        }
        if let Some(after) = after {
            self.unused_chunks.insert(after);
        }
        self.used_chunks.insert(chunk);

        Ok(chunk)
    }

    fn allocate_any(&mut self, size: GuestUSize) -> Result<Chunk, VMAllocError> {
        assert!(size.is_multiple_of(PAGE_SIZE) && size >= PAGE_SIZE);

        let alloc = self
            .unused_chunks
            .allocate(size)
            .ok_or(VMAllocError::NoSpace)?;

        self.used_chunks.insert(alloc);

        Ok(alloc)
    }
}
