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

use std::alloc::Layout;
use std::sync::{
    Mutex,
    atomic::{AtomicPtr, AtomicUsize},
};

use crate::storage::arena::{ArenaPolicy, ArenaSize, allocator::ChunkAllocator};

#[derive(Debug)]
pub(crate) enum ArenaError {
    AllocationError(usize),
}

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

    // We will need to be concurrent for writers here
    // https://stackoverflow.com/questions/45681531/what-is-the-right-way-to-write-double-checked-locking-in-rust
    // Shows a double-checked locking pattern for concurrent writers based on:
    // https://preshing.com/20130930/double-checked-locking-is-fixed-in-cpp11/
    //
    // Also we need a CAS loop in order to bump the offset
    // https://algomaster.io/learn/concurrency-interview/compare-and-swap
    // Shows a simple CAS loop where we get value Relaxed - compute the new value we want and try to CAS - if we fail we try again.
    //
    pub(crate) fn alloc(&self, layout: Layout) -> Result<(), ArenaError> {
        //

        loop {
            // We get relaxed bump here because we will double check if CAS if it fails we try to get bump again in the loop
            let bump = self.bump.load(std::sync::atomic::Ordering::Relaxed);

            let new_bump = bump + 1;
            //TODO: Actually determine the offset after getting the alignment

            if self
                .bump
                .compare_exchange_weak(
                    bump,
                    new_bump,
                    std::sync::atomic::Ordering::AcqRel,
                    std::sync::atomic::Ordering::Relaxed,
                )
                .is_ok()
            {
                // If we are ok then we can write to the arena heap
                return Ok(());
            }

            // Another thread beat us - we try again
            std::hint::spin_loop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::{self};

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

    #[test]
    fn allocate() {
        let arena = Arena::new(ArenaSize::Default, FakeAlloc::boxed());

        println!("chunk {:?}", arena.chunks.lock().unwrap()[0]);
    }

    #[test]
    fn competing_allocs() {
        let arena = Arena::new(ArenaSize::Default, FakeAlloc::boxed());

        thread::scope(|s| {
            // Don't need arc because scope guarantees arena is dropped when scope ends
            for _ in 0..10 {
                s.spawn(|| {
                    for _ in 0..1000 {
                        arena.alloc(Layout::new::<u32>()).unwrap();
                    }
                });
            }
        });

        println!(
            "arena bump {:?}",
            arena.bump.load(std::sync::atomic::Ordering::Relaxed)
        );
    }
}
