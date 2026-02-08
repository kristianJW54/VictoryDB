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

trait Allocator {
    // TOOD: We want to declare the number of arenas based on the max memtables
    // BUT we may want to reserve a spare arena incase we don't switch arenas quick enough we don't have to block then
    type NumOfArenaa: u8; // All arena sizes will be uniform across the memtables so we only need to define one size (one size fits all here ;) )
    type ArenaSize: ArenaSize;

    // Functions
}

// TODO: Or should allocator be a struct which holds a memtable manager and arenas etc?
// TODO: Or should a memtable manager implement the allocator or take something which implements the allocator trait?
