// Memtable Manager sits between the allocator and memtables and is responsible for giving semantic meaning to the memory arenas allocated by the allocator
// It creates and manages memtable states and rotations
//

use crate::storage::memtable::memtable::Memtable;
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, AtomicU8};

const MAX_MEMTABLES: u8 = 4;
const MAX_IMMUTABLE_MEMTABLES: u8 = 3;

// For now we'll have snapshotting here until we can move to the db engine layer correct file
// Ref counting within ref counting
struct InMemView {
    active: *const Memtable,
    immutable: Vec<*const Memtable>,
}

// Moving to a more registry based approach
pub(crate) struct MemTableList {
    active_memtable: AtomicPtr<Memtable>,
    immutable_memtables: Vec<Memtable>,
    //
    //
    // TODO: Where do we handle flush memtables?
    // TODO: Where do we handle free_memtables?
}
