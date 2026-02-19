// Memtable Manager sits between the allocator and memtables and is responsible for giving semantic meaning to the memory arenas allocated by the allocator
// It creates and manages memtable states and rotations
//

use crate::core::memtable::memtable::{Immutable, Memtable, Mutable};
use std::ptr::NonNull;

// Memtable List Version is a snapshot of the memtable registry at a given point in time
// We centralise the memtable registry access for a particular point in time to give to a database snapshot which will allow readers to not block on a mutable mem_list which
// is changing as memtables are rotated
pub(crate) struct MemListVersion {
    active_memtable: NonNull<Mutable>,
    imm_list: Vec<NonNull<Memtable<Immutable>>>,
}

// Moving to a more registry based approach
pub(crate) struct MemTableList {
    active_memtable: Memtable<Mutable>,
    immutable_memtables: Vec<Memtable<Immutable>>,
}
