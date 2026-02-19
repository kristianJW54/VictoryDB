// Arena allocator which manages the arenas that each memtable uses
//
// What do we need:
// Arena invariants:
//
// - An arena has a fixed capacity.
// - An arena has exactly one active owner at a time (a memtable).
// - While active:
// - allocations are bump-only
// - memory is never reclaimed
// - An arena is reset only when it has no active owner.
// - Reset invalidates all pointers previously allocated from it.

use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

// Arean Allocator must only allocate one arena at a time and give ownership of that memory to an arena

//
pub(crate) trait ChunkAllocator: Send + Sync {
    unsafe fn allocate(&self, size: usize) -> Box<[u8]>;
}

// Default Allocator for allocating chunks to arena
pub(crate) struct SystemAllocator {}

impl SystemAllocator {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) unsafe fn allocate(&self, size: usize) -> Box<[u8]> {
        let heap = Box::<[u8]>::new_uninit_slice(size);
        unsafe { heap.assume_init() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate() {
        let alloc = SystemAllocator::new();
        let chunk = unsafe { alloc.allocate(10) };

        println!("chunk size {:?}", chunk.len());
    }
}
