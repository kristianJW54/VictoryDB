use crate::memtable::memtable::{Flushed, Immutable, Memtable, Mutable};
use std::{ptr::NonNull, sync::Arc};

//------------------
//
//
//
//
// MemtableList holds the immutable state and logic for Immutable Memtables
pub(crate) struct MemTableList {
    imm: Vec<Arc<Memtable<Immutable>>>,
    current_version: Arc<MemListVersion>,
    flushed: Vec<Arc<Memtable<Flushed>>>,
    // TOOD: Need to add Flushed List
}

// Memtable List Version is a snapshot of the memtable registry at a given point in time
// We centralise the memtable registry access for a particular point in time to give to a database snapshot which will allow readers to
// access memtables without blocking or seeing conflicting states
pub(crate) struct MemListVersion {
    imm_version_list: Vec<Arc<Memtable<Immutable>>>,
}
