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
use std::ptr::NonNull;
use std::ptr::*;
use std::sync::{
    Mutex,
    atomic::{AtomicPtr, AtomicUsize, Ordering},
};

use crate::storage::memory::allocator::Allocator;
use crate::storage::memory::{ArenaPolicy, ArenaSize};

#[derive(Debug)]
pub(crate) enum ArenaError {
    AllocationError(usize),
    Overflow,
    ChunkDoubleCheckFailed,
    ArenaFull,
}

pub(super) type ChunkPtr = AtomicPtr<u8>;

/// Arena is responsible for holding blocks of memory and managing memory allocation into those blocks. It will handle alignment and block allocation.
/// Only Memtables will hold an arena.
///
/// Arena makes no attempt ensure pointers are not leaked or that memory being written is correclty aligned.
/// It is the responsibility of the caller to maintain that the data written is the same as the layout provided which arena used to reserve memory for.
///
/// For this reason, no specific Drop implementation is needed. Instead, we rely on memtables to implement Drop to know when an arena can be deallocated.
pub(crate) struct Arena {
    current_chunk: ChunkPtr,
    end: ChunkPtr,
    chunks: Mutex<Vec<Box<[u8]>>>,
    bump: AtomicUsize,
    allocated_bytes: AtomicUsize,
    memory_used: AtomicUsize,
    // TODO: May want total padding bytes? for later optimization
    allocator: Allocator,
    policy: ArenaPolicy,
}

impl Arena {
    pub(crate) fn new(policy: ArenaSize, allocator: Allocator) -> Self {
        let policy = policy.to_policy();

        let mut heap = unsafe { allocator.allocate(policy.block_size) };
        let chunk_ptr = heap.as_mut_ptr();
        let end = unsafe { chunk_ptr.add(policy.block_size) };

        let block_cap = policy.cap / policy.block_size;
        let mut chunks = Vec::with_capacity(block_cap);
        chunks.push(heap);

        Self {
            current_chunk: AtomicPtr::new(chunk_ptr),
            end: AtomicPtr::new(end),
            chunks: Mutex::new(chunks),
            bump: AtomicUsize::new(0),
            allocated_bytes: AtomicUsize::new(policy.block_size),
            memory_used: AtomicUsize::new(0),
            allocator,
            policy,
        }
    }

    fn alignment_check(&self, bump: usize, layout: Layout) -> Result<(usize, usize), ArenaError> {
        // Get the next required offset based on the layout alignment
        debug_assert!(layout.align().is_power_of_two());
        let aligned = (bump + (layout.align() - 1)) & !(layout.align() - 1);

        let next = aligned
            .checked_add(layout.size())
            .ok_or(ArenaError::Overflow)?;

        if next > self.policy.block_size {
            return Err(ArenaError::Overflow);
        }

        Ok((aligned, next))
    }

    // https://algomaster.io/learn/concurrency-interview/compare-and-swap
    // Shows a simple CAS loop where we get value Relaxed - compute the new value we want and try to CAS - if we fail we try again.
    //

    // NOTE: I've made the closure unsafe and it is up to the caller to ensure that the Layout and write to the pointer are correct.
    pub(crate) unsafe fn alloc_raw(&self, layout: Layout) -> Result<NonNull<u8>, ArenaError> {
        //

        loop {
            // We get relaxed bump here because we will double check if CAS if it fails we try to get bump again in the loop
            let bump = self.bump.load(Ordering::Relaxed);

            match self.alignment_check(bump, layout) {
                Err(_) => {
                    // If we fail alignment check we try_new_chunk
                    match self.try_new_chunk(layout) {
                        // Return out
                        Err(e) => return Err(e),
                        Ok(_) => continue,
                    };
                }
                Ok((aligned, next)) => {
                    // If CAS works we can write to the arena heap
                    if self
                        .bump
                        .compare_exchange_weak(bump, next, Ordering::AcqRel, Ordering::Relaxed)
                        .is_ok()
                    {
                        // If we are ok then we can write to the arena heap by passing the aligned pointer into closure
                        //

                        let current_ptr = self.current_chunk.load(Ordering::Acquire);

                        let ptr = unsafe { NonNull::new_unchecked(current_ptr.add(aligned)) };

                        // Update meta data
                        self.memory_used.fetch_add(layout.size(), Ordering::AcqRel);

                        return Ok(ptr);
                    }

                    // Another thread beat us - we try again
                    std::hint::spin_loop();
                }
            }
        }
    }

    fn try_new_chunk(&self, layout: Layout) -> Result<(), ArenaError> {
        // We need to lock and then check we are still ok to mutate the vec and pointer
        let mut lock = self.chunks.lock().unwrap();

        let bump = self.bump.load(Ordering::Relaxed);

        // Now we double check we are still good to mutate by checking size and alignment
        if let Ok(_) = self.alignment_check(bump, layout) {
            return Ok(());
        }

        // We failed the size and alignment check meaning we need to allocate a new chunk

        // We need to check that by adding a new chunk we don't exceed the cap
        if self.allocated_bytes.load(Ordering::Relaxed) + self.policy.block_size > self.policy.cap {
            return Err(ArenaError::ArenaFull);
        }

        self.allocated_bytes
            .fetch_add(self.policy.block_size, Ordering::Relaxed);

        // Now we allocate a new chunk of memory from the allocator
        let mut chunk = unsafe { self.allocator.allocate(self.policy.block_size) };
        let chunk_ptr = chunk.as_mut_ptr();

        lock.push(chunk);

        // Update the bump pointer
        self.bump.store(0, Ordering::Relaxed);
        // And update end pointer
        self.end.store(
            unsafe { chunk_ptr.add(self.policy.block_size) },
            Ordering::Relaxed,
        );
        // Now we need to atomically update the current chunk pointer
        self.current_chunk.store(chunk_ptr, Ordering::Relaxed);

        Ok(())
    }

    #[inline(always)]
    fn blocks_used(&self) -> usize {
        let used = self.allocated_bytes.load(Ordering::Relaxed);
        used / self.policy.block_size
    }

    #[inline(always)]
    fn max_bytes(&self) -> usize {
        self.policy.cap
    }

    #[inline(always)]
    fn number_of_blocks(&self) -> usize {
        self.policy.cap / self.policy.block_size
    }

    #[inline(always)]
    pub(crate) fn memory_used(&self) -> usize {
        let used = self.memory_used.load(Ordering::Relaxed);
        used
    }

    #[inline]
    pub(crate) fn get_current_init_slice(&self) -> &[u8] {
        let current = self.current_chunk.load(Ordering::Relaxed);

        let bump = self.bump.load(Ordering::Relaxed);

        unsafe { &*slice_from_raw_parts(current, bump) }
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::memory::allocator::{Allocator, SystemAllocator};

    use super::*;
    use std::thread::{self};

    #[test]
    fn competing_allocs() {
        let arena = Arena::new(
            ArenaSize::Default,
            Allocator::System(SystemAllocator::new()),
        );

        thread::scope(|s| {
            // Don't need arc because scope guarantees arena is dropped when scope ends
            for _ in 0..10 {
                s.spawn(|| {
                    for _ in 0..1000 {
                        unsafe {
                            let _ = arena.alloc_raw(Layout::new::<u32>());
                        };
                    }
                });
            }
        });

        println!("arena bump {:?}", arena.bump.load(Ordering::Relaxed));
    }

    #[test]
    fn arena_sizing() {
        let arena = Arena::new(
            ArenaSize::Test(10, 20),
            Allocator::System(SystemAllocator::new()),
        );

        println!("arena {:?}", arena.chunks.lock().unwrap()[0]);

        // Want to print up until the bump
        let current = arena.current_chunk.load(Ordering::Relaxed).addr();
        let bump = arena.bump.load(Ordering::Relaxed);

        let diff = bump - current;

        println!("chunk {:?}", unsafe {
            &*slice_from_raw_parts(current as *const u8, diff)
        });

        println!("arena max size {:?}", arena.max_bytes());
        println!("arena max blocks {:?}", arena.number_of_blocks());
    }

    #[test]
    fn alignment_bitwise() {
        let arena = Arena::new(
            ArenaSize::Test(10, 20),
            Allocator::System(SystemAllocator::new()),
        );

        // First lets alloc a char (1-byte)
        let layout = Layout::new::<u8>();
        unsafe {
            let ptr = arena.alloc_raw(layout).unwrap();
            ptr.write(2u8);
        }

        let layout = Layout::new::<u32>();

        unsafe {
            arena.alloc_raw(layout);
        }

        // Should get overflow error
        let l3 = Layout::new::<u64>();
        unsafe {
            let _ = arena.alloc_raw(l3).expect("errored");
        }
    }

    #[test]
    fn chunk_change() {
        let arena = Arena::new(
            ArenaSize::Test(10, 20),
            Allocator::System(SystemAllocator::new()),
        );

        // We nede to alloacate a u32 - then allocate a u16 - allocating another u32 should trigger a chunk allocation

        let layout_u32 = Layout::new::<u32>();
        unsafe {
            let ptr = arena.alloc_raw(layout_u32).unwrap();
            ptr.write(42);
        }
        let layout_u16 = Layout::new::<u16>();
        unsafe {
            let ptr = arena.alloc_raw(layout_u16).unwrap();
            ptr.write(12);
        }
        let layout_u32_2 = Layout::new::<u32>();
        unsafe {
            let ptr = arena.alloc_raw(layout_u32_2).unwrap();
            ptr.write(67)
        }

        println!(
            "arena first vec chunk  {:?}",
            arena.chunks.lock().unwrap()[0]
        );

        println!(
            "arena second vec chunk {:?}",
            arena.chunks.lock().unwrap()[1]
        );

        let slice = unsafe {
            std::slice::from_raw_parts(
                arena.current_chunk.load(Ordering::Relaxed),
                arena.policy.block_size,
            )
        };
        println!("from current pointer   {:?}", slice);
        println!("memory used {:?}", arena.memory_used());
    }

    #[test]
    fn tower_and_bytes() {
        let arena = Arena::new(
            ArenaSize::Test(20, 20),
            Allocator::System(SystemAllocator::new()),
        );

        #[repr(C)]
        struct Node {
            refs_and_height: AtomicUsize,
            key_len: u32, // We lose nothing by making it a u32
            value_len: u32,
            tower: [AtomicPtr<Node>; 0],
        }

        unsafe {
            let ptr = arena.alloc_raw(Layout::new::<Node>()).unwrap();
            ptr.cast::<Node>().write(Node {
                key_len: 1,
                value_len: 2,
                refs_and_height: AtomicUsize::new(3),
                tower: [],
            });
        }

        println!("size of node {:?}", std::mem::size_of::<Node>());

        // Now we try to add a byte or element in the tower array
        unsafe {
            let _ = arena
                .alloc_raw(Layout::new::<u8>())
                .expect("Error allocating byte");
        }

        unsafe {
            let _ = arena
                .alloc_raw(Layout::new::<u8>())
                .expect("Error allocating byte");
        }

        println!("current chunk {:?}", arena.get_current_init_slice());
        println!("memory used {:?}", arena.memory_used());
    }
}
