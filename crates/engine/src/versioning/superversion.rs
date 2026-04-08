use std::num::Wrapping;
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

pub(crate) struct SVCache {
    generation: u64,
    sv: *const Superversion,
}

impl SVCache {
    pub(crate) fn new() -> Self {
        Self {
            generation: 0,
            sv: std::ptr::null(),
        }
    }

    // Methods operating or deferencing the ptr MUST use a pin()
}
