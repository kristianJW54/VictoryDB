//
//
//
//
//
use std::ptr::NonNull;
use std::sync::Arc;

use crate::storage::column_family::cf::ColumnFamilyData;
use crate::storage::memtable::memtable::{Immutable, Memtable, Mutable};

pub(crate) struct Superversion {
    cf: NonNull<ColumnFamilyData>,
    mem: Arc<Memtable<Mutable>>,
    imm: Arc<Memtable<Immutable>>,
    // NOTE: Version
}
