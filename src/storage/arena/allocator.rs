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

use super::ArenaSize;
use super::arena::Arena;

use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

// Arean Allocator must only allocate one arena at a time and give ownership of that memory to an arena

// NOTE: We keep the trait interface intentionally simple here - the idea is that if we need to treat different arena sizes differently and if they impact algorithms or the system in different ways
// then we can implement each ArenaSize enum variant separately to give us Arena<Small>, Arena<Medium>, and Arena<Large> for example.
//
pub(crate) trait ArenaAllocator {
    fn allocate_arena(&self, arena_size: ArenaSize) -> Arena;
    fn deallocate_arena(&self, arena: Arena);
}

// We make a GlobalArenaAllocator which acts as a factory for arenas.
// This will sit in a DB engine structure (in RocksDB this is the DBImpl)
pub(crate) struct GlobalArenaAllocator {
    arenas_allocated: AtomicU8,
    total_memory_allocated: AtomicUsize,
}

impl ArenaAllocator for GlobalArenaAllocator {
    fn allocate_arena(&self, arena_size: ArenaSize) -> Arena {
        self.arenas_allocated.fetch_add(1, Ordering::Relaxed);
        todo!("todo")
    }

    fn deallocate_arena(&self, arena: Arena) {
        self.arenas_allocated.fetch_sub(1, Ordering::Relaxed);
        todo!("todo")
    }
}
