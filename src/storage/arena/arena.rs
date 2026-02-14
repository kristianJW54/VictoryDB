// Arena runs the management of it's memory allocation given to it by the allocator
//
//

// The Arena will be used like a simple bump allocator for it's memory region
//
// For reference I used:
// https://fitzgen.com/2019/11/01/always-bump-downwards.html
// https://www.williamfedele.com/blog/arena-allocators
//
// This describes a good rust approach to alignment rounding and a recommendation of bumping downward as an optimisation
// RocksDB also uses an arena allocator but allocates from either end - front being aligned and back being unaligned
// For now, we'll use a simple bump allocator that allocates from the front and carefully control alignment of the skip nodes and byte data - if we notice improvements to be made then we can change
// Arena implementation to use a more efficient allocator
//
// Because we'll be allocating T (such as skiplist Nodes) and bytes (already aligned) we need to makes sure that what we write to in the heap is aligned
//

use std::sync::{
    Mutex,
    atomic::{AtomicPtr, AtomicUsize},
};

use crate::storage::arena::{ArenaPolicy, ArenaSize, allocator::ChunkAllocator};

pub(super) type ChunkPtr = AtomicPtr<u8>;

pub(crate) struct Arena {
    current_chunk: ChunkPtr,
    chunks: Mutex<Vec<Box<[u8]>>>,
    bump: AtomicUsize,
    used: AtomicUsize,
    allocator: Box<dyn ChunkAllocator>,
    policy: ArenaPolicy,
}

impl Arena {
    pub(crate) fn new(policy: ArenaSize, allocator: Box<dyn ChunkAllocator>) -> Self {
        let policy = policy.to_policy();

        let mut chunk = unsafe { allocator.allocate(policy.block_size) };
        Self {
            current_chunk: AtomicPtr::new(chunk.as_mut_ptr()),
            chunks: Mutex::new(vec![chunk]),
            bump: AtomicUsize::new(0),
            used: AtomicUsize::new(0),
            allocator,
            policy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate() {
        struct FakeAlloc {}

        impl ChunkAllocator for FakeAlloc {
            unsafe fn allocate(&self, size: usize) -> Box<[u8]> {
                let _ = size;
                vec![0; 10].into_boxed_slice()
            }
        }

        impl FakeAlloc {
            fn boxed() -> Box<Self> {
                Box::new(Self {})
            }
        }

        let arena = Arena::new(ArenaSize::Default, FakeAlloc::boxed());

        println!("chunk {:?}", arena.chunks.lock().unwrap()[0]);
    }
}
