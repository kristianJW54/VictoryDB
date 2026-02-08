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

use super::arena::Arena;
use std::{collections::VecDeque, mem::MaybeUninit};

pub(crate) struct ArenaAllocator<const NUM_OF_ARENAS: usize, const ARENA_SIZE: usize> {
    backing_mem: Box<[u8]>,         // The backing memory for the arenas.
    arenas: [Arena; NUM_OF_ARENAS], // Arenas are initialized and must remain valid until the allocator is dropped.
    free_list: [usize; NUM_OF_ARENAS],
}

impl<const NUM_OF_ARENAS: usize, const ARENA_SIZE: usize>
    ArenaAllocator<NUM_OF_ARENAS, ARENA_SIZE>
{
    pub fn new() -> Self {
        // Need to create backing memory

        // We can take the performance hit of allocating the memory upfront from vec to box here as this is a one time cost.
        let mem = vec![0u8; ARENA_SIZE * NUM_OF_ARENAS].into_boxed_slice();

        let arenas = core::array::from_fn(|_| Arena::default());
        let free_list = [1; NUM_OF_ARENAS];
        Self {
            backing_mem: mem,
            arenas,
            free_list,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_allocator() {
        let alloc = ArenaAllocator::<3, 10>::new();
        println!("New allocator created");
        println!("arena 1 = {:?}", alloc.arenas[0]);
        // If i loop through the free list - i should have 0, 1, 2
        for i in 0..alloc.free_list.len() {
            println!("alloc index {:}", alloc.free_list[i]);
        }
    }
}
