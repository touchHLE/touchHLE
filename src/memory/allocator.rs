use super::{GuestUSize, Memory, VAddr};
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
        let null_page = Chunk::new(0, Memory::NULL_PAGE_SIZE);
        let main_thread_stack = Chunk::new(
            Memory::MAIN_THREAD_STACK_LOW_END,
            Memory::MAIN_THREAD_STACK_SIZE,
        );
        let rest = Chunk::new(
            Memory::NULL_PAGE_SIZE,
            Memory::MAIN_THREAD_STACK_LOW_END - Memory::NULL_PAGE_SIZE,
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
}
