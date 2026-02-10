// Memtable Manager sits between the allocator and memtables and is responsible for giving semantic meaning to the memory arenas allocated by the allocator
// It creates and manages memtable states and rotations
//

use super::memtable::Memtable;
use std::sync::atomic::{AtomicPtr, AtomicU8};

const MAX_MEMTABLES: u8 = 4;
const MAX_IMMUTABLE_MEMTABLES: u8 = 3;

pub(crate) struct MemTableManager {
    active_memtable: AtomicPtr<Memtable>,
    memtables: Vec<Memtable>,
    immutable_memtables: [AtomicPtr<Memtable>; MAX_IMMUTABLE_MEMTABLES as usize],
    spare_memtable: AtomicPtr<Memtable>,
    // TODO: Need flush thread logic here
}

// TODO: Will have a flush thread which will take a memtable waiting to be flushed - try to flush it and then call the memtable manager to try to reset
// If we can't reset it, it's fine, we have marked flushed so no more readers and we wait for drain and can enforce blocking policy if we stall
// On last reader the memtable checks if it's is flushed and will call try_reset
