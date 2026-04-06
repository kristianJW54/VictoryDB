//
//
//
//
//
use std::ptr::NonNull;
use std::sync::Arc;

use crate::column_family::cf::ColumnFamilyData;
use crate::memtable::memtable::{Immutable, Memtable, Mutable};

pub(crate) struct Superversion {
    cf: NonNull<ColumnFamilyData>, // Circular reference to parent
    mem: Arc<Memtable<Mutable>>,
    imm: Arc<Memtable<Immutable>>,
    // NOTE: Version
}
