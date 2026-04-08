//
//
//
//
use std::sync::{Arc, atomic::AtomicPtr};

use crate::{
    memtable::memtable::{Memtable, Mutable},
    versioning::{memtable_list::MemTableList, superversion::Superversion},
};

pub(crate) struct ColumnFamilyData {
    id: u64,
    name: String,
    //
    // Write Path
    mem: Arc<Memtable<Mutable>>,
    imm: Arc<MemTableList>,
    //
    // Read Path
    superversion: AtomicPtr<Superversion>,
    // --
    // NOTE: *Version
    // NOTE: ThreadLocal<Superversion>,
}

// NOTE: Client facing wrapper around ColumnFamilyData
pub(crate) struct ColumnFamilyHandle {
    inner: Arc<ColumnFamilyData>,
}
