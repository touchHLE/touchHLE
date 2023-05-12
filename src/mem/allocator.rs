/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
use super::{GuestUSize, Mem, VAddr};
use std::num::NonZeroU32;

/// A non-empty range of bytes in virtual address space.
///
/// Similar to [`RangeInclusive<u32>`][std::ops::RangeInclusive] but with a
/// more convenient representation.
#[derive(PartialEq, Eq, Copy, Clone)]
pub struct Chunk {
    base: VAddr,
    size: NonZeroU32,
}

impl Chunk {
    pub fn new(base: VAddr, size: GuestUSize) -> Chunk {
        Chunk {
            base,
            size: NonZeroU32::new(size).unwrap(),
        }
    }

    fn last_byte(&self) -> VAddr {
        self.base + (self.size.get() - 1)
    }

    fn contains(&self, addr: VAddr) -> bool {
        self.base <= addr && addr <= self.last_byte()
    }

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

/// Tracks which memory is in use and (TODO:) makes allocations from it.
#[derive(Debug)]
pub struct Allocator {
    used_chunks: Vec<Chunk>,
    unused_chunks: Vec<Chunk>,
}

impl Allocator {
    pub fn new() -> Allocator {
        let null_page = Chunk::new(0, Mem::NULL_PAGE_SIZE);
        let main_thread_stack =
            Chunk::new(Mem::MAIN_THREAD_STACK_LOW_END, Mem::MAIN_THREAD_STACK_SIZE);
        let rest = Chunk::new(
            Mem::NULL_PAGE_SIZE,
            Mem::MAIN_THREAD_STACK_LOW_END - Mem::NULL_PAGE_SIZE,
        );

        Allocator {
            used_chunks: vec![null_page, main_thread_stack],
            unused_chunks: vec![rest],
        }
    }

    pub fn reserve(&mut self, chunk: Chunk) {
        for i in 0..self.unused_chunks.len() {
            if let Some((before, after)) = self.unused_chunks[i].trisect_by(chunk) {
                self.unused_chunks.remove(i);
                if let Some(before) = before {
                    self.unused_chunks.push(before);
                }
                if let Some(after) = after {
                    self.unused_chunks.push(after);
                }

                self.used_chunks.push(chunk);
                return;
            }
        }

        panic!("Could not reserve chunk {:?}!", chunk);
    }

    pub fn alloc(&mut self, size: GuestUSize) -> VAddr {
        // TODO: use a better allocation strategy, probably using buckets.

        // iPhone OS's allocator always aligns to 16 bytes at minimum, and this
        // is also the minimum allocation size.
        // TODO: also do the 4096-byte alignment.
        let size = size.max(16);
        let size = if size % 16 != 0 {
            size + 16 - (size % 16)
        } else {
            size
        };

        let existing_chunk = {
            let mut perfect_chunk: Option<usize> = None;
            let mut big_enough_chunk: Option<(usize, GuestUSize)> = None;

            // Search from end because we should prefer recently-freed
            // allocations that might be the right size.
            for (idx, chunk) in self.unused_chunks.iter().enumerate().rev() {
                if chunk.size.get() == size {
                    perfect_chunk = Some(idx);
                    break;
                }
                if chunk.size.get() > size
                    && (big_enough_chunk.is_none()
                        || big_enough_chunk.unwrap().1 > chunk.size.get())
                {
                    big_enough_chunk = Some((idx, chunk.size.get()));
                }
            }

            if let Some(idx) = perfect_chunk {
                self.unused_chunks.remove(idx)
            } else if let Some((idx, _)) = big_enough_chunk {
                self.unused_chunks.remove(idx)
            } else {
                panic!(
                    "Could not find large enough chunk to allocate {:#x} bytes",
                    size
                )
            }
        };

        if size < existing_chunk.size.get() {
            let alloc = Chunk::new(existing_chunk.base, size);
            let rump = Chunk::new(existing_chunk.base + size, existing_chunk.size.get() - size);

            let res = alloc.base;
            self.used_chunks.push(alloc);
            self.unused_chunks.push(rump);
            res
        } else {
            assert!(size == existing_chunk.size.get());

            let res = existing_chunk.base;
            self.used_chunks.push(existing_chunk);
            res
        }
    }

    /// This is used for realloc
    pub fn find_allocated_size(&mut self, base: VAddr) -> GuestUSize {
        let Some(idx) = self.used_chunks.iter().position(|chunk| chunk.base == base) else {
            panic!("Can't find {:#x}, unknown allocation!", base);
        };
        let chunk = self.used_chunks.get(idx).unwrap();
        chunk.size.get()
    }

    /// Returns the size of the freed chunk so it can be zeroed if desired
    #[must_use]
    pub fn free(&mut self, base: VAddr) -> GuestUSize {
        let Some(idx) = self.used_chunks.iter().position(|chunk| chunk.base == base) else {
            panic!("Can't free {:#x}, unknown allocation!", base);
        };
        let chunk = self.used_chunks.remove(idx);
        let size = chunk.size.get();

        if let Some(other_chunk_idx) = self.unused_chunks.iter().position(|other_chunk| {
            (other_chunk.base as u64) == (chunk.last_byte() as u64 + 1)
                || (chunk.base as u64) == (other_chunk.last_byte() as u64 + 1)
        }) {
            let other_chunk = self.unused_chunks.swap_remove(other_chunk_idx);
            let combined = Chunk::new(
                chunk.base.min(other_chunk.base),
                chunk.size.get() + other_chunk.size.get(),
            );
            self.unused_chunks.push(combined);
        } else {
            self.unused_chunks.push(chunk);
        }
        size
    }
}
