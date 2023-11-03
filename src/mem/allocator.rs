/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use super::{GuestUSize, Mem, VAddr};
use std::collections::BTreeMap;
use std::num::NonZeroU32;

/// iPhone OS's allocator always aligns to 16 bytes at minimum, and this
/// is also the minimum allocation size.
/// TODO: also do the 4096-byte alignment.
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

        let left = match middle.base - self.base {
            0 => None,
            size => Some(Chunk::new(self.base, size)),
        };
        let right = match self.last_byte() - middle.last_byte() {
            0 => None,
            size => Some(Chunk::new(middle.last_byte() + 1, size)),
        };
        Some((left, right))
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
    fn test() {
        assert!(Chunk::new(2, 4).contains(2));
        assert!(Chunk::new(2, 4).contains(5));
        assert!(!Chunk::new(2, 4).contains(6));

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
        pub fn drain(self) -> impl Iterator<Item = Chunk> {
            self.chunks
                .into_iter()
                .map(|(base, size)| Chunk { base, size })
        }
        #[inline(always)]
        pub fn get_size_with_base(&self, base: VAddr) -> Option<NonZeroU32> {
            self.chunks.get(&base).copied()
        }
    }

    #[derive(Default, Debug)]
    pub struct SizeBucketedChunkMap {
        chunks: ChunkMap,
        chunks_by_log2_size: [Vec<Chunk>; Self::bucket_for(u32::MAX) + 1],
    }
    impl SizeBucketedChunkMap {
        /// Get log2 size bucket for chunk.
        #[inline(always)]
        const fn bucket_for(size: GuestUSize) -> usize {
            (size.ilog2() - MIN_CHUNK_SIZE.ilog2()) as usize
        }

        pub fn insert(&mut self, chunk: Chunk) {
            assert!(chunk.size.get() >= MIN_CHUNK_SIZE);
            self.chunks.insert(chunk);
            self.chunks_by_log2_size[Self::bucket_for(chunk.size.get())].push(chunk);
        }

        #[inline(always)]
        fn remove_from_bucket(&mut self, chunk: Chunk) {
            let bucket = &mut self.chunks_by_log2_size[Self::bucket_for(chunk.size.get())];
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
            assert!(size >= MIN_CHUNK_SIZE);

            // Look in the smallest bucket first. This is the only bucket where
            // an exact match can be found.

            let bucket = Self::bucket_for(size);
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
pub struct Allocator {
    used_chunks: ChunkMap,
    unused_chunks: SizeBucketedChunkMap,
}

impl Allocator {
    pub fn new() -> Allocator {
        let main_thread_stack =
            Chunk::new(Mem::MAIN_THREAD_STACK_LOW_END, Mem::MAIN_THREAD_STACK_SIZE);
        let rest = Chunk::new(0, Mem::MAIN_THREAD_STACK_LOW_END);

        let mut used_chunks: ChunkMap = Default::default();
        used_chunks.insert(main_thread_stack);

        let mut unused_chunks: SizeBucketedChunkMap = Default::default();
        unused_chunks.insert(rest);

        Allocator {
            used_chunks,
            unused_chunks,
        }
    }

    pub fn reserve(&mut self, chunk: Chunk) {
        let mut to_trisect = None;
        for unused_chunk in self.unused_chunks.iter() {
            if unused_chunk.trisect_by(chunk).is_some() {
                to_trisect = Some(unused_chunk);
                break;
            }
        }

        let Some(to_trisect) = to_trisect else {
            panic!("Could not reserve chunk {:?}!", chunk);
        };

        let (before, after) = to_trisect.trisect_by(chunk).unwrap();
        self.unused_chunks.remove_with_base(to_trisect.base);
        if let Some(before) = before {
            self.unused_chunks.insert(before);
        }
        if let Some(after) = after {
            self.unused_chunks.insert(after);
        }
        self.used_chunks.insert(chunk);
    }

    pub fn alloc(&mut self, size: GuestUSize) -> VAddr {
        let size = size.max(MIN_CHUNK_SIZE);
        let size = if size % MIN_CHUNK_SIZE != 0 {
            size + MIN_CHUNK_SIZE - (size % MIN_CHUNK_SIZE)
        } else {
            size
        };

        let Some(alloc) = self.unused_chunks.allocate(size) else {
            panic!(
                "Could not find large enough chunk to allocate {:#x} bytes",
                size
            );
        };
        self.used_chunks.insert(alloc);

        alloc.base
    }

    /// This is used for realloc
    pub fn find_allocated_size(&mut self, base: VAddr) -> GuestUSize {
        let Some(size) = self.used_chunks.get_size_with_base(base) else {
            panic!("Can't find {:#x}, unknown allocation!", base);
        };
        size.get()
    }

    /// Returns the size of the freed chunk so it can be zeroed if desired
    #[must_use]
    pub fn free(&mut self, base: VAddr) -> GuestUSize {
        let Some(freed) = self.used_chunks.remove_with_base(base) else {
            log!("Can't free {:#x}, unknown allocation!", base);
            return 0;
        };

        if let Some(adjacent) = self
            .unused_chunks
            .remove_with_base(freed.last_byte() + 1)
            .or_else(|| self.unused_chunks.remove_with_end(freed.base))
        {
            let combined = Chunk::new(
                freed.base.min(adjacent.base),
                freed.size.get() + adjacent.size.get(),
            );
            self.unused_chunks.insert(combined);
        } else {
            self.unused_chunks.insert(freed);
        }

        freed.size.get()
    }

    pub(super) fn reset_and_drain_used_chunks(&mut self) -> impl Iterator<Item = Chunk> {
        let chunks = std::mem::take(&mut self.used_chunks);
        *self = Allocator::new();
        chunks.drain()
    }
}
